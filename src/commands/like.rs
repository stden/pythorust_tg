//! Like command - send reactions to messages from specific users
//!
//! Usage: telegram_reader like --chat "Хара" --user "Anna Mart" --emoji "❤️"

use std::time::Duration;

use grammers_client::types::peer::Peer;
use grammers_client::types::reactions::InputReactions;
use grammers_client::types::Message;
use grammers_client::Client;
use grammers_tl_types as tl;
use rand::Rng;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::chat::find_chat;
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
            delay_ms: 1500,
        }
    }
}

/// Result of like operation
#[derive(Debug, Default)]
pub struct LikeResult {
    pub liked_count: usize,
    pub skipped_count: usize,
    pub already_reacted_count: usize,
    pub error_count: usize,
    pub messages_scanned: usize,
}

/// Target user matcher: supports numeric IDs, @usernames and name substrings.
#[derive(Debug, Clone)]
enum SenderMatcher {
    Id(i64),
    Username(String),
    NameSubstr(String),
}

impl SenderMatcher {
    fn from_input(input: &str) -> Self {
        let trimmed = input.trim();
        if let Some(username) = trimmed.strip_prefix('@') {
            return Self::Username(username.to_lowercase());
        }

        if let Ok(id) = trimmed.parse::<i64>() {
            return Self::Id(id);
        }

        Self::NameSubstr(trimmed.to_lowercase())
    }

    fn description(&self) -> String {
        match self {
            SenderMatcher::Id(id) => format!("ID {}", id),
            SenderMatcher::Username(username) => format!("@{}", username),
            SenderMatcher::NameSubstr(substr) => substr.clone(),
        }
    }

    fn matches(&self, sender: &SenderInfo) -> bool {
        match self {
            SenderMatcher::Id(id) => sender.id.map(|sid| sid == *id).unwrap_or(false),
            SenderMatcher::Username(username) => sender
                .username
                .as_ref()
                .map(|u| u.to_lowercase() == *username)
                .unwrap_or(false),
            SenderMatcher::NameSubstr(substr) => {
                let display = sender.display_name.to_lowercase();
                let username_match = sender
                    .username
                    .as_ref()
                    .map(|u| u.to_lowercase().contains(substr))
                    .unwrap_or(false);
                display.contains(substr) || username_match
            }
        }
    }
}

/// Extracted sender info used for matching.
#[derive(Debug, Clone)]
struct SenderInfo {
    id: Option<i64>,
    username: Option<String>,
    display_name: String,
}

/// Get title from a Peer
fn get_peer_title(peer: &Peer) -> String {
    match peer {
        Peer::Channel(c) => c.title().to_string(),
        Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
        Peer::User(u) => u.full_name(),
    }
}

/// Get optional username from a Peer
fn get_peer_username(peer: &Peer) -> Option<String> {
    match peer {
        Peer::User(user) => user.username().map(|u| u.to_string()),
        Peer::Channel(channel) => channel.username().map(|u| u.to_string()),
        Peer::Group(group) => group.username().map(|u| u.to_string()),
    }
}

/// Get numeric id from a Peer, if available
fn get_peer_id(peer: &Peer) -> Option<i64> {
    match peer {
        Peer::User(user) => Some(user.raw.id()),
        Peer::Channel(channel) => Some(channel.raw.id),
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Empty(c) => Some(c.id),
            tl::enums::Chat::Chat(c) => Some(c.id),
            tl::enums::Chat::Forbidden(c) => Some(c.id),
            tl::enums::Chat::Channel(c) => Some(c.id),
            tl::enums::Chat::ChannelForbidden(c) => Some(c.id),
        },
    }
}

/// Build a normalized sender info struct
fn extract_sender_info(peer: &Peer) -> SenderInfo {
    SenderInfo {
        id: get_peer_id(peer),
        username: get_peer_username(peer),
        display_name: get_peer_title(peer),
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

/// Check if a reaction matches the target emoji
fn reaction_matches_emoji(reaction: &tl::enums::Reaction, emoji: &str) -> bool {
    match reaction {
        tl::enums::Reaction::Emoji(r) => r.emoticon == emoji,
        _ => false,
    }
}

/// Detect if the current user already reacted with the target emoji
fn reactions_contain_my_emoji(reactions: &tl::types::MessageReactions, emoji: &str) -> bool {
    if let Some(list) = &reactions.recent_reactions {
        for reaction in list {
            let tl::enums::MessagePeerReaction::Reaction(reaction) = reaction;
            if reaction.my && reaction_matches_emoji(&reaction.reaction, emoji) {
                return true;
            }
        }
    }
    false
}

/// Get reactions from a message (if available)
fn message_reactions(message: &Message) -> Option<&tl::types::MessageReactions> {
    match &message.raw {
        tl::enums::Message::Message(m) => match &m.reactions {
            Some(tl::enums::MessageReactions::Reactions(r)) => Some(r),
            _ => None,
        },
        tl::enums::Message::Service(m) => match &m.reactions {
            Some(tl::enums::MessageReactions::Reactions(r)) => Some(r),
            _ => None,
        },
        _ => None,
    }
}

/// Determine whether we already placed the same emoji on this message
fn already_reacted_with_emoji(message: &Message, emoji: &str) -> bool {
    message_reactions(message)
        .map(|reactions| reactions_contain_my_emoji(reactions, emoji))
        .unwrap_or(false)
}

/// Extract flood wait seconds from an error string (best-effort)
fn parse_flood_wait_seconds(error: &str) -> Option<u64> {
    if let Some(idx) = error.find("FLOOD_WAIT_") {
        let start = idx + "FLOOD_WAIT_".len();
        let secs = error[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if let Ok(v) = secs.parse::<u64>() {
            return Some(v);
        }
    }

    if let Some(idx) = error.find("value:") {
        let start = idx + "value:".len();
        let secs = error[start..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if let Ok(v) = secs.parse::<u64>() {
            return Some(v);
        }
    }

    None
}

/// Human-like delay generator to avoid predictable timing
#[derive(Debug)]
struct HumanDelayStrategy {
    base_delay_ms: u64,
    current_delay_ms: u64,
    burst_len: u8,
}

impl HumanDelayStrategy {
    fn new(base_delay_ms: u64) -> Self {
        Self {
            base_delay_ms,
            current_delay_ms: base_delay_ms,
            burst_len: 0,
        }
    }

    fn drift_towards_base(&mut self) {
        if self.current_delay_ms > self.base_delay_ms {
            let diff = self.current_delay_ms - self.base_delay_ms;
            let step = (diff * 15).div_ceil(100); // ceil(diff * 0.15)
            self.current_delay_ms = self.current_delay_ms.saturating_sub(step.max(1));
        }
    }

    fn next_pause_ms(&mut self, rng: &mut impl Rng) -> u64 {
        self.drift_towards_base();
        self.burst_len = self.burst_len.saturating_add(1);

        let jitter_percent: u64 = rng.gen_range(85..=130);
        let micro_offset: u64 = rng.gen_range(90..=320);
        let mut pause_ms = (self.current_delay_ms.saturating_mul(jitter_percent) / 100)
            .saturating_add(micro_offset);

        // Occasionally insert a longer break to mimic a human taking a pause.
        if self.burst_len >= rng.gen_range(3..=7) {
            let long_pause = rng.gen_range(1200..=4500);
            pause_ms = pause_ms.saturating_add(long_pause);
            self.burst_len = 0;
        }

        pause_ms
    }

    fn register_flood_wait(&mut self) {
        self.current_delay_ms = (self.current_delay_ms.saturating_mul(2)).min(10_000);
        self.burst_len = 0;
    }
}

/// Like all messages from a specific user in a chat
pub async fn like_user_messages(
    chat_name: &str,
    user_name: &str,
    config: LikeConfig,
) -> Result<LikeResult> {
    let mut rng = rand::thread_rng();
    let mut delay_strategy = HumanDelayStrategy::new(config.delay_ms);
    let target = SenderMatcher::from_input(user_name);

    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    // Find chat
    let chat = match find_chat(&client, chat_name).await {
        Ok(peer) => peer,
        Err(_) => {
            warn!(
                "Chat '{}' not found via config/username lookup, trying dialog title search",
                chat_name
            );
            find_chat_by_name(&client, chat_name).await?
        }
    };

    let mut result = LikeResult::default();

    info!(
        "Scanning messages in chat, looking for sender '{}'",
        target.description()
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
        if let Some(sender) = msg.sender() {
            let sender_info = extract_sender_info(sender);
            if !target.matches(&sender_info) {
                result.skipped_count += 1;
                continue;
            }

            if already_reacted_with_emoji(&msg, &config.emoji) {
                result.already_reacted_count += 1;
                continue;
            }

            // Try to send reaction
            match client
                .send_reactions(&chat, msg.id(), InputReactions::emoticon(&config.emoji))
                .await
            {
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
                    let delay_ms = delay_strategy.next_pause_ms(&mut rng);
                    sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if let Some(wait_secs) = parse_flood_wait_seconds(&err_str) {
                        let wait = wait_secs + 1;
                        warn!("Rate limit hit, waiting {}s: {}", wait, err_str);
                        sleep(Duration::from_secs(wait)).await;
                        // Increase delay to reduce future flood waits and reset burst counter
                        delay_strategy.register_flood_wait();
                        continue;
                    }
                    warn!("Error on message {}: {}", msg.id(), err_str);
                    result.error_count += 1;
                }
            }
        } else {
            result.skipped_count += 1;
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

    let matcher = SenderMatcher::from_input(user);
    println!("🔍 Ищу чат '{}'...", chat);
    println!("👤 Ищу сообщения от '{}'...", matcher.description());
    println!("💝 Буду ставить реакцию: {}", config.emoji);
    println!();

    let result = like_user_messages(chat, user, config).await?;

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Результат:");
    println!("   Просканировано сообщений: {}", result.messages_scanned);
    println!("   Поставлено лайков: {}", result.liked_count);
    println!(
        "   Пропущено по фильтру отправителя: {}",
        result.skipped_count
    );
    println!("   Уже была наша реакция: {}", result.already_reacted_count);
    println!("   Ошибок: {}", result.error_count);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn test_default_config() {
        let config = LikeConfig::default();
        assert_eq!(config.emoji, "❤️");
        assert_eq!(config.limit, 500);
        assert_eq!(config.delay_ms, 1500);
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
        assert_eq!(result.already_reacted_count, 0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.messages_scanned, 0);
    }

    #[test]
    fn test_like_result_accumulation() {
        let result = LikeResult {
            liked_count: 50,
            messages_scanned: 500,
            error_count: 2,
            already_reacted_count: 3,
            ..Default::default()
        };

        assert_eq!(result.liked_count, 50);
        assert_eq!(result.messages_scanned, 500);
        assert_eq!(result.error_count, 2);
        assert_eq!(result.already_reacted_count, 3);
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

    fn sender_info(id: Option<i64>, username: Option<&str>, display_name: &str) -> SenderInfo {
        SenderInfo {
            id,
            username: username.map(|u| u.to_string()),
            display_name: display_name.to_string(),
        }
    }

    #[test]
    fn matcher_by_id() {
        let matcher = SenderMatcher::from_input("12345");
        let sender = sender_info(Some(12345), Some("user"), "User Name");
        assert!(matcher.matches(&sender));

        let other = sender_info(Some(678), Some("user"), "User Name");
        assert!(!matcher.matches(&other));
    }

    #[test]
    fn matcher_by_username() {
        let matcher = SenderMatcher::from_input("@AnnaMart");
        let sender = sender_info(Some(1), Some("annamart"), "Anna Mart");
        assert!(matcher.matches(&sender));

        let other = sender_info(Some(1), Some("not_match"), "Anna Mart");
        assert!(!matcher.matches(&other));
    }

    #[test]
    fn matcher_by_name_substring() {
        let matcher = SenderMatcher::from_input("anna");
        let sender = sender_info(Some(1), Some("annamart"), "Anna Mart");
        assert!(matcher.matches(&sender));

        let other = sender_info(Some(1), Some("masha"), "Maria Petrova");
        assert!(!matcher.matches(&other));
    }

    fn make_peer_reaction(my: bool, emoji: &str) -> tl::enums::MessagePeerReaction {
        tl::enums::MessagePeerReaction::Reaction(tl::types::MessagePeerReaction {
            big: false,
            unread: false,
            my,
            peer_id: tl::enums::Peer::User(tl::types::PeerUser { user_id: 42 }),
            date: 0,
            reaction: tl::enums::Reaction::Emoji(tl::types::ReactionEmoji {
                emoticon: emoji.to_string(),
            }),
        })
    }

    #[test]
    fn detect_existing_reaction() {
        let reactions = tl::types::MessageReactions {
            min: false,
            can_see_list: true,
            reactions_as_tags: false,
            results: Vec::new(),
            recent_reactions: Some(vec![make_peer_reaction(true, "❤️")]),
            top_reactors: None,
        };

        assert!(reactions_contain_my_emoji(&reactions, "❤️"));
        assert!(!reactions_contain_my_emoji(&reactions, "🔥"));
    }

    #[test]
    fn detect_only_my_reaction_counts() {
        let reactions = tl::types::MessageReactions {
            min: false,
            can_see_list: true,
            reactions_as_tags: false,
            results: Vec::new(),
            recent_reactions: Some(vec![
                make_peer_reaction(false, "❤️"),
                make_peer_reaction(true, "🔥"),
            ]),
            top_reactors: None,
        };

        assert!(reactions_contain_my_emoji(&reactions, "🔥"));
        assert!(!reactions_contain_my_emoji(&reactions, "❤️"));
    }

    #[test]
    fn test_human_delay_jitter_range() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut strategy = HumanDelayStrategy::new(1500);

        let delay1 = strategy.next_pause_ms(&mut rng);
        let delay2 = strategy.next_pause_ms(&mut rng);

        let min_delay = (1500 * 85) / 100 + 90; // jitter + micro offset
        let max_delay = (1500 * 130) / 100 + 320 + 4500; // jitter + micro offset + long pause

        assert!(delay1 >= min_delay && delay1 <= max_delay);
        assert!(delay2 >= min_delay && delay2 <= max_delay);
        assert_ne!(delay1, delay2);
    }

    #[test]
    fn test_human_delay_handles_flood_wait() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut strategy = HumanDelayStrategy::new(1000);

        strategy.register_flood_wait();
        assert_eq!(strategy.current_delay_ms, 2000);

        let delay = strategy.next_pause_ms(&mut rng);
        let min_delay = (2000 * 85) / 100 + 90;
        let max_delay = (2000 * 130) / 100 + 320 + 4500;
        assert!(delay >= min_delay && delay <= max_delay);

        // Drift back toward base after several pauses
        for _ in 0..5 {
            strategy.next_pause_ms(&mut rng);
        }
        assert!(strategy.current_delay_ms < 2000);
        assert!(strategy.current_delay_ms >= 1000);
    }
}
