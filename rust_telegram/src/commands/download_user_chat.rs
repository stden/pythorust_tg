//! Download chat with a user by username.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use grammers_client::types::peer::Peer;
use grammers_client::types::Message;
use grammers_client::Client;
use regex::Regex;

use crate::config::KNOWN_SENDERS;
use crate::error::{Error, Result};

/// Sanitize to safe filename.
fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r"[^\w\s\-]").unwrap();
    let cleaned = re.replace_all(name, "");
    let re_spaces = Regex::new(r"\s+").unwrap();
    let result = re_spaces.replace_all(cleaned.trim(), "_");
    let truncated: String = result.chars().take(50).collect();
    if truncated.is_empty() {
        "unknown".to_string()
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

/// Download chat messages with a user by username.
pub async fn download_user_chat(client: &Client, username: &str, limit: usize) -> Result<String> {
    let username = username.trim_start_matches('@');
    tracing::info!("Connecting to @{}...", username);

    let chat = client
        .resolve_username(username)
        .await?
        .ok_or_else(|| Error::ChatNotFound(username.to_string()))?;

    let display_name = chat.name().unwrap_or("Unknown");
    let filename = sanitize_filename(username);
    tracing::info!("User: {} (@{}) -> {}.md", display_name, username, filename);

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
    content.push_str(&format!("# Chat with {}\n", display_name));
    content.push_str(&format!("Username: @{}\n", username));
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
        assert_eq!(sanitize_filename("@exampleuser"), "exampleuser");
        assert_eq!(sanitize_filename("some user"), "some_user");
        assert_eq!(sanitize_filename(""), "unknown");
    }
}
