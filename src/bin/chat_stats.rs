//! Chat statistics utility - shows message counts, top senders, activity

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use std::collections::HashMap;
use telegram_reader::chat::resolve_chat;
use telegram_reader::config::{ChatEntity, Config};
use telegram_reader::get_client;
use telegram_reader::session::SessionLock;

#[derive(Parser)]
#[command(name = "chat_stats")]
#[command(about = "Show statistics for a Telegram chat")]
struct Cli {
    /// Chat name from config or username
    chat: String,

    /// Message limit to analyze
    #[arg(short, long, default_value = "1000")]
    limit: usize,

    /// Show top N senders
    #[arg(short, long, default_value = "10")]
    top: usize,
}

#[derive(Default)]
struct UserStats {
    message_count: u32,
    total_chars: usize,
    reactions_received: u32,
    first_message: Option<DateTime<Utc>>,
    last_message: Option<DateTime<Utc>>,
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

    println!("Analyzing chat: {}...\n", cli.chat);

    let mut user_stats: HashMap<i64, (String, UserStats)> = HashMap::new();
    let mut total_messages = 0;
    let total_reactions = 0;
    let mut messages_with_media = 0;
    let mut first_msg_date: Option<DateTime<Utc>> = None;
    let mut last_msg_date: Option<DateTime<Utc>> = None;

    let mut messages_iter = client.iter_messages(&peer);
    let mut count = 0;

    while let Some(message) = messages_iter.next().await? {
        if count >= cli.limit {
            break;
        }
        count += 1;
        total_messages += 1;

        let msg_date =
            DateTime::from_timestamp(message.date().timestamp(), 0).unwrap_or_else(Utc::now);

        if first_msg_date.is_none() || msg_date < first_msg_date.unwrap() {
            first_msg_date = Some(msg_date);
        }
        if last_msg_date.is_none() || msg_date > last_msg_date.unwrap() {
            last_msg_date = Some(msg_date);
        }

        if message.media().is_some() {
            messages_with_media += 1;
        }

        if let Some(sender) = message.sender() {
            let sender_id: i64 = sender.id().to_string().parse().unwrap_or(0);
            let sender_name = sender.name().unwrap_or("Unknown").to_string();
            let text_len = message.text().len();

            let (_, stats) = user_stats
                .entry(sender_id)
                .or_insert_with(|| (sender_name, UserStats::default()));

            stats.message_count += 1;
            stats.total_chars += text_len;

            if stats.first_message.is_none() || msg_date < stats.first_message.unwrap() {
                stats.first_message = Some(msg_date);
            }
            if stats.last_message.is_none() || msg_date > stats.last_message.unwrap() {
                stats.last_message = Some(msg_date);
            }
        }
    }

    // Print summary
    println!("=== Chat Statistics ===\n");
    println!("Total messages analyzed: {}", total_messages);
    println!(
        "Messages with media: {} ({:.1}%)",
        messages_with_media,
        (messages_with_media as f64 / total_messages as f64) * 100.0
    );
    println!("Unique participants: {}", user_stats.len());

    if let (Some(first), Some(last)) = (first_msg_date, last_msg_date) {
        let days = (last - first).num_days().max(1);
        println!(
            "Date range: {} to {}",
            first.format("%Y-%m-%d"),
            last.format("%Y-%m-%d")
        );
        println!(
            "Messages per day: {:.1}",
            total_messages as f64 / days as f64
        );
    }

    // Sort by message count
    let mut sorted: Vec<_> = user_stats.into_iter().collect();
    sorted.sort_by(|a, b| b.1 .1.message_count.cmp(&a.1 .1.message_count));

    println!("\n=== Top {} Senders ===\n", cli.top);
    println!(
        "{:<4} {:<25} {:>8} {:>10} {:>8}",
        "#", "Name", "Messages", "Chars", "Avg len"
    );
    println!("{}", "-".repeat(60));

    for (i, (id, (name, stats))) in sorted.iter().take(cli.top).enumerate() {
        let avg_len = if stats.message_count > 0 {
            stats.total_chars / stats.message_count as usize
        } else {
            0
        };
        println!(
            "{:<4} {:<25} {:>8} {:>10} {:>8}",
            i + 1,
            truncate(name, 24),
            stats.message_count,
            stats.total_chars,
            avg_len
        );
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}
