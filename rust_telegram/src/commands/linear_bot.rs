//! Linear bot - Telegram bot for creating Linear issues.

use std::collections::HashSet;
use std::env;

use grammers_client::types::peer::Peer;
use grammers_client::types::Message;
use grammers_client::Client;
use regex::Regex;
use tokio::signal;

use crate::linear::{CreateIssueInput, LinearClient};
use crate::error::{Error, Result};

/// Linear bot configuration.
#[derive(Debug, Clone)]
pub struct LinearBotConfig {
    pub command_prefix: String,
    pub team_key: Option<String>,
    pub project_id: Option<String>,
    pub default_priority: i32,
    pub allowed_sender_ids: HashSet<i64>,
}

impl Default for LinearBotConfig {
    fn default() -> Self {
        Self {
            command_prefix: "!linear".to_string(),
            team_key: None,
            project_id: None,
            default_priority: 1,
            allowed_sender_ids: HashSet::new(),
        }
    }
}

impl LinearBotConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let command_prefix = env::var("LINEAR_COMMAND_PREFIX")
            .unwrap_or_else(|_| "!linear".to_string())
            .trim()
            .to_string();

        let team_key = env::var("LINEAR_TEAM_KEY").ok();
        let project_id = env::var("LINEAR_PROJECT_ID").ok();

        let default_priority = env::var("LINEAR_DEFAULT_PRIORITY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let allowed_sender_ids = env::var("LINEAR_ALLOWED_SENDERS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();

        Self {
            command_prefix: if command_prefix.is_empty() {
                "!linear".to_string()
            } else {
                command_prefix
            },
            team_key,
            project_id,
            default_priority,
            allowed_sender_ids,
        }
    }
}

/// Extract sender ID from peer.
fn get_sender_id(sender: Option<&Peer>) -> i64 {
    sender
        .map(|s| match s {
            Peer::User(u) => u.raw.id(),
            Peer::Group(g) => match &g.raw {
                grammers_tl_types::enums::Chat::Chat(c) => c.id,
                grammers_tl_types::enums::Chat::Forbidden(f) => f.id,
                _ => 0,
            },
            Peer::Channel(c) => c.raw.id,
        })
        .unwrap_or(0)
}

/// Linear bot handler.
pub struct LinearBot {
    config: LinearBotConfig,
    linear: LinearClient,
    command_pattern: Regex,
}

impl LinearBot {
    /// Create new bot with config.
    pub fn new(config: LinearBotConfig) -> Result<Self> {
        let linear = LinearClient::from_optional_key(None)?;
        let pattern = format!(r"^{}\s+(.+)", regex::escape(&config.command_prefix));
        let command_pattern = Regex::new(&pattern)
            .map_err(|e| Error::InvalidArgument(format!("Invalid regex: {}", e)))?;

        Ok(Self {
            config,
            linear,
            command_pattern,
        })
    }

    /// Check if message matches command pattern.
    pub fn matches(&self, text: &str) -> Option<String> {
        self.command_pattern
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Check if sender is allowed.
    pub fn is_sender_allowed(&self, sender_id: i64) -> bool {
        self.config.allowed_sender_ids.is_empty()
            || self.config.allowed_sender_ids.contains(&sender_id)
    }

    /// Split payload into title and description.
    fn split_payload(payload: &str) -> (String, Option<String>) {
        if let Some((title, desc)) = payload.split_once('|') {
            let desc = desc.trim();
            (
                title.trim().to_string(),
                if desc.is_empty() { None } else { Some(desc.to_string()) },
            )
        } else {
            (payload.trim().to_string(), None)
        }
    }

    /// Merge description with reply text.
    fn merge_description(description: Option<&str>, reply_text: Option<&str>) -> Option<String> {
        let reply_text = reply_text.map(|s| s.trim()).filter(|s| !s.is_empty());
        let description = description.map(|s| s.trim()).filter(|s| !s.is_empty());

        match (description, reply_text) {
            (Some(desc), Some(reply)) => {
                Some(format!("{}\n\nИсходное сообщение из Telegram:\n{}", desc, reply))
            }
            (None, Some(reply)) => Some(format!("Исходное сообщение из Telegram:\n{}", reply)),
            (Some(desc), None) => Some(desc.to_string()),
            (None, None) => None,
        }
    }

    /// Handle incoming message.
    pub async fn handle_message(
        &self,
        _client: &Client,
        msg: &Message,
        reply_text: Option<&str>,
    ) -> Result<String> {
        let text = msg.text();

        // Check command pattern
        let payload = self
            .matches(text)
            .ok_or_else(|| Error::InvalidArgument("Message doesn't match command".to_string()))?;

        // Check sender
        let sender_id = get_sender_id(msg.sender());
        if !self.is_sender_allowed(sender_id) {
            return Err(Error::InvalidArgument("Sender not allowed".to_string()));
        }

        // Check team key
        let team_key = self
            .config
            .team_key
            .as_ref()
            .ok_or_else(|| Error::InvalidArgument("LINEAR_TEAM_KEY не задан".to_string()))?;

        if payload.is_empty() {
            return Err(Error::InvalidArgument(format!(
                "Использование: {} <заголовок> | <описание>",
                self.config.command_prefix
            )));
        }

        let (title, description) = Self::split_payload(&payload);
        if title.is_empty() {
            return Err(Error::InvalidArgument(
                "Нужно указать заголовок задачи после команды".to_string(),
            ));
        }

        let full_description = Self::merge_description(description.as_deref(), reply_text);

        let input = CreateIssueInput {
            team_key: team_key.clone(),
            title,
            description: full_description,
            project_id: self.config.project_id.clone(),
            priority: Some(self.config.default_priority),
            assignee_id: None,
            label_ids: vec![],
        };

        let issue = self.linear.create_issue(input).await?;

        let identifier = issue.identifier.as_deref().unwrap_or("");
        let url = issue.url.as_deref().unwrap_or("");

        Ok(format!("Создана задача {} {}", identifier, url).trim().to_string())
    }
}

/// Run Linear bot (polling version for grammers 0.8).
pub async fn run_linear_bot(client: Client) -> Result<()> {
    let config = LinearBotConfig::from_env();
    let bot = LinearBot::new(config.clone())?;

    let allowed_str = if config.allowed_sender_ids.is_empty() {
        "все".to_string()
    } else {
        config
            .allowed_sender_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    tracing::info!(
        "Linear bot запущен. Команда {}, команда: {}, разрешённые отправители: {}",
        config.command_prefix,
        config.team_key.as_deref().unwrap_or("не задана"),
        allowed_str
    );

    // Polling implementation for grammers 0.8
    let mut last_seen_id: Option<i32> = None;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                tracing::info!("Остановка бота...");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                // Poll dialogs for new messages with command
                let mut dialogs = client.iter_dialogs();

                while let Some(dialog) = dialogs.next().await.transpose() {
                    if let Ok(dialog) = dialog {
                        let chat = &dialog.peer;
                        let mut messages = client.iter_messages(chat);

                        if let Some(Ok(msg)) = messages.next().await.transpose() {
                            let msg_id = msg.id();
                            if let Some(last_id) = last_seen_id {
                                if msg_id <= last_id {
                                    continue;
                                }
                            }

                            if msg.outgoing() {
                                last_seen_id = Some(msg_id);
                                continue;
                            }

                            let text = msg.text();
                            if bot.matches(text).is_some() {
                                match bot.handle_message(&client, &msg, None).await {
                                    Ok(response) => {
                                        if let Err(e) = msg.reply(response).await {
                                            tracing::error!("Failed to send reply: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Ошибка: {}", e);
                                        if let Err(e) = msg.reply(error_msg).await {
                                            tracing::error!("Failed to send error reply: {}", e);
                                        }
                                    }
                                }
                            }

                            last_seen_id = Some(msg_id);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_payload() {
        let (title, desc) = LinearBot::split_payload("Title | Description");
        assert_eq!(title, "Title");
        assert_eq!(desc, Some("Description".to_string()));

        let (title, desc) = LinearBot::split_payload("Just title");
        assert_eq!(title, "Just title");
        assert_eq!(desc, None);

        let (title, desc) = LinearBot::split_payload("Title | ");
        assert_eq!(title, "Title");
        assert_eq!(desc, None);
    }

    #[test]
    fn test_merge_description() {
        let result = LinearBot::merge_description(Some("Desc"), Some("Reply"));
        assert!(result.unwrap().contains("Desc"));

        let result = LinearBot::merge_description(None, Some("Reply"));
        assert!(result.unwrap().contains("Исходное сообщение"));

        let result = LinearBot::merge_description(Some("Desc"), None);
        assert_eq!(result, Some("Desc".to_string()));

        let result = LinearBot::merge_description(None, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_config_from_env() {
        // Just test it doesn't panic
        let _config = LinearBotConfig::from_env();
    }
}
