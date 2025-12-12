//! Download chat by ID command.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use grammers_client::types::peer::Peer;
use grammers_client::types::Message;
use grammers_client::Client;
use regex::Regex;

use crate::config::KNOWN_SENDERS;
use crate::error::{Error, Result};

/// Sanitize chat title to safe filename.
fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r"[^\w\s\-]").unwrap();
    let cleaned = re.replace_all(name, "");
    let re_spaces = Regex::new(r"\s+").unwrap();
    let result = re_spaces.replace_all(cleaned.trim(), "_");
    let truncated: String = result.chars().take(50).collect();
    if truncated.is_empty() {
        "unknown_chat".to_string()
    } else {
        truncated
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

/// Download chat messages by channel ID.
pub async fn download_chat(client: &Client, chat_id: i64, limit: usize) -> Result<String> {
    tracing::info!("Connecting to chat {}...", chat_id);

    // Try to resolve by ID as string
    let chat = client
        .resolve_username(&chat_id.to_string())
        .await
        .ok()
        .flatten()
        .ok_or_else(|| Error::ChatNotFound(chat_id.to_string()))?;

    let chat_title = chat.name().unwrap_or("Unknown");
    let filename = sanitize_filename(chat_title);
    tracing::info!("Chat: {} -> {}.md", chat_title, filename);

    tracing::info!("Downloading {} messages...", limit);

    let mut messages: Vec<Message> = Vec::new();
    let mut iter = client.iter_messages(&chat);

    while let Some(msg) = iter.next().await? {
        messages.push(msg);
        if messages.len() >= limit {
            break;
        }
    }

    tracing::info!("Got {} messages", messages.len());

    // Create output directory
    let chats_dir = Path::new("chats");
    fs::create_dir_all(chats_dir)?;

    let output_path = chats_dir.join(format!("{}.md", filename));

    // Build markdown content
    let mut content = String::new();
    content.push_str(&format!("# {}\n", chat_title));
    content.push_str(&format!("Chat ID: {}\n", chat_id));
    content.push_str(&format!("Messages: {}\n\n---\n\n", messages.len()));

    let mut known_senders: HashMap<i64, String> = KNOWN_SENDERS
        .iter()
        .map(|(k, v)| (*k, v.to_string()))
        .collect();

    // Reverse to get oldest first
    for msg in messages.iter().rev() {
        let sender_id = get_sender_id(msg.sender());

        let sender_name = if let Some(name) = known_senders.get(&sender_id) {
            name.clone()
        } else {
            let name = msg
                .sender()
                .as_ref()
                .and_then(|s| s.name())
                .map(|n| n.to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            known_senders.insert(sender_id, name.clone());
            name
        };

        let timestamp = msg.date().format("%d.%m.%Y %H:%M").to_string();

        // Build text
        let mut text = msg.text().to_string();
        if msg.media().is_some() {
            if text.is_empty() {
                text = "[Media]".to_string();
            } else {
                text.push_str(" [Media]");
            }
        }

        if !text.trim().is_empty() {
            content.push_str(&format!(
                "**{}** ({}):\n{}\n\n",
                sender_name, timestamp, text
            ));
        }
    }

    fs::write(&output_path, content)?;
    let path_str = output_path.to_string_lossy().to_string();
    tracing::info!("Saved: {}", path_str);

    Ok(path_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World!"), "Hello_World");
        assert_eq!(sanitize_filename("Тест чат 😀"), "Тест_чат");
        assert_eq!(sanitize_filename("   spaces   "), "spaces");
        assert_eq!(sanitize_filename(""), "unknown_chat");
    }
}
