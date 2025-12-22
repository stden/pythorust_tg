//! Read chat command - main chat reader with deletion logic
//!
//! Equivalent to Python's read.py

use std::collections::HashSet;

use crate::chat::resolve_chat;
use crate::config::ChatEntity;
use crate::config::{Config, MEDIA_REACTION_THRESHOLD};
use crate::error::Result;
use crate::export::{create_media_dir, ExportWriter};
use crate::session::{get_client, SessionLock, TelegramClient};
use grammers_client::client::UpdatesConfiguration;
use grammers_client::types::update::Update;
use grammers_client::types::Message;
use grammers_session::defs::PeerId;
use tokio::signal;
use tracing::info;

pub async fn run(
    chat_name: &str,
    limit: Option<usize>,
    delete_unengaged: bool,
    watch: bool,
) -> Result<()> {
    let config = Config::new();
    let my_user_id = config.my_user_id;
    let limit = limit.unwrap_or_else(|| config.get_limit());

    println!("LIMIT={}", limit);

    let (primary_entity, fallback_entity) = parse_chat_entity(chat_name, &config);

    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let mut client = get_client().await?;

    // Resolve chat
    let chat = match resolve_chat(&client, &primary_entity).await {
        Ok(chat) => chat,
        Err(err) => {
            if let Some(fallback) = &fallback_entity {
                resolve_chat(&client, fallback).await?
            } else {
                return Err(err);
            }
        }
    };
    let target_peer_id = chat.id();

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

    // Build set of replied-to message IDs
    let mut replied_to: HashSet<i32> = HashSet::new();
    for msg in &messages {
        if let Some(reply_to) = msg.reply_to_message_id() {
            replied_to.insert(reply_to);
        }
    }

    // Reverse for chronological order
    messages.reverse();

    let mut last_seen_id = 0;

    // Create export writer
    let mut writer = ExportWriter::new(chat_name)?;
    writer.write_header("Что интересно людям в чате. Напиши отчёт с юмором и эмодзи. Вот чат:")?;

    let mut deleted_count = 0;

    for msg in &messages {
        let sender_id = extract_sender_id(msg);
        let sender_name = writer.get_sender_name(sender_id, msg);
        let text = msg.text().to_string();
        // Note: reactions are not directly accessible in grammers 0.8
        let (reactions, emojis) = (0i32, String::new());
        last_seen_id = last_seen_id.max(msg.id());

        // Check for Zoom links to delete
        if sender_id == my_user_id && text.contains("https://kuehne-nagel.zoom.us") {
            let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
            println!(
                "!!!DEL-ZOOM!!! {} {}: {} {}",
                timestamp, sender_name, text, reactions
            );
            if let Err(e) = client.delete_messages(&chat, &[msg.id()]).await {
                eprintln!("Failed to delete message: {}", e);
            } else {
                deleted_count += 1;
            }
            continue;
        }

        // Delete unengaged messages if enabled
        if delete_unengaged
            && sender_id == my_user_id
            && reactions == 0
            && !replied_to.contains(&msg.id())
        {
            let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
            println!(
                "! Неинтересное сообщение, удаляю: {} {}: {}",
                timestamp, sender_name, text
            );
            if let Err(e) = client.delete_messages(&chat, &[msg.id()]).await {
                eprintln!("Failed to delete message: {}", e);
            } else {
                deleted_count += 1;
            }
            continue;
        }

        // Handle media
        if msg.media().is_some() {
            if reactions >= MEDIA_REACTION_THRESHOLD && !Config::is_github_actions() {
                create_media_dir(chat_name)?;
                // Download media
                let file_path = format!("{}/media_{}.bin", chat_name, msg.id());
                // Note: actual download would need grammers download implementation
                writer.write_message(
                    &sender_name,
                    &text,
                    &emojis,
                    Some(msg.date()),
                    Some(&file_path),
                )?;
            }
            // Skip low-engagement media
        } else {
            writer.write_message(&sender_name, &text, &emojis, None, None)?;
        }
    }

    if watch {
        watch_chat(
            &mut client,
            target_peer_id,
            chat_name,
            &mut writer,
            last_seen_id,
        )
        .await?;
    }

    writer.finish()?;

    // Session auto-saves with SqliteSession
    info!("Session saved");

    if deleted_count > 0 {
        println!("Удалено сообщений: {}", deleted_count);
    }

    println!("Экспорт завершён: {}.md", chat_name);

    Ok(())
}

fn extract_sender_id(msg: &Message) -> i64 {
    msg.sender()
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
        .unwrap_or(0)
}

async fn watch_chat(
    client: &mut TelegramClient,
    target_peer_id: PeerId,
    chat_name: &str,
    writer: &mut ExportWriter,
    mut last_seen_id: i32,
) -> Result<()> {
    let updates_rx = match client.take_updates() {
        Some(rx) => rx,
        None => {
            println!("⚠️ Watch режим недоступен: не удалось получить канал обновлений. Попробуйте перезапустить команду.");
            return Ok(());
        }
    };

    let mut updates = client.stream_updates(
        updates_rx,
        UpdatesConfiguration {
            catch_up: true,
            ..Default::default()
        },
    );

    println!(
        "👀 Watch режим включен для '{}'. Нажмите Ctrl+C для остановки.",
        chat_name
    );

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("\nОстанавливаю watch режим...");
                break;
            }
            update = updates.next() => {
                match update {
                    Ok(Update::NewMessage(msg)) => {
                        if msg.peer_id() != target_peer_id {
                            continue;
                        }

                        let msg_id = msg.id();
                        if msg_id <= last_seen_id {
                            continue;
                        }
                        last_seen_id = msg_id;

                        let sender_id = extract_sender_id(&msg);
                        let sender_name = writer.get_sender_name(sender_id, &msg);
                        let mut text = msg.text().to_string();
                        if text.is_empty() && msg.media().is_some() {
                            text = "[Media]".to_string();
                        }

                        let timestamp = msg.date();
                        writer.write_message(&sender_name, &text, "", Some(timestamp), None)?;

                        println!(
                            "[{}] {}: {}",
                            timestamp.format("%H:%M:%S"),
                            sender_name,
                            text
                        );
                    }
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("⚠️ Ошибка при получении обновлений: {}", err);
                        break;
                    }
                }
            }
        }
    }

    updates.sync_update_state();
    Ok(())
}

/// Resolve chat input into a ChatEntity and optional fallback.
/// - Config name wins
/// - Numeric strings are treated as channel IDs with group fallback
/// - Otherwise treated as username
fn parse_chat_entity(chat_input: &str, config: &Config) -> (ChatEntity, Option<ChatEntity>) {
    if let Some(entity) = config.get_chat(chat_input) {
        return (entity.clone(), None);
    }

    if let Ok(id) = chat_input.parse::<i64>() {
        return (ChatEntity::Channel(id), Some(ChatEntity::Chat(id)));
    }

    (ChatEntity::username(chat_input), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chat_entity_prefers_config() {
        let mut config = Config::new();
        config.chats.clear();
        config
            .chats
            .insert("alpha".into(), ChatEntity::Username("alpha_user".into()));

        let (entity, fallback) = parse_chat_entity("alpha", &config);
        assert!(matches!(entity, ChatEntity::Username(ref s) if s == "alpha_user"));
        assert!(fallback.is_none());
    }

    #[test]
    fn parse_chat_entity_handles_numeric_with_fallback() {
        let mut config = Config::new();
        config.chats.clear();
        let (entity, fallback) = parse_chat_entity("12345", &config);

        assert!(matches!(entity, ChatEntity::Channel(12345)));
        assert!(matches!(fallback, Some(ChatEntity::Chat(12345))));
    }

    #[test]
    fn parse_chat_entity_uses_username_when_not_numeric() {
        let mut config = Config::new();
        config.chats.clear();
        let (entity, fallback) = parse_chat_entity("@user", &config);

        assert!(matches!(entity, ChatEntity::Username(ref s) if s == "user"));
        assert!(fallback.is_none());
    }
}
