//! Search messages in a chat by keyword or regex

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use regex::Regex;
use telegram_reader::chat::resolve_chat;
use telegram_reader::config::{ChatEntity, Config};
use telegram_reader::get_client;
use telegram_reader::session::SessionLock;

#[derive(Parser)]
#[command(name = "search_messages")]
#[command(about = "Search messages in a Telegram chat")]
struct Cli {
    /// Chat name from config or username
    chat: String,

    /// Search query (regex supported)
    query: String,

    /// Message limit to search through
    #[arg(short, long, default_value = "3000")]
    limit: usize,

    /// Case insensitive search
    #[arg(short, long)]
    ignore_case: bool,

    /// Only show messages from specific user ID
    #[arg(long)]
    user_id: Option<i64>,

    /// Only show my own messages
    #[arg(long)]
    outgoing: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let config = Config::new();
    let chat_entity = config
        .chats
        .get(&cli.chat)
        .cloned()
        .unwrap_or_else(|| ChatEntity::Username(cli.chat.clone()));

    let peer = resolve_chat(&client, &chat_entity).await?;

    let pattern = if cli.ignore_case {
        format!("(?i){}", cli.query)
    } else {
        cli.query.clone()
    };
    let regex = Regex::new(&pattern)?;

    println!("Searching for '{}' in {}...\n", cli.query, cli.chat);

    let mut messages_iter = client.iter_messages(&peer);
    let mut count = 0;
    let mut found = 0;

    while let Some(message) = messages_iter.next().await? {
        if count >= cli.limit {
            break;
        }
        count += 1;

        let text = message.text();
        if text.is_empty() {
            continue;
        }

        // Filter by outgoing
        if cli.outgoing && !message.outgoing() {
            continue;
        }

        // Filter by user ID
        if let Some(filter_user_id) = cli.user_id {
            if let Some(sender) = message.sender() {
                let sender_id: i64 = sender.id().to_string().parse().unwrap_or(0);
                if sender_id != filter_user_id {
                    continue;
                }
            } else {
                continue;
            }
        }

        if regex.is_match(text) {
            found += 1;

            let msg_date =
                DateTime::from_timestamp(message.date().timestamp(), 0).unwrap_or_else(Utc::now);

            let sender_name = message
                .sender()
                .map(|s| s.name().unwrap_or("Unknown").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            println!(
                "--- #{} [{}] {} ---",
                found,
                msg_date.format("%Y-%m-%d %H:%M"),
                sender_name
            );

            // Highlight matches
            let highlighted = regex.replace_all(text, "\x1b[1;33m$0\x1b[0m");
            println!("{}\n", highlighted);
        }
    }

    println!("Found {} matches in {} messages", found, count);

    Ok(())
}
