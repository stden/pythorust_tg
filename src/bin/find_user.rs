//! Find a user by name in a chat

use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use telegram_reader::chat::resolve_chat;
use telegram_reader::config::{ChatEntity, Config};
use telegram_reader::get_client;
use telegram_reader::session::SessionLock;

#[derive(Parser)]
#[command(name = "find_user")]
#[command(about = "Find a user by name in a Telegram chat")]
struct Cli {
    /// Chat name from config or username
    chat: String,

    /// Name to search for (partial match)
    name: String,

    /// Message limit to search through
    #[arg(short, long, default_value = "1000")]
    limit: usize,
}

#[derive(Default)]
#[allow(dead_code)]
struct UserInfo {
    name: String,
    username: Option<String>,
    message_count: u32,
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

    let search_lower = cli.name.to_lowercase();
    let mut users: HashMap<i64, UserInfo> = HashMap::new();

    let mut messages_iter = client.iter_messages(&peer);
    let mut count = 0;

    while let Some(message) = messages_iter.next().await? {
        if count >= cli.limit {
            break;
        }
        count += 1;

        if let Some(sender) = message.sender() {
            let sender_id: i64 = sender.id().to_string().parse().unwrap_or(0);
            let sender_name = sender.name().unwrap_or("Unknown").to_string();

            // Check if name matches
            if sender_name.to_lowercase().contains(&search_lower) {
                let info = users.entry(sender_id).or_insert_with(|| UserInfo {
                    name: sender_name,
                    username: None, // Would need to check User type
                    message_count: 0,
                });
                info.message_count += 1;
            }
        }
    }

    if users.is_empty() {
        println!("No users found matching '{}'", cli.name);
    } else {
        println!("Found {} users matching '{}':\n", users.len(), cli.name);
        println!("{:<15} {:<30} {:>10}", "User ID", "Name", "Messages");
        println!("{}", "-".repeat(58));

        let mut sorted: Vec<_> = users.into_iter().collect();
        sorted.sort_by(|a, b| b.1.message_count.cmp(&a.1.message_count));

        for (id, info) in sorted {
            println!(
                "{:<15} {:<30} {:>10}",
                id,
                truncate(&info.name, 29),
                info.message_count
            );
        }
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
