//! List chats command
//!
//! Equivalent to Python's list_chats.py

use std::fs::File;
use std::io::Write;

use crate::error::Result;
use crate::session::{get_client, SessionLock};
use chrono::{DateTime, Utc};
use grammers_client::types::peer::Peer;

#[derive(Debug)]
struct ChatInfo {
    title: String,
    id: i64,
    last_message: DateTime<Utc>,
    unread: i32,
    chat_type: String,
}

/// Filter for chat types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChatFilter {
    All,
    Users,
    Groups,
    Channels,
}

pub async fn run(limit: usize) -> Result<()> {
    run_with_filter(limit, ChatFilter::All).await
}

/// Run with specific chat type filter
pub async fn run_with_filter(limit: usize, filter: ChatFilter) -> Result<()> {
    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    let mut chat_activity: Vec<ChatInfo> = Vec::new();
    let mut dialogs = client.iter_dialogs();

    let mut count = 0;
    while let Some(dialog) = dialogs.next().await.transpose() {
        let dialog = dialog.map_err(|e| crate::error::Error::TelegramError(e.to_string()))?;

        // dialog.peer is the chat in grammers 0.8
        let chat = &dialog.peer;
        let (chat_type_str, include) = match chat {
            Peer::Channel(_) => (
                "channel",
                filter == ChatFilter::All || filter == ChatFilter::Channels,
            ),
            Peer::Group(_) => (
                "group",
                filter == ChatFilter::All || filter == ChatFilter::Groups,
            ),
            Peer::User(u) => {
                // Skip bots - check via User enum variant
                let is_bot = match &u.raw {
                    grammers_tl_types::enums::User::User(user) => user.bot,
                    grammers_tl_types::enums::User::Empty(_) => false,
                };
                (
                    if is_bot { "bot" } else { "user" },
                    (filter == ChatFilter::All || filter == ChatFilter::Users) && !is_bot,
                )
            }
        };

        if include {
            // Get latest message date
            let mut messages = client.iter_messages(chat);
            if let Some(msg) = messages.next().await.transpose() {
                let msg = msg.map_err(|e| crate::error::Error::TelegramError(e.to_string()))?;

                let title = match chat {
                    Peer::Channel(c) => c.title().to_string(),
                    Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
                    Peer::User(u) => u.full_name(),
                };

                let id: i64 = match chat {
                    Peer::Channel(c) => c.raw.id,
                    Peer::Group(g) => match &g.raw {
                        grammers_tl_types::enums::Chat::Empty(c) => c.id,
                        grammers_tl_types::enums::Chat::Chat(c) => c.id,
                        grammers_tl_types::enums::Chat::Forbidden(c) => c.id,
                        grammers_tl_types::enums::Chat::Channel(c) => c.id,
                        grammers_tl_types::enums::Chat::ChannelForbidden(c) => c.id,
                    },
                    Peer::User(u) => u.raw.id(),
                };

                chat_activity.push(ChatInfo {
                    title,
                    id,
                    last_message: msg.date(),
                    unread: 0, // Would need dialog.unread_count
                    chat_type: chat_type_str.to_string(),
                });
            }
        }

        count += 1;
        if count >= 200 {
            // Increased limit to get more chats
            break;
        }
    }

    // Sort by last message date (newest first)
    chat_activity.sort_by(|a, b| b.last_message.cmp(&a.last_message));

    println!("Наиболее активные чаты:\n");

    for (i, chat) in chat_activity.iter().take(limit).enumerate() {
        println!("{}. {}", i + 1, chat.title);
        println!(
            "   ID: {} | Тип: {} | Непрочитано: {}",
            chat.id, chat.chat_type, chat.unread
        );
        println!(
            "   Последнее сообщение: {}",
            chat.last_message.format("%d.%m.%Y %H:%M")
        );
        println!();
    }

    // Save to YAML file
    let mut file = File::create("chats.yml")?;
    writeln!(file, "# Активные чаты Telegram")?;
    if let Some(first) = chat_activity.first() {
        writeln!(
            file,
            "# Обновлено: {}\n",
            first.last_message.format("%d.%m.%Y %H:%M")
        )?;
    }
    writeln!(file, "chats:")?;

    for chat in &chat_activity {
        writeln!(file, "  - title: \"{}\"", chat.title)?;
        writeln!(file, "    id: {}", chat.id)?;
        writeln!(file, "    type: {}", chat.chat_type)?;
        writeln!(file, "    unread: {}", chat.unread)?;
        writeln!(
            file,
            "    last_message: \"{}\"",
            chat.last_message.format("%d.%m.%Y %H:%M:%S")
        )?;
        writeln!(file)?;
    }

    println!(
        "\nИнформация сохранена в chats.yml ({} чатов)",
        chat_activity.len()
    );

    Ok(())
}
