//! Like command - send reactions to messages from specific users
//!
//! Usage: telegram_reader like --chat "Хара" --user "Anna Mart" --emoji "❤️"

use std::time::Duration;

use grammers_client::types::peer::Peer;
use grammers_client::Client;
use grammers_tl_types as tl;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

/// Configuration for the like command
#[derive(Debug, Clone)]
pub struct LikeConfig {
    /// Emoji to use for reaction (default: ❤️)
    pub emoji: String,
    /// Maximum messages to scan
    pub limit: usize,
    /// Delay between reactions in milliseconds (to avoid rate limits)
    pub delay_ms: u64,
}

impl Default for LikeConfig {
    fn default() -> Self {
        Self {
            emoji: "❤️".to_string(),
            limit: 500,
            delay_ms: 500,
        }
    }
}

/// Result of like operation
#[derive(Debug, Default)]
pub struct LikeResult {
    pub liked_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub messages_scanned: usize,
}

/// Get title from a Peer
fn get_peer_title(peer: &Peer) -> String {
    match peer {
        Peer::Channel(c) => c.title().to_string(),
        Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
        Peer::User(u) => u.full_name(),
    }
}

/// Find chat by name (partial match in title)
async fn find_chat_by_name(client: &Client, name: &str) -> Result<Peer> {
    let name_lower = name.to_lowercase();
    let mut dialogs = client.iter_dialogs();

    while let Some(dialog) = dialogs
        .next()
        .await
        .map_err(|e| Error::TelegramError(e.to_string()))?
    {
        let title = get_peer_title(&dialog.peer).to_lowercase();
        if title.contains(&name_lower) {
            info!(
                "Found chat: {} (searching for '{}')",
                get_peer_title(&dialog.peer),
                name
            );
            return Ok(dialog.peer);
        }
    }

    Err(Error::ChatNotFound(format!(
        "Chat containing '{}' not found",
        name
    )))
}

/// Get sender name from a message
fn get_sender_name(msg: &grammers_client::types::Message) -> Option<String> {
    msg.sender().map(|sender| match sender {
        Peer::User(user) => user.full_name(),
        Peer::Channel(ch) => ch.title().to_string(),
        Peer::Group(g) => g.title().unwrap_or("Unknown").to_string(),
    })
}

/// Convert Peer to InputPeer for API calls
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

/// Send reaction to a message
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

/// Like all messages from a specific user in a chat
pub async fn like_user_messages(
    chat_name: &str,
    user_name: &str,
    config: LikeConfig,
) -> Result<LikeResult> {
    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    // Find chat
    let chat = find_chat_by_name(&client, chat_name).await?;
    let input_peer = peer_to_input(&chat);

    let user_name_lower = user_name.to_lowercase();
    let mut result = LikeResult::default();

    info!(
        "Scanning messages in chat, looking for user '{}'...",
        user_name
    );

    // Iterate messages
    let mut iter = client.iter_messages(&chat);

    while let Some(msg) = iter.next().await.transpose() {
        let msg = msg.map_err(|e| Error::TelegramError(e.to_string()))?;
        result.messages_scanned += 1;

        if result.messages_scanned > config.limit {
            break;
        }

        // Check sender
        if let Some(sender_name) = get_sender_name(&msg) {
            if sender_name.to_lowercase().contains(&user_name_lower) {
                // Try to send reaction
                match send_reaction(&client, &input_peer, msg.id(), &config.emoji).await {
                    Ok(()) => {
                        result.liked_count += 1;
                        let text_preview = msg
                            .text()
                            .chars()
                            .take(50)
                            .collect::<String>()
                            .replace('\n', " ");
                        println!(
                            "{} Лайк на сообщение {}: {}...",
                            config.emoji,
                            msg.id(),
                            text_preview
                        );

                        // Delay to avoid rate limits
                        sleep(Duration::from_millis(config.delay_ms)).await;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        if err_str.contains("wait") || err_str.contains("FLOOD") {
                            warn!("Rate limit hit, stopping: {}", err_str);
                            result.error_count += 1;
                            break;
                        }
                        warn!("Error on message {}: {}", msg.id(), err_str);
                        result.error_count += 1;
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Main entry point for the like command
pub async fn run(chat: &str, user: &str, emoji: Option<&str>, limit: usize) -> Result<()> {
    let config = LikeConfig {
        emoji: emoji.unwrap_or("❤️").to_string(),
        limit,
        ..Default::default()
    };

    println!("🔍 Ищу чат '{}'...", chat);
    println!("👤 Ищу сообщения от '{}'...", user);
    println!("💝 Буду ставить реакцию: {}", config.emoji);
    println!();

    let result = like_user_messages(chat, user, config).await?;

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Результат:");
    println!("   Просканировано сообщений: {}", result.messages_scanned);
    println!("   Поставлено лайков: {}", result.liked_count);
    println!("   Ошибок: {}", result.error_count);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LikeConfig::default();
        assert_eq!(config.emoji, "❤️");
        assert_eq!(config.limit, 500);
        assert_eq!(config.delay_ms, 500);
    }

    #[test]
    fn test_custom_config() {
        let config = LikeConfig {
            emoji: "🔥".to_string(),
            limit: 100,
            delay_ms: 1000,
        };
        assert_eq!(config.emoji, "🔥");
        assert_eq!(config.limit, 100);
        assert_eq!(config.delay_ms, 1000);
    }

    #[test]
    fn test_like_result_default() {
        let result = LikeResult::default();
        assert_eq!(result.liked_count, 0);
        assert_eq!(result.skipped_count, 0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.messages_scanned, 0);
    }

    #[test]
    fn test_like_result_accumulation() {
        let mut result = LikeResult::default();
        result.liked_count = 50;
        result.messages_scanned = 500;
        result.error_count = 2;

        assert_eq!(result.liked_count, 50);
        assert_eq!(result.messages_scanned, 500);
        assert_eq!(result.error_count, 2);
    }

    #[test]
    fn test_config_clone() {
        let config = LikeConfig::default();
        let cloned = config.clone();
        assert_eq!(config.emoji, cloned.emoji);
        assert_eq!(config.limit, cloned.limit);
    }

    #[test]
    fn test_various_emojis() {
        let emojis = vec!["❤️", "👍", "🔥", "😍", "👏", "🎉", "💯", "⭐"];
        for emoji in emojis {
            let config = LikeConfig {
                emoji: emoji.to_string(),
                ..Default::default()
            };
            assert_eq!(config.emoji, emoji);
        }
    }
}
