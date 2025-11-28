//! Delete Zoom messages command
//!
//! Equivalent to Python's delete_zoom_messages.py

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

pub async fn run(username: &str, limit: usize) -> Result<()> {
    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    // Resolve username
    let chat = client
        .resolve_username(username)
        .await
        .map_err(|e| Error::TelegramError(e.to_string()))?
        .ok_or_else(|| Error::ChatNotFound(format!("Username @{} not found", username)))?;

    let name = match &chat {
        grammers_client::types::peer::Peer::User(u) => u.full_name(),
        grammers_client::types::peer::Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
        grammers_client::types::peer::Peer::Channel(c) => c.title().to_string(),
    };

    println!("Поиск сообщений с Zoom ссылками в чате с @{}", username);

    // Collect messages
    let mut messages = Vec::new();
    let mut iter = client.iter_messages(&chat);

    while let Some(msg) = iter.next().await.transpose() {
        let msg = msg.map_err(|e| Error::TelegramError(e.to_string()))?;
        messages.push(msg);
        if messages.len() >= limit {
            break;
        }
    }

    let mut deleted_count = 0;

    for msg in &messages {
        let text = msg.text();
        if text.contains("https://us06web.zoom.us/") {
            let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
            let sender = if msg.outgoing() { "Я" } else { &name };

            let preview = if text.len() > 50 {
                format!("{}...", &text[..50])
            } else {
                text.to_string()
            };

            println!("Удаляю: {} {}: {}", timestamp, sender, preview);

            // Delete message for both users (revoke)
            if let Err(e) = client.delete_messages(&chat, &[msg.id()]).await {
                eprintln!("Failed to delete message: {}", e);
            } else {
                deleted_count += 1;
            }
        }
    }

    println!("\nУдалено {} сообщений с Zoom ссылками", deleted_count);

    Ok(())
}
