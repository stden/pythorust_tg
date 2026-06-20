//! CRM Parser - Extract contacts, companies, and deal stages from chat messages
//!
//! Based on the CRM idea from example_channel chat - automatically parse conversations
//! to extract business information

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};
use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequest,
    },
    Client as OpenAIClient,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const CRM_EXTRACTION_PROMPT: &str = r#"Ты — эксперт по CRM и продажам. Проанализируй переписку и извлеки структурированные данные.

Извлеки следующую информацию в JSON формате:
{
  "contacts": [
    {
      "name": "Имя контакта",
      "company": "Название компании",
      "role": "Должность",
      "phone": "телефон если есть",
      "email": "email если есть",
      "telegram": "@username если есть"
    }
  ],
  "deals": [
    {
      "title": "Краткое название сделки",
      "description": "Описание что обсуждается",
      "stage": "один из: lead|qualification|proposal|negotiation|closed_won|closed_lost",
      "estimated_value": "сумма если упоминается",
      "next_action": "следующий шаг",
      "deadline": "дедлайн если упоминается"
    }
  ],
  "action_items": [
    {
      "task": "описание задачи",
      "assignee": "кто должен сделать",
      "due_date": "когда если известно"
    }
  ],
  "sentiment": "positive|neutral|negative",
  "summary": "краткое резюме переписки в 1-2 предложения"
}

Отвечай ТОЛЬКО валидным JSON без дополнительного текста."#;

/// Extracted contact information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contact {
    pub name: Option<String>,
    pub company: Option<String>,
    pub role: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub telegram: Option<String>,
}

/// Deal/opportunity information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Deal {
    pub title: Option<String>,
    pub description: Option<String>,
    pub stage: Option<String>,
    pub estimated_value: Option<String>,
    pub next_action: Option<String>,
    pub deadline: Option<String>,
}

/// Action item from conversation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionItem {
    pub task: Option<String>,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
}

/// Full CRM extraction result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrmExtraction {
    pub contacts: Vec<Contact>,
    pub deals: Vec<Deal>,
    pub action_items: Vec<ActionItem>,
    pub sentiment: Option<String>,
    pub summary: Option<String>,
}

/// CRM parser configuration
pub struct CrmConfig {
    /// OpenAI model to use
    pub model: String,
    /// Maximum messages to analyze
    pub max_messages: usize,
}

impl Default for CrmConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            max_messages: 100,
        }
    }
}

/// Parse chat for CRM data
pub async fn parse_chat(chat_name: &str, config: CrmConfig) -> Result<CrmExtraction> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| Error::InvalidArgument("OPENAI_API_KEY not set".to_string()))?;

    let openai_config = OpenAIConfig::new().with_api_key(api_key);
    let openai_client = OpenAIClient::with_config(openai_config);

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    println!("🔍 Анализирую чат '{}' для CRM...", chat_name);

    let chat = crate::chat::find_chat(&client, chat_name).await?;

    // Collect messages
    let mut messages: Vec<(String, String, DateTime<Utc>)> = Vec::new();
    let mut iter = client.iter_messages(&chat);

    while let Some(msg_result) = iter.next().await.transpose() {
        if messages.len() >= config.max_messages {
            break;
        }

        if let Ok(msg) = msg_result {
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

            let timestamp: DateTime<Utc> = msg.date();
            messages.push((sender, text, timestamp));
        }
    }

    if messages.is_empty() {
        return Ok(CrmExtraction::default());
    }

    // Reverse to chronological order
    messages.reverse();

    // Prepare conversation text
    let conversation = messages
        .iter()
        .map(|(sender, text, ts)| format!("[{}] {}: {}", ts.format("%d.%m %H:%M"), sender, text))
        .collect::<Vec<_>>()
        .join("\n");

    // Extract CRM data with AI
    let extraction = extract_crm_data(&openai_client, &config.model, &conversation).await?;

    Ok(extraction)
}

async fn extract_crm_data(
    client: &OpenAIClient<OpenAIConfig>,
    model: &str,
    conversation: &str,
) -> Result<CrmExtraction> {
    let request = CreateChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(
                    CRM_EXTRACTION_PROMPT.to_string(),
                ),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(format!(
                    "Проанализируй эту переписку:\n\n{}",
                    conversation
                )),
                name: None,
            }),
        ],
        temperature: Some(0.3), // Lower temperature for more consistent JSON
        max_completion_tokens: Some(2000),
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
        .unwrap_or_default();

    // Parse JSON response
    let extraction: CrmExtraction = serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("⚠️ Ошибка парсинга JSON: {}", e);
        eprintln!("Ответ AI: {}", content);
        CrmExtraction::default()
    });

    Ok(extraction)
}

/// Export CRM data to CSV
pub fn export_contacts_csv(extraction: &CrmExtraction) -> String {
    let mut csv = String::from("name,company,role,phone,email,telegram\n");

    for contact in &extraction.contacts {
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            contact.name.as_deref().unwrap_or(""),
            contact.company.as_deref().unwrap_or(""),
            contact.role.as_deref().unwrap_or(""),
            contact.phone.as_deref().unwrap_or(""),
            contact.email.as_deref().unwrap_or(""),
            contact.telegram.as_deref().unwrap_or(""),
        ));
    }

    csv
}

/// Export deals to CSV
pub fn export_deals_csv(extraction: &CrmExtraction) -> String {
    let mut csv = String::from("title,description,stage,value,next_action,deadline\n");

    for deal in &extraction.deals {
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            deal.title.as_deref().unwrap_or(""),
            deal.description.as_deref().unwrap_or(""),
            deal.stage.as_deref().unwrap_or(""),
            deal.estimated_value.as_deref().unwrap_or(""),
            deal.next_action.as_deref().unwrap_or(""),
            deal.deadline.as_deref().unwrap_or(""),
        ));
    }

    csv
}

/// Print CRM extraction in human-readable format
pub fn print_extraction(extraction: &CrmExtraction) {
    println!("\n📊 CRM Extraction Results\n");

    if let Some(summary) = &extraction.summary {
        println!("📝 Summary: {}\n", summary);
    }

    if let Some(sentiment) = &extraction.sentiment {
        let emoji = match sentiment.as_str() {
            "positive" => "😊",
            "negative" => "😟",
            _ => "😐",
        };
        println!("💭 Sentiment: {} {}\n", sentiment, emoji);
    }

    if !extraction.contacts.is_empty() {
        println!("👥 Contacts ({}):", extraction.contacts.len());
        for contact in &extraction.contacts {
            println!(
                "  • {} ({}) @ {}",
                contact.name.as_deref().unwrap_or("Unknown"),
                contact.role.as_deref().unwrap_or("N/A"),
                contact.company.as_deref().unwrap_or("N/A")
            );
            if let Some(email) = &contact.email {
                println!("    📧 {}", email);
            }
            if let Some(phone) = &contact.phone {
                println!("    📞 {}", phone);
            }
        }
        println!();
    }

    if !extraction.deals.is_empty() {
        println!("💼 Deals ({}):", extraction.deals.len());
        for deal in &extraction.deals {
            println!(
                "  • {} [{}]",
                deal.title.as_deref().unwrap_or("Untitled"),
                deal.stage.as_deref().unwrap_or("unknown")
            );
            if let Some(value) = &deal.estimated_value {
                println!("    💰 {}", value);
            }
            if let Some(action) = &deal.next_action {
                println!("    ➡️ Next: {}", action);
            }
        }
        println!();
    }

    if !extraction.action_items.is_empty() {
        println!("✅ Action Items ({}):", extraction.action_items.len());
        for item in &extraction.action_items {
            println!(
                "  □ {} ({})",
                item.task.as_deref().unwrap_or("No description"),
                item.assignee.as_deref().unwrap_or("Unassigned")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_contacts_csv() {
        let extraction = CrmExtraction {
            contacts: vec![Contact {
                name: Some("Sample Contact".to_string()),
                company: Some("Acme Corp".to_string()),
                role: Some("CEO".to_string()),
                email: Some("contact@example.com".to_string()),
                phone: None,
                telegram: Some("@sample_contact".to_string()),
            }],
            ..Default::default()
        };

        let csv = export_contacts_csv(&extraction);
        assert!(csv.contains("Sample Contact"));
        assert!(csv.contains("Acme Corp"));
        assert!(csv.contains("contact@example.com"));
    }

    #[test]
    fn test_export_deals_csv() {
        let extraction = CrmExtraction {
            deals: vec![Deal {
                title: Some("Big Deal".to_string()),
                stage: Some("negotiation".to_string()),
                estimated_value: Some("$10000".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let csv = export_deals_csv(&extraction);
        assert!(csv.contains("Big Deal"));
        assert!(csv.contains("negotiation"));
        assert!(csv.contains("$10000"));
    }
}
