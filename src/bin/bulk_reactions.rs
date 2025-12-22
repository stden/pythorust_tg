//! Send the same reaction to many messages in a chat.
//!
//! Usage:
//!   cargo run --bin bulk_reactions -- --chat @channel --emoji 🔥 --ids 123 456 789
//!   cargo run --bin bulk_reactions -- --chat @channel --recent 50

use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use grammers_client::types::peer::Peer;
use grammers_tl_types as tl;
use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use telegram_reader::chat::find_chat;
use telegram_reader::get_client;
use telegram_reader::session::SessionLock;
use tokio::time::sleep;
use tracing::error;

/// Convert a Peer to InputPeer for API calls.
fn peer_to_input(peer: &Peer) -> tl::enums::InputPeer {
    match peer {
        Peer::User(user) => {
            let (user_id, access_hash) = match &user.raw {
                tl::enums::User::User(u) => (u.id, u.access_hash.unwrap_or(0)),
                tl::enums::User::Empty(u) => (u.id, 0),
            };
            tl::enums::InputPeer::User(tl::types::InputPeerUser {
                user_id,
                access_hash,
            })
        }
        Peer::Channel(channel) => tl::enums::InputPeer::Channel(tl::types::InputPeerChannel {
            channel_id: channel.raw.id,
            access_hash: channel.raw.access_hash.unwrap_or(0),
        }),
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Chat(c) => {
                tl::enums::InputPeer::Chat(tl::types::InputPeerChat { chat_id: c.id })
            }
            tl::enums::Chat::Channel(c) => {
                tl::enums::InputPeer::Channel(tl::types::InputPeerChannel {
                    channel_id: c.id,
                    access_hash: c.access_hash.unwrap_or(0),
                })
            }
            _ => tl::enums::InputPeer::Empty,
        },
    }
}

#[derive(Parser, Debug)]
#[command(name = "bulk_reactions")]
#[command(about = "Send reactions to many messages in a chat")]
struct Args {
    /// Chat alias from config.yml, @username or numeric id
    #[arg(long, env = "BULK_REACTIONS_CHAT")]
    chat: Option<String>,

    /// Reaction emoji to send
    #[arg(long, env = "BULK_REACTIONS_EMOJI", default_value = "🔥")]
    emoji: String,

    /// Message ids or t.me links (space separated)
    #[arg(long, num_args = 0..)]
    ids: Vec<String>,

    /// Path to file with message ids or t.me links
    #[arg(long)]
    file: Option<PathBuf>,

    /// React to last N messages from chat
    #[arg(long, default_value = "0")]
    recent: usize,

    /// Only react to messages from this sender id (with --recent)
    #[arg(long)]
    user_id: Option<i64>,

    /// Delay in seconds between reactions
    #[arg(long, env = "BULK_REACTIONS_DELAY", default_value = "0.6")]
    delay: f64,

    /// Preview actions without sending reactions
    #[arg(long)]
    dry_run: bool,
}

/// Parse message id from a number or t.me link.
fn parse_message_token(token: &str) -> Option<i32> {
    let cleaned = token.trim();
    if cleaned.is_empty() {
        return None;
    }

    // Try parsing as number
    if let Ok(id) = cleaned.parse::<i32>() {
        return Some(id);
    }

    // Try extracting from t.me link
    let re = Regex::new(r"/(\d+)(?:\?.*)?$").ok()?;
    re.captures(cleaned)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Collect message ids from CLI args and file.
fn collect_message_ids(raw_ids: &[String], file_path: Option<&PathBuf>) -> Vec<i32> {
    let mut tokens: Vec<String> = Vec::new();

    // Parse CLI args (comma/space separated)
    for raw in raw_ids {
        for part in raw.split(|c: char| c == ',' || c.is_whitespace()) {
            let part = part.trim();
            if !part.is_empty() {
                tokens.push(part.to_string());
            }
        }
    }

    // Parse file if provided
    if let Some(path) = file_path {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                for part in line.split(|c: char| c == ',' || c.is_whitespace()) {
                    let part = part.trim();
                    if !part.is_empty() {
                        tokens.push(part.to_string());
                    }
                }
            }
        }
    }

    // Convert to ids
    tokens
        .iter()
        .filter_map(|t| parse_message_token(t))
        .collect()
}

/// Resolve chat target from args/env.
fn resolve_chat_target(chat_arg: Option<&str>) -> Result<String> {
    let chat = chat_arg
        .map(|s| s.to_string())
        .or_else(|| env::var("BULK_REACTIONS_CHAT").ok())
        .or_else(|| env::var("LIKE_CHAT_ID").ok());

    let chat = chat
        .ok_or_else(|| anyhow::anyhow!("Set --chat or BULK_REACTIONS_CHAT/LIKE_CHAT_ID env var"))?;

    Ok(chat.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let chat_target = resolve_chat_target(args.chat.as_deref())?;
    let mut message_ids = collect_message_ids(&args.ids, args.file.as_ref());

    if message_ids.is_empty() && args.recent == 0 {
        anyhow::bail!("Provide message ids/links or set --recent to react to latest messages");
    }

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    // Resolve chat using find_chat (handles config aliases, usernames, numeric ids)
    let peer = find_chat(&client, &chat_target).await?;
    let chat_name = peer.name().unwrap_or("Unknown");

    // Fetch recent messages if requested
    if args.recent > 0 {
        let mut messages_iter = client.iter_messages(&peer);
        let mut count = 0;

        while let Some(message) = messages_iter.next().await? {
            if count >= args.recent {
                break;
            }

            let sender_id = message.sender().map(|s| match s {
                grammers_client::types::peer::Peer::User(u) => u.raw.id(),
                grammers_client::types::peer::Peer::Channel(c) => c.raw.id,
                grammers_client::types::peer::Peer::Group(g) => {
                    // Extract id from group's raw enum
                    match &g.raw {
                        grammers_tl_types::enums::Chat::Empty(c) => c.id,
                        grammers_tl_types::enums::Chat::Chat(c) => c.id,
                        grammers_tl_types::enums::Chat::Forbidden(c) => c.id,
                        grammers_tl_types::enums::Chat::Channel(c) => c.id,
                        grammers_tl_types::enums::Chat::ChannelForbidden(c) => c.id,
                    }
                }
            });

            // Filter by user if specified
            if let Some(target_user) = args.user_id {
                if sender_id != Some(target_user) {
                    continue;
                }
            }

            message_ids.push(message.id());
            count += 1;
        }
    }

    // Deduplicate
    let mut seen = HashSet::new();
    let unique_ids: Vec<i32> = message_ids
        .into_iter()
        .filter(|id| seen.insert(*id))
        .collect();

    if unique_ids.is_empty() {
        println!("Nothing to do: no valid message ids resolved.");
        return Ok(());
    }

    // Build previews
    let mut previews: std::collections::HashMap<i32, String> = std::collections::HashMap::new();
    for &msg_id in &unique_ids {
        // Fetch message for preview
        let mut iter = client.iter_messages(&peer);
        while let Some(message) = iter.next().await? {
            if message.id() == msg_id {
                let text = message.text();
                let preview = if text.is_empty() {
                    "[no text]".to_string()
                } else if text.len() > 80 {
                    format!("{}…", &text[..80].replace('\n', " "))
                } else {
                    text.replace('\n', " ")
                };
                previews.insert(msg_id, preview);
                break;
            }
        }
    }

    println!("Chat: {}", chat_name);
    println!("Emoji: {}", args.emoji);
    println!("Messages to react: {}", unique_ids.len());
    if args.dry_run {
        println!("Dry run: no reactions will be sent.");
    }
    println!();

    let mut sent = 0;
    let mut errors = 0;

    for msg_id in unique_ids {
        let preview = previews
            .get(&msg_id)
            .map(|s| s.as_str())
            .unwrap_or("[message not found]");
        println!("{} -> {}: {}", args.emoji, msg_id, preview);

        if args.dry_run {
            continue;
        }

        // Send reaction using raw API
        let reaction = tl::enums::Reaction::Emoji(tl::types::ReactionEmoji {
            emoticon: args.emoji.clone(),
        });

        let request = tl::functions::messages::SendReaction {
            peer: peer_to_input(&peer),
            msg_id,
            reaction: Some(vec![reaction]),
            big: false,
            add_to_recent: false,
        };

        match client.invoke(&request).await {
            Ok(_) => {
                sent += 1;
                sleep(Duration::from_secs_f64(args.delay)).await;
            }
            Err(e) => {
                errors += 1;
                error!(msg_id = msg_id, error = %e, "Error sending reaction");
            }
        }
    }

    println!();
    println!("Done.");
    println!("Sent: {}", sent);
    println!("Errors: {}", errors);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_token() {
        assert_eq!(parse_message_token("123"), Some(123));
        assert_eq!(parse_message_token("https://t.me/channel/456"), Some(456));
        assert_eq!(parse_message_token("t.me/c/123456/789"), Some(789));
        assert_eq!(parse_message_token(""), None);
        assert_eq!(parse_message_token("abc"), None);
    }
}
