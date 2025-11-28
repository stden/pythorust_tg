//! Экспорт произвольного чата по имени
//!
//! Export any Telegram chat by searching for its name in dialogs.
//! Equivalent to Python's export_any_chat.py

use chrono::Local;
use grammers_client::types::peer::Peer;
use std::fs;
use std::io::Write;
use telegram_reader::error::{Error, Result};
use telegram_reader::session::{get_client, SessionLock};

/// Export chat by searching dialogs for name match
async fn export_chat_by_name(chat_name: &str, limit: usize) -> Result<String> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    // Search through all dialogs
    let mut dialogs = client.iter_dialogs();
    let mut target_dialog = None;

    while let Some(dialog) = dialogs.next().await.transpose() {
        if let Ok(dialog) = dialog {
            let title = match &dialog.peer {
                Peer::User(u) => u.full_name(),
                Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
                Peer::Channel(c) => c.title().to_string(),
            };

            // Case-insensitive search
            if title.to_lowercase().contains(&chat_name.to_lowercase()) {
                target_dialog = Some((dialog.peer, title));
                break;
            }
        }
    }

    let (chat, title) = target_dialog
        .ok_or_else(|| Error::InvalidArgument(format!("❌ Чат '{}' не найден", chat_name)))?;

    println!("✅ Найден чат: {}", title);
    println!("Экспортирую последние {} сообщений...", limit);

    // Collect messages
    let mut messages = Vec::new();
    let mut iter = client.iter_messages(&chat);

    while let Some(msg) = iter.next().await.transpose() {
        let msg = msg.map_err(|e| Error::TelegramError(e.to_string()))?;

        // Only collect messages with text
        if !msg.text().is_empty() {
            messages.push(msg);
        }

        if messages.len() >= limit {
            break;
        }
    }

    // Reverse for chronological order (oldest first)
    messages.reverse();

    // Create output directory
    fs::create_dir_all("chats")?;

    // Create output file
    let safe_filename = chat_name.replace(' ', "_").replace('/', "_");
    let output_file = format!("chats/{}.txt", safe_filename);
    let mut file = fs::File::create(&output_file)?;

    // Write header
    let now = Local::now();
    writeln!(file, "# Чат: {}", title)?;
    writeln!(
        file,
        "# Экспортировано: {}",
        now.format("%Y-%m-%d %H:%M:%S")
    )?;
    writeln!(file, "# Сообщений: {}", messages.len())?;
    writeln!(file)?;

    // Write messages
    for msg in &messages {
        let date_str = msg.date().format("%d.%m.%Y %H:%M:%S");

        // Determine sender name
        let sender_name = if msg.outgoing() {
            "Я".to_string()
        } else {
            // Try to get sender from the message
            // For channels/groups, we might not have sender info
            match &chat {
                Peer::User(u) => u.full_name(),
                Peer::Channel(_) | Peer::Group(_) => {
                    // In groups/channels, use the title or "Unknown"
                    title.clone()
                }
            }
        };

        let text = msg.text();
        writeln!(file, "[{}] {}: {}", date_str, sender_name, text)?;
    }

    println!(
        "✅ Экспортировано {} сообщений → {}",
        messages.len(),
        output_file
    );

    Ok(output_file)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Использование: {} <chat_name> [limit]", args[0]);
        eprintln!();
        eprintln!("Примеры:");
        eprintln!("  {} \"Golang GO\" 500", args[0]);
        eprintln!("  {} вайбкодеры 1000", args[0]);
        std::process::exit(1);
    }

    let chat_name = &args[1];
    let limit = if args.len() > 2 {
        args[2].parse().unwrap_or_else(|_| {
            eprintln!("⚠️  Неверный формат лимита, используем 500");
            500
        })
    } else {
        500
    };

    match export_chat_by_name(chat_name, limit).await {
        Ok(output_file) => {
            println!("\n✅ Успешно экспортировано в {}", output_file);
            Ok(())
        }
        Err(e) => {
            eprintln!("\n❌ Ошибка: {}", e);
            std::process::exit(1);
        }
    }
}
