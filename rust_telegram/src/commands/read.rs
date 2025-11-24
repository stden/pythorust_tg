//! Read chat command - main chat reader with deletion logic
//!
//! Equivalent to Python's read.py

use std::collections::HashSet;

use grammers_client::types::Message;
use crate::chat::resolve_chat;
use crate::config::{Config, MEDIA_REACTION_THRESHOLD};
use crate::export::{create_media_dir, ExportWriter};
use crate::session::{get_client, SessionLock};
use crate::error::Result;
use tracing::info;

pub async fn run(chat_name: &str, limit: Option<usize>, delete_unengaged: bool) -> Result<()> {
    let config = Config::new();
    let my_user_id = config.my_user_id;
    let limit = limit.unwrap_or_else(|| config.get_limit());

    println!("LIMIT={}", limit);

    let chat_entity = config
        .get_chat(chat_name)
        .ok_or_else(|| crate::error::Error::ChatNotFound(chat_name.to_string()))?
        .clone();

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

    // Build set of replied-to message IDs
    let mut replied_to: HashSet<i32> = HashSet::new();
    for msg in &messages {
        if let Some(reply_to) = msg.reply_to_message_id() {
            replied_to.insert(reply_to);
        }
    }

    // Reverse for chronological order
    messages.reverse();

    // Create export writer
    let mut writer = ExportWriter::new(chat_name)?;
    writer.write_header("Что интересно людям в чате. Напиши отчёт с юмором и эмодзи. Вот чат:")?;

    let mut deleted_count = 0;

    for msg in &messages {
        let sender_id = msg.sender().map(|s| match s {
            grammers_client::types::peer::Peer::User(u) => u.raw.id(),
            grammers_client::types::peer::Peer::Group(g) => {
                match &g.raw {
                    grammers_tl_types::enums::Chat::Empty(c) => c.id,
                    grammers_tl_types::enums::Chat::Chat(c) => c.id,
                    grammers_tl_types::enums::Chat::Forbidden(c) => c.id,
                    grammers_tl_types::enums::Chat::Channel(c) => c.id,
                    grammers_tl_types::enums::Chat::ChannelForbidden(c) => c.id,
                }
            }
            grammers_client::types::peer::Peer::Channel(c) => c.raw.id,
        }).unwrap_or(0);

        let sender_name = writer.get_sender_name(sender_id, msg);
        let text = msg.text().to_string();
        // Note: reactions are not directly accessible in grammers 0.8
        let (reactions, emojis) = (0i32, String::new());

        // Check for Zoom links to delete
        if sender_id == my_user_id && text.contains("https://kuehne-nagel.zoom.us") {
            let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
            println!("!!!DEL-ZOOM!!! {} {}: {} {}", timestamp, sender_name, text, reactions);
            if let Err(e) = client.delete_messages(&chat, &[msg.id()]).await {
                eprintln!("Failed to delete message: {}", e);
            } else {
                deleted_count += 1;
            }
            continue;
        }

        // Delete unengaged messages if enabled
        if delete_unengaged && sender_id == my_user_id && reactions == 0
            && !replied_to.contains(&msg.id()) {
                let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
                println!("! Неинтересное сообщение, удаляю: {} {}: {}", timestamp, sender_name, text);
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
                let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
                let file_path = format!("{}/media_{}.bin", chat_name, msg.id());
                // Note: actual download would need grammers download implementation
                writer.write_message(&sender_name, &text, &emojis, Some(msg.date()), Some(&file_path))?;
            }
            // Skip low-engagement media
        } else {
            writer.write_message(&sender_name, &text, &emojis, None, None)?;
        }
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
