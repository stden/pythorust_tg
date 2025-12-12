//! Simple chat export command
//!
//! Equivalent to Python's tg.py

use crate::chat::resolve_chat;
use crate::config::{ChatEntity, Config, MEDIA_REACTION_THRESHOLD_TG};
use crate::error::Result;
use crate::export::{create_media_dir, ExportWriter};
use crate::session::{get_client, SessionLock};
use grammers_client::types::Message;
use tracing::info;

pub async fn run(chat_name: &str, limit: usize) -> Result<()> {
    let config = Config::new();

    // Support direct ID input or config lookup
    let chat_entity = resolve_chat_entity(chat_name, &config)?;

    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    // Resolve chat
    let chat = resolve_chat(&client, &chat_entity).await?;

    info!("Reading messages from: {}", chat_name);

    // Collect messages
    let mut messages: Vec<Message> = Vec::new();
    let mut iter = client.iter_messages(&chat);

    while let Some(msg) = iter.next().await.transpose() {
        let msg = msg.map_err(|e| crate::error::Error::TelegramError(e.to_string()))?;
        messages.push(msg);
        if messages.len() >= limit {
            break;
        }
    }

    // Reverse for chronological order
    messages.reverse();

    // Create export writer
    let mut writer = ExportWriter::new(chat_name)?;
    writer.write_header(
        "Напиши сообщение которое соберёт максимум лайков (сердечек). Используй эмоджи:",
    )?;

    for msg in &messages {
        let sender_id = msg
            .sender()
            .map(|s| match s {
                grammers_client::types::peer::Peer::User(u) => u.raw.id(),
                grammers_client::types::peer::Peer::Group(g) => match &g.raw {
                    grammers_tl_types::enums::Chat::Empty(c) => c.id,
                    grammers_tl_types::enums::Chat::Chat(c) => c.id,
                    grammers_tl_types::enums::Chat::Forbidden(c) => c.id,
                    grammers_tl_types::enums::Chat::Channel(c) => c.id,
                    grammers_tl_types::enums::Chat::ChannelForbidden(c) => c.id,
                },
                grammers_client::types::peer::Peer::Channel(c) => c.raw.id,
            })
            .unwrap_or(0);

        let sender_name = writer.get_sender_name(sender_id, msg);
        let text = msg.text().to_string();
        // Note: reactions are not directly accessible in grammers 0.8
        let (reactions, emojis) = (0i32, String::new());
        let timestamp = msg.date();

        // Handle media
        if msg.media().is_some() {
            if reactions >= MEDIA_REACTION_THRESHOLD_TG {
                create_media_dir(chat_name)?;
                let file_path = format!("{}/media_{}.bin", chat_name, msg.id());
                println!(
                    "{} {}: {} {} --->",
                    timestamp.format("%d.%m.%Y %H:%M:%S"),
                    sender_name,
                    text,
                    emojis
                );
                // Note: actual download would need implementation
                println!(
                    "{} {}: {} {} {}",
                    timestamp.format("%d.%m.%Y %H:%M:%S"),
                    sender_name,
                    text,
                    emojis,
                    file_path
                );
                writer.write_message(
                    &sender_name,
                    &text,
                    &emojis,
                    Some(timestamp),
                    Some(&file_path),
                )?;
            } else {
                writer.write_message(
                    &sender_name,
                    &text,
                    &emojis,
                    Some(timestamp),
                    Some("[Media]"),
                )?;
            }
        } else {
            writer.write_message(&sender_name, &text, &emojis, None, None)?;
        }
    }

    writer.finish()?;

    println!("Экспорт завершён: {}.md", chat_name);

    Ok(())
}

/// Resolve chat input into a ChatEntity.
/// Supports:
/// - Config name lookup (e.g., "Хара")
/// - Direct numeric ID (e.g., "1187714594")
/// - Username with or without @ (e.g., "@username" or "username")
fn resolve_chat_entity(chat_input: &str, config: &Config) -> Result<ChatEntity> {
    // First, check config
    if let Some(entity) = config.get_chat(chat_input) {
        return Ok(entity.clone());
    }

    // Try parsing as numeric ID
    if let Ok(id) = chat_input.parse::<i64>() {
        // Return as group first (most common), resolve_chat will handle fallback
        return Ok(ChatEntity::Chat(id));
    }

    // Treat as username
    Ok(ChatEntity::username(chat_input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_chat_entity_prefers_config() {
        let mut config = Config::new();
        config.chats.clear();
        config
            .chats
            .insert("test_chat".into(), ChatEntity::Channel(12345));

        let entity = resolve_chat_entity("test_chat", &config).unwrap();
        assert!(matches!(entity, ChatEntity::Channel(12345)));
    }

    #[test]
    fn resolve_chat_entity_handles_numeric_id() {
        let mut config = Config::new();
        config.chats.clear();

        let entity = resolve_chat_entity("1187714594", &config).unwrap();
        assert!(matches!(entity, ChatEntity::Chat(1187714594)));
    }

    #[test]
    fn resolve_chat_entity_handles_username() {
        let mut config = Config::new();
        config.chats.clear();

        let entity = resolve_chat_entity("@testuser", &config).unwrap();
        assert!(matches!(entity, ChatEntity::Username(ref s) if s == "testuser"));
    }

    #[test]
    fn resolve_chat_entity_handles_username_without_at() {
        let mut config = Config::new();
        config.chats.clear();

        let entity = resolve_chat_entity("testuser", &config).unwrap();
        assert!(matches!(entity, ChatEntity::Username(ref s) if s == "testuser"));
    }
}
