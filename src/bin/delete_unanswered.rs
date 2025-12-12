//! Delete unanswered outgoing messages without reactions

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use clap::Parser;
use grammers_client::types::Message;
use std::collections::HashSet;
use telegram_reader::chat::resolve_chat;
use telegram_reader::config::{ChatEntity, Config};
use telegram_reader::get_client;
use telegram_reader::session::SessionLock;

#[derive(Parser)]
#[command(name = "delete_unanswered")]
#[command(about = "Delete unanswered outgoing messages without reactions")]
struct Cli {
    /// Chat name from config or username (@user)
    #[arg(long, conflicts_with = "chat_id")]
    chat: Option<String>,

    /// Chat/channel/user numeric ID
    #[arg(long, conflicts_with = "chat")]
    chat_id: Option<i64>,

    /// Message limit to check
    #[arg(short, long, default_value = "500")]
    limit: usize,

    /// Minimum message age in hours
    #[arg(long, default_value = "1")]
    hours: i64,

    /// Dry run - don't actually delete
    #[arg(long)]
    dry_run: bool,

    /// Process all chats from config
    #[arg(long)]
    all: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;
    let config = Config::new();

    if cli.all {
        let mut total_deleted = 0;
        for (name, entity) in &config.chats {
            match process_chat(&client, entity, name, cli.limit, cli.hours, cli.dry_run).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        println!("{}: deleted {} messages", name, deleted);
                    }
                    total_deleted += deleted;
                }
                Err(e) => {
                    eprintln!("Error processing {}: {}", name, e);
                }
            }
        }
        println!("\n=== Total deleted: {} messages ===", total_deleted);
    } else {
        // Resolve chat target from args or config
        let (chat_name, chat_entity) = resolve_chat_arg(&cli, &config)?;

        let deleted = process_chat(
            &client,
            &chat_entity,
            &chat_name,
            cli.limit,
            cli.hours,
            cli.dry_run,
        )
        .await?;
        println!("\nDeleted {} messages", deleted);
    }

    Ok(())
}

async fn process_chat(
    client: &grammers_client::Client,
    chat_entity: &ChatEntity,
    chat_name: &str,
    limit: usize,
    min_age_hours: i64,
    dry_run: bool,
) -> Result<usize> {
    let peer = resolve_chat(client, chat_entity).await?;
    let now = Utc::now();
    let min_age = Duration::hours(min_age_hours);

    let mut messages_iter = client.iter_messages(&peer);
    let mut count = 0;
    let mut to_delete: Vec<Message> = Vec::new();
    let mut replied_to: HashSet<i32> = HashSet::new();

    // First pass: collect all messages and track replies
    let mut all_messages: Vec<Message> = Vec::new();

    while let Some(message) = messages_iter.next().await? {
        if count >= limit {
            break;
        }
        count += 1;

        // Track what messages have been replied to
        if let Some(reply_id) = message.reply_to_message_id() {
            replied_to.insert(reply_id);
        }

        all_messages.push(message);
    }

    println!(
        "Checking {} messages in {}...",
        all_messages.len(),
        chat_name
    );

    // Second pass: find outgoing messages without replies and reactions
    for message in &all_messages {
        if !message.outgoing() {
            continue;
        }

        let msg_id = message.id();
        let msg_date = DateTime::from_timestamp(message.date().timestamp(), 0).unwrap_or(now);
        let age = now - msg_date;

        // Skip if message is too recent
        if age < min_age {
            continue;
        }

        // Check if someone replied to this message
        if replied_to.contains(&msg_id) {
            continue;
        }

        // If message is outgoing, old enough, and has no replies, mark for deletion
        to_delete.push(message.clone());
    }

    // Delete messages
    let mut deleted = 0;
    for message in to_delete {
        let msg_date =
            DateTime::from_timestamp(message.date().timestamp(), 0).unwrap_or_else(Utc::now);
        let text_preview = truncate(message.text(), 50);

        if dry_run {
            println!(
                "  WOULD DELETE: {} - {}...",
                msg_date.format("%d.%m.%Y %H:%M"),
                text_preview
            );
        } else {
            println!(
                "  DEL: {} - {}...",
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

fn resolve_chat_arg(cli: &Cli, config: &Config) -> Result<(String, ChatEntity)> {
    if let Some(id) = cli.chat_id {
        return Ok((id.to_string(), ChatEntity::Channel(id)));
    }

    if let Some(name) = &cli.chat {
        if let Ok(id) = name.parse::<i64>() {
            return Ok((name.clone(), ChatEntity::Channel(id)));
        }

        if let Some(entity) = config.chats.get(name) {
            return Ok((name.clone(), entity.clone()));
        }

        return Ok((name.clone(), ChatEntity::Username(name.clone())));
    }

    Err(anyhow::anyhow!(
        "Specify --chat <name> or --chat-id <id>, or use --all"
    ))
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
