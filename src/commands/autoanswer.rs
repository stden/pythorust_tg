//! AI Auto-responder command
//!
//! Equivalent to Python's autoanswer.py

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
use tokio::signal;

const SYSTEM_INSTRUCTIONS: &str = r#"Ты - полезный ассистент, который отвечает на вопросы в Telegram-чате.
Старайся давать подробные, ясные и понятные ответы.
Отвечай нейтральным тоном, при необходимости давай примеры кода и избегай ненормативной лексики.
Если пользователь задаёт технический вопрос, постарайся дать максимально понятный и точный ответ.
Если пользователь не указал иное, отвечай на русском языке."#;

pub async fn run(model: &str) -> Result<()> {
    // Get OpenAI API key from environment
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        Error::InvalidArgument("OPENAI_API_KEY environment variable not set".to_string())
    })?;

    let openai_config = OpenAIConfig::new().with_api_key(api_key);
    let openai_client = OpenAIClient::with_config(openai_config);

    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    println!("Бот запущен. Ожидаю сообщения...");
    println!("Нажмите Ctrl+C для остановки.");

    // Note: grammers 0.8 removed next_update() - need to use handle.step() pattern
    // For now, this is a placeholder implementation using polling
    // Real implementation would use client.handle.step() with proper update handling

    // Simple polling implementation - poll recent messages periodically
    let mut last_seen_id: Option<i32> = None;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("\nОстанавливаю бота...");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                // Poll for new messages in all dialogs
                // This is a simplified approach - real implementation would use update streaming
                let mut dialogs = client.iter_dialogs();

                while let Some(dialog) = dialogs.next().await.transpose() {
                    if let Ok(dialog) = dialog {
                        let chat = &dialog.peer;
                        let mut messages = client.iter_messages(chat);

                        if let Some(Ok(msg)) = messages.next().await.transpose() {
                            // Check if this is a new message we haven't seen
                            let msg_id = msg.id();
                            if let Some(last_id) = last_seen_id {
                                if msg_id <= last_id {
                                    continue;
                                }
                            }

                            // Skip outgoing messages
                            if msg.outgoing() {
                                last_seen_id = Some(msg_id);
                                continue;
                            }

                            let user_message = msg.text().trim().to_string();
                            if user_message.is_empty() {
                                last_seen_id = Some(msg_id);
                                continue;
                            }

                            println!("Получено сообщение: {}", user_message);
                            last_seen_id = Some(msg_id);

                            // Generate AI response
                            match generate_response(&openai_client, model, &user_message).await {
                                Ok(response) => {
                                    if let Err(e) = msg.reply(response).await {
                                        eprintln!("Ошибка при отправке ответа: {}", e);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Ошибка при генерации ответа: {}", e);
                                }
                            }
                            break; // Process one message per cycle
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn generate_response(
    client: &OpenAIClient<OpenAIConfig>,
    model: &str,
    user_message: &str,
) -> Result<String> {
    let request = CreateChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: async_openai::types::ChatCompletionRequestSystemMessageContent::Text(
                    SYSTEM_INSTRUCTIONS.to_string(),
                ),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(
                    user_message.to_string(),
                ),
                name: None,
            }),
        ],
        temperature: Some(0.7),
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
        .unwrap_or_else(|| "Не удалось сгенерировать ответ.".to_string());

    Ok(content)
}
