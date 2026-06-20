//! Bulk reactions helper (send the same emoji to multiple messages).
//!
//! Supports direct message ids, t.me links, ids from file, or the latest N messages.

use std::path::Path;
use std::time::Duration;

use grammers_client::types::peer::Peer;
use grammers_client::Client;
use grammers_tl_types as tl;
use tokio::time::sleep;
use tracing::warn;

use crate::chat::find_chat;
use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

/// Arguments for bulk reactions.
pub struct ReactArgs {
    pub chat: String,
    pub emoji: String,
    pub ids: Vec<String>,
    pub file: Option<std::path::PathBuf>,
    pub recent: usize,
    pub user_id: Option<i64>,
    pub delay_ms: u64,
    pub dry_run: bool,
}

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

/// Extract a message id from a numeric string or t.me link.
fn parse_message_token(token: &str) -> Option<i32> {
    let cleaned = token.trim().trim_end_matches('/');
    if cleaned.is_empty() {
        return None;
    }

    // If it's purely digits, parse directly.
    if cleaned.chars().all(|c| c.is_ascii_digit()) {
        return cleaned.parse::<i32>().ok();
    }

    // Try to extract the trailing numeric segment from a URL.
    let last_segment = cleaned
        .rsplit('/')
        .next()
        .unwrap_or(cleaned)
        .split('?')
        .next()
        .unwrap_or(cleaned);
    if last_segment.chars().all(|c| c.is_ascii_digit()) {
        return last_segment.parse::<i32>().ok();
    }

    None
}

/// Collect message ids from CLI args and optional file.
fn collect_message_ids(ids: &[String], file: Option<&Path>) -> Result<Vec<i32>> {
    let mut tokens: Vec<String> = Vec::new();
    for raw in ids {
        for part in raw
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
        {
            tokens.push(part.to_string());
        }
    }

    if let Some(path) = file {
        let content =
            std::fs::read_to_string(path).map_err(|e| Error::InvalidArgument(e.to_string()))?;
        for line in content.lines() {
            for part in line
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
            {
                tokens.push(part.to_string());
            }
        }
    }

    let mut ids: Vec<i32> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for token in tokens {
        if let Some(id) = parse_message_token(&token) {
            if seen.insert(id) {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

/// Fetch the latest N message ids, optionally filtered by sender id.
async fn fetch_recent_ids(
    client: &Client,
    chat: &Peer,
    limit: usize,
    user_id: Option<i64>,
) -> Result<Vec<i32>> {
    let mut result = Vec::new();
    let mut iter = client.iter_messages(chat);

    while let Some(msg) = iter.next().await.transpose() {
        let msg = msg.map_err(|e| Error::TelegramError(e.to_string()))?;
        if result.len() >= limit {
            break;
        }

        if let Some(expected) = user_id {
            let sender_id = match msg.sender() {
                Some(Peer::User(u)) => u.raw.id(),
                Some(Peer::Channel(c)) => c.raw.id,
                Some(Peer::Group(g)) => match &g.raw {
                    tl::enums::Chat::Chat(ch) => ch.id,
                    tl::enums::Chat::Channel(ch) => ch.id,
                    tl::enums::Chat::Forbidden(ch) => ch.id,
                    tl::enums::Chat::ChannelForbidden(ch) => ch.id,
                    tl::enums::Chat::Empty(ch) => ch.id,
                },
                None => 0,
            };
            if sender_id != expected {
                continue;
            }
        }

        result.push(msg.id());
    }

    Ok(result)
}

/// Send reaction to a single message id.
async fn send_reaction(
    client: &Client,
    peer: &tl::enums::InputPeer,
    msg_id: i32,
    emoji: &str,
) -> Result<()> {
    let request = tl::functions::messages::SendReaction {
        peer: peer.clone(),
        msg_id,
        reaction: Some(vec![tl::enums::Reaction::Emoji(tl::types::ReactionEmoji {
            emoticon: emoji.to_string(),
        })]),
        big: false,
        add_to_recent: false,
    };

    client
        .invoke(&request)
        .await
        .map_err(|e| Error::TelegramError(e.to_string()))?;

    Ok(())
}

/// Execute bulk reactions.
pub async fn run(args: ReactArgs) -> Result<()> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let chat = find_chat(&client, &args.chat).await?;
    let input_peer = peer_to_input(&chat);

    let mut ids = collect_message_ids(&args.ids, args.file.as_deref())?;
    if args.recent > 0 {
        let recent_ids = fetch_recent_ids(&client, &chat, args.recent, args.user_id).await?;
        ids.extend(recent_ids);
    }

    // Deduplicate while preserving order.
    let mut unique: Vec<i32> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if seen.insert(id) {
            unique.push(id);
        }
    }

    if unique.is_empty() {
        return Err(Error::InvalidArgument(
            "No valid message ids provided.".to_string(),
        ));
    }

    println!("Chat: {}", args.chat);
    println!("Emoji: {}", args.emoji);
    println!("Messages to react: {}", unique.len());
    if args.dry_run {
        println!("Dry run: reactions will not be sent.");
    }
    println!();

    let mut sent = 0usize;
    let mut errors = 0usize;

    for msg_id in unique {
        println!("{} -> {}", args.emoji, msg_id);

        if args.dry_run {
            continue;
        }

        match send_reaction(&client, &input_peer, msg_id, &args.emoji).await {
            Ok(()) => {
                sent += 1;
                sleep(Duration::from_millis(args.delay_ms)).await;
            }
            Err(e) => {
                errors += 1;
                let err_str = e.to_string();
                warn!("Error on {}: {}", msg_id, err_str);
                if err_str.contains("FLOOD") || err_str.contains("wait") {
                    warn!("Rate limit detected, stopping early.");
                    break;
                }
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
    fn parse_plain_number() {
        assert_eq!(parse_message_token("12345"), Some(12345));
    }

    #[test]
    fn parse_link_with_query() {
        assert_eq!(
            parse_message_token("https://t.me/channel/12345?single"),
            Some(12345)
        );
    }

    #[test]
    fn parse_link_without_query() {
        assert_eq!(parse_message_token("https://t.me/c/123/999"), Some(999));
    }

    #[test]
    fn parse_invalid() {
        assert_eq!(parse_message_token(""), None);
        assert_eq!(parse_message_token("abc"), None);
    }

    #[test]
    fn collect_ids_dedupes() {
        let ids = collect_message_ids(
            &[
                "123,124".to_string(),
                "https://t.me/c/1/123".to_string(),
                "125".to_string(),
            ],
            None,
        )
        .unwrap();
        assert_eq!(ids, vec![123, 124, 125]);
    }
}
