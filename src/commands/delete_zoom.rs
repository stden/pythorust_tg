//! Delete Zoom messages command
//!
//! Equivalent to Python's delete_zoom_messages.py

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

const ZOOM_URL_PREFIX: &str = "https://us06web.zoom.us/";

fn contains_zoom_link(text: &str) -> bool {
    text.contains(ZOOM_URL_PREFIX)
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let mut preview: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

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
        if contains_zoom_link(text) {
            let timestamp = msg.date().format("%d.%m.%Y %H:%M:%S").to_string();
            let sender = if msg.outgoing() { "Я" } else { &name };

            let preview = preview_text(text, 50);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_zoom_link_detects_zoom_urls() {
        assert!(contains_zoom_link("Join: https://us06web.zoom.us/j/123"));
        assert!(!contains_zoom_link("https://example.com/zoom"));
    }

    #[test]
    fn preview_text_returns_full_text_when_short() {
        assert_eq!(preview_text("hello", 50), "hello");
    }

    #[test]
    fn preview_text_truncates_and_adds_ellipsis() {
        let text = "a".repeat(60);
        let preview = preview_text(&text, 50);
        assert_eq!(preview, format!("{}...", "a".repeat(50)));
    }

    #[test]
    fn preview_text_handles_unicode_safely() {
        let text = "Привет".repeat(20);
        let preview = preview_text(&text, 50);
        assert!(preview.ends_with("..."));
        assert!(preview.len() > 50);
    }
}
