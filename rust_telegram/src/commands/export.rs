//! Export chat command
//!
//! Equivalent to Python's export_chat.py

use std::fs::File;
use std::io::Write;

use crate::session::{get_client, SessionLock};
use crate::error::{Result, Error};

pub async fn run(username: &str, output: Option<&str>, limit: usize) -> Result<()> {
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

    println!("Экспортирую чат: {} (@{})", name, username);

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

    // Reverse for chronological order
    messages.reverse();

    // Create output file
    let output_file = output
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}.md", username));

    let mut file = File::create(&output_file)?;
    writeln!(file, "# Чат с @{}\n", username)?;

    for msg in &messages {
        let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
        let is_outgoing = msg.outgoing();
        let sender = if is_outgoing { "Я" } else { &name };

        let text = msg.text();
        if !text.is_empty() {
            writeln!(file, "{} {}: {}", timestamp, sender, text)?;
        } else if msg.media().is_some() {
            writeln!(file, "{} {}: [Media]", timestamp, sender)?;
        }
    }

    println!("Экспортировано {} сообщений в {}", messages.len(), output_file);

    Ok(())
}
