//! Generate a daily/weekly digest of chat activity

use anyhow::Result;
use chrono::{DateTime, Duration, Local, Timelike, Utc};
use clap::{Parser, ValueEnum};
use std::collections::HashMap;
use telegram_reader::chat::resolve_chat;
use telegram_reader::config::{ChatEntity, Config};
use telegram_reader::get_client;
use telegram_reader::session::SessionLock;

#[derive(Parser)]
#[command(name = "message_digest")]
#[command(about = "Generate a digest of chat activity")]
struct Cli {
    /// Chat name from config or username
    chat: String,

    /// Time period for digest
    #[arg(short, long, default_value = "day")]
    period: Period,

    /// Show top N active users
    #[arg(short, long, default_value = "5")]
    top: usize,

    /// Include sample messages
    #[arg(long)]
    samples: bool,
}

#[derive(Clone, ValueEnum)]
enum Period {
    Day,
    Week,
    Month,
}

#[derive(Default)]
struct DigestStats {
    message_count: u32,
    media_count: u32,
    total_chars: usize,
    users: HashMap<i64, (String, u32)>,
    sample_messages: Vec<String>,
    busiest_hour: [u32; 24],
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

    let now = Local::now();
    let start_date = match cli.period {
        Period::Day => now - Duration::days(1),
        Period::Week => now - Duration::weeks(1),
        Period::Month => now - Duration::days(30),
    };

    let mut stats = DigestStats::default();
    let mut messages_iter = client.iter_messages(&peer);

    while let Some(message) = messages_iter.next().await? {
        let msg_date =
            DateTime::from_timestamp(message.date().timestamp(), 0).unwrap_or_else(Utc::now);

        // Stop if we've gone past our time window
        if msg_date < start_date.with_timezone(&Utc) {
            break;
        }

        stats.message_count += 1;

        if message.media().is_some() {
            stats.media_count += 1;
        }

        let text = message.text();
        stats.total_chars += text.len();

        // Track hourly activity
        let hour = msg_date.hour() as usize;
        stats.busiest_hour[hour] += 1;

        // Track user activity
        if let Some(sender) = message.sender() {
            let sender_id: i64 = sender.id().to_string().parse().unwrap_or(0);
            let sender_name = sender.name().unwrap_or("Unknown").to_string();

            let (_, count) = stats
                .users
                .entry(sender_id)
                .or_insert_with(|| (sender_name, 0));
            *count += 1;
        }

        // Collect sample messages (longer ones)
        if cli.samples && text.len() > 50 && stats.sample_messages.len() < 5 {
            stats.sample_messages.push(truncate(text, 100));
        }
    }

    // Print digest
    let period_name = match cli.period {
        Period::Day => "Daily",
        Period::Week => "Weekly",
        Period::Month => "Monthly",
    };

    println!("=== {} Digest for {} ===\n", period_name, cli.chat);
    println!(
        "Period: {} to {}",
        start_date.format("%Y-%m-%d"),
        now.format("%Y-%m-%d")
    );
    println!("Total messages: {}", stats.message_count);
    println!(
        "Media messages: {} ({:.1}%)",
        stats.media_count,
        if stats.message_count > 0 {
            (stats.media_count as f64 / stats.message_count as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("Active participants: {}", stats.users.len());
    println!(
        "Avg message length: {:.0} chars",
        if stats.message_count > 0 {
            stats.total_chars as f64 / stats.message_count as f64
        } else {
            0.0
        }
    );

    // Busiest hour
    let busiest = stats
        .busiest_hour
        .iter()
        .enumerate()
        .max_by_key(|(_, &count)| count)
        .map(|(hour, count)| (hour, *count))
        .unwrap_or((0, 0));
    println!("Busiest hour: {:02}:00 ({} messages)", busiest.0, busiest.1);

    // Top users
    println!("\n=== Top {} Active Users ===\n", cli.top);
    let mut sorted_users: Vec<_> = stats.users.into_iter().collect();
    sorted_users.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));

    for (i, (_, (name, count))) in sorted_users.iter().take(cli.top).enumerate() {
        let pct = (*count as f64 / stats.message_count as f64) * 100.0;
        println!("{}. {} - {} messages ({:.1}%)", i + 1, name, count, pct);
    }

    // Sample messages
    if cli.samples && !stats.sample_messages.is_empty() {
        println!("\n=== Sample Messages ===\n");
        for msg in &stats.sample_messages {
            println!("• {}", msg);
        }
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
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
