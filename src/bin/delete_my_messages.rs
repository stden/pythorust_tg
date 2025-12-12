//! Delete ALL my outgoing messages from a chat

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use grammers_client::types::peer::Peer;
use grammers_client::types::Message;
use telegram_reader::chat::find_chat;
use telegram_reader::get_client;
use telegram_reader::session::SessionLock;

#[derive(Parser)]
#[command(name = "delete_my_messages")]
#[command(about = "Delete ALL my outgoing messages from a chat")]
struct Cli {
    /// Chat name, ID, or username (@user)
    #[arg(long)]
    chat: String,

    /// Message limit to check (0 = unlimited)
    #[arg(short, long, default_value = "1000")]
    limit: usize,

    /// Dry run - don't actually delete
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    // Find chat by name, ID, or username
    let peer = find_chat(&client, &cli.chat).await?;
    let chat_name = peer.name().unwrap_or("Unknown");

    println!("Found chat: {}", chat_name);

    let deleted = process_chat(&client, &peer, chat_name, cli.limit, cli.dry_run).await?;

    println!("\n=== Deleted {} messages ===", deleted);

    Ok(())
}

async fn process_chat(
    client: &grammers_client::Client,
    peer: &Peer,
    chat_name: &str,
    limit: usize,
    dry_run: bool,
) -> Result<usize> {
    let mut messages_iter = client.iter_messages(peer);
    let mut count = 0;
    let mut to_delete: Vec<Message> = Vec::new();

    println!("Scanning messages in {}...", chat_name);

    // Collect all outgoing messages
    while let Some(message) = messages_iter.next().await? {
        if limit > 0 && count >= limit {
            break;
        }
        count += 1;

        // Only delete outgoing (my) messages
        if message.outgoing() {
            to_delete.push(message);
        }
    }

    println!(
        "Found {} outgoing messages out of {} total",
        to_delete.len(),
        count
    );

    // Delete messages
    let mut deleted = 0;
    for message in to_delete {
        let msg_date =
            DateTime::from_timestamp(message.date().timestamp(), 0).unwrap_or_else(Utc::now);
        let text_preview = truncate(message.text(), 50);

        if dry_run {
            println!(
                "  WOULD DELETE: {} - {}",
                msg_date.format("%d.%m.%Y %H:%M"),
                text_preview
            );
        } else {
            println!(
                "  DEL: {} - {}",
                msg_date.format("%d.%m.%Y %H:%M"),
                text_preview
            );
            if let Err(e) = message.delete().await {
                eprintln!("    Error deleting message: {}", e);
            } else {
                deleted += 1;
            }
        }
    }

    Ok(deleted)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.is_empty() {
        "[media]".to_string()
    } else if s.chars().count() <= max_len {
        s.replace('\n', " ")
    } else {
        format!(
            "{}...",
            s.chars()
                .take(max_len)
                .collect::<String>()
                .replace('\n', " ")
        )
    }
}
