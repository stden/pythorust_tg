//! Chat digest/summary generator
//!
//! Generates AI-powered summaries of chat discussions for stories/reports

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, CreateChatCompletionRequest,
    },
    Client as OpenAIClient,
};
use chrono::{DateTime, Duration, Utc};

const DIGEST_SYSTEM_PROMPT: &str = r#"Ты — эксперт по анализу чатов. Твоя задача — создать краткий дайджест обсуждений.

Формат вывода:
## 📊 Дайджест чата за [период]

### 🔥 Главные темы
- Тема 1: краткое описание
- Тема 2: краткое описание

### 💡 Ключевые идеи и инсайты
- Инсайт 1
- Инсайт 2

### 🔗 Полезные ссылки
- [описание](ссылка)

### 👥 Активные участники
- @username1 - вклад
- @username2 - вклад

### 📈 Статистика
- Всего сообщений: N
- Активных участников: N
- Самая обсуждаемая тема: X

Пиши кратко, по делу, с эмодзи. Максимум 500 слов."#;

/// Digest configuration
pub struct DigestConfig {
    /// Time period for digest (hours)
    pub hours: i64,
    /// Maximum messages to analyze
    pub max_messages: usize,
    /// OpenAI model to use
    pub model: String,
    /// Output format (markdown, text, html)
    pub format: DigestFormat,
}

impl Default for DigestConfig {
    fn default() -> Self {
        Self {
            hours: 24,
            max_messages: 500,
            model: "gpt-4o-mini".to_string(),
            format: DigestFormat::Markdown,
        }
    }
}

#[derive(Clone, Copy)]
pub enum DigestFormat {
    Markdown,
    Text,
    Html,
}

/// Message data for digest
struct MessageData {
    sender: String,
    text: String,
    timestamp: DateTime<Utc>,
    reactions: i32,
}

/// Generate chat digest
pub async fn run(chat_name: &str, config: DigestConfig) -> Result<String> {
    // Get OpenAI API key
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| Error::InvalidArgument("OPENAI_API_KEY not set".to_string()))?;

    let openai_config = OpenAIConfig::new().with_api_key(api_key);
    let openai_client = OpenAIClient::with_config(openai_config);

    // Acquire session lock
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    println!(
        "📊 Генерирую дайджест чата '{}' за {} часов...",
        chat_name, config.hours
    );

    // Find chat
    let chat = crate::chat::find_chat(&client, chat_name).await?;

    // Calculate time cutoff
    let cutoff = Utc::now() - Duration::hours(config.hours);

    // Collect messages
    let mut messages: Vec<MessageData> = Vec::new();
    let mut iter = client.iter_messages(&chat);

    while let Some(msg_result) = iter.next().await.transpose() {
        if messages.len() >= config.max_messages {
            break;
        }

        if let Ok(msg) = msg_result {
            let msg_time: DateTime<Utc> = msg.date();
            if msg_time < cutoff {
                break;
            }

            let text = msg.text().trim().to_string();
            if text.is_empty() {
                continue;
            }

            let sender = if let Some(sender) = msg.sender() {
                match sender {
                    grammers_client::types::Peer::User(u) => u
                        .username()
                        .map(|s| format!("@{}", s))
                        .unwrap_or_else(|| u.full_name()),

                    grammers_client::types::Peer::Channel(c) => c.title().to_string(),
                    grammers_client::types::Peer::Group(g) => {
                        g.title().unwrap_or("Group").to_string()
                    }
                }
            } else {
                "Unknown".to_string()
            };

            let reactions = crate::reactions::count_reactions(&msg);

            messages.push(MessageData {
                sender,
                text,
                timestamp: msg_time,
                reactions,
            });
        }
    }

    if messages.is_empty() {
        return Ok("📭 Нет сообщений за указанный период".to_string());
    }

    // Reverse to chronological order
    messages.reverse();

    // Prepare chat content for AI
    let chat_content = prepare_chat_content(&messages);

    // Generate digest with AI
    let digest =
        generate_digest(&openai_client, &config.model, &chat_content, config.hours).await?;

    // Add statistics
    let stats = format!(
        "\n\n---\n*Проанализировано {} сообщений от {} участников*",
        messages.len(),
        count_unique_senders(&messages)
    );

    Ok(format!("{}{}", digest, stats))
}

fn prepare_chat_content(messages: &[MessageData]) -> String {
    let mut content = String::new();

    for msg in messages {
        let reactions_str = if msg.reactions > 0 {
            format!(" [{}❤]", msg.reactions)
        } else {
            String::new()
        };

        content.push_str(&format!(
            "{} {}: {}{}\n",
            msg.timestamp.format("%H:%M"),
            msg.sender,
            msg.text.chars().take(500).collect::<String>(),
            reactions_str
        ));
    }

    content
}

fn count_unique_senders(messages: &[MessageData]) -> usize {
    let senders: std::collections::HashSet<_> = messages.iter().map(|m| &m.sender).collect();
    senders.len()
}

async fn generate_digest(
    client: &OpenAIClient<OpenAIConfig>,
    model: &str,
    chat_content: &str,
    hours: i64,
) -> Result<String> {
    let user_prompt = format!(
        "Проанализируй этот чат за последние {} часов и создай дайджест:\n\n{}",
        hours, chat_content
    );

    let request = CreateChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: async_openai::types::ChatCompletionRequestSystemMessageContent::Text(
                    DIGEST_SYSTEM_PROMPT.to_string(),
                ),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(
                    user_prompt,
                ),
                name: None,
            }),
        ],
        temperature: Some(0.7),
        max_tokens: Some(1500),
        ..Default::default()
    };

    let response = client
        .chat()
        .create(request)
        .await
        .map_err(|e| Error::OpenAiError(e.to_string()))?;

    let content = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Не удалось сгенерировать дайджест".to_string());

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_unique_senders() {
        let messages = vec![
            MessageData {
                sender: "@user1".to_string(),
                text: "Hello".to_string(),
                timestamp: Utc::now(),
                reactions: 0,
            },
            MessageData {
                sender: "@user2".to_string(),
                text: "Hi".to_string(),
                timestamp: Utc::now(),
                reactions: 0,
            },
            MessageData {
                sender: "@user1".to_string(),
                text: "Bye".to_string(),
                timestamp: Utc::now(),
                reactions: 0,
            },
        ];

        assert_eq!(count_unique_senders(&messages), 2);
    }

    #[test]
    fn test_prepare_chat_content() {
        let messages = vec![MessageData {
            sender: "@test".to_string(),
            text: "Test message".to_string(),
            timestamp: Utc::now(),
            reactions: 5,
        }];

        let content = prepare_chat_content(&messages);
        assert!(content.contains("@test"));
        assert!(content.contains("Test message"));
        assert!(content.contains("[5❤]"));
    }
}
