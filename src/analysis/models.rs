//! Data models for message analysis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Analyzed message with embedding
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyzedMessage {
    /// Unique ID for this analyzed message
    pub id: Uuid,
    /// Original Telegram message ID
    pub telegram_id: i32,
    /// Chat ID where the message was sent
    pub chat_id: i64,
    /// Chat name/title
    pub chat_name: String,
    /// Sender user ID
    pub sender_id: i64,
    /// Sender display name
    pub sender_name: String,
    /// Message text content
    pub text: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Reply to message ID (if any)
    pub reply_to_id: Option<i32>,
    /// Reaction count
    pub reaction_count: u32,
    /// Reaction emojis
    pub reactions: Vec<String>,
    /// Whether this is an outgoing message
    pub is_outgoing: bool,
    /// Generated embedding vector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Detected topics/keywords
    pub topics: Vec<String>,
    /// Sentiment score (-1.0 to 1.0)
    pub sentiment: Option<f32>,
}

impl AnalyzedMessage {
    pub fn new(
        telegram_id: i32,
        chat_id: i64,
        chat_name: String,
        sender_id: i64,
        sender_name: String,
        text: String,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            telegram_id,
            chat_id,
            chat_name,
            sender_id,
            sender_name,
            text,
            timestamp,
            reply_to_id: None,
            reaction_count: 0,
            reactions: Vec::new(),
            is_outgoing: false,
            embedding: None,
            topics: Vec::new(),
            sentiment: None,
        }
    }
}

/// Relationship between messages or users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRelation {
    /// Source message/user ID
    pub from_id: String,
    /// Target message/user ID
    pub to_id: String,
    /// Relationship type
    pub relation_type: RelationType,
    /// Relationship strength/weight
    pub weight: f32,
    /// Additional properties
    pub properties: serde_json::Value,
}

/// Types of relationships in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationType {
    /// Message replies to another message
    RepliesTo,
    /// User sent a message
    SentBy,
    /// User reacted to a message
    ReactedTo,
    /// Users frequently interact
    InteractsWith,
    /// Messages are similar (by embedding)
    SimilarTo,
    /// Message mentions a user
    Mentions,
    /// Message is in a chat
    InChat,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::RepliesTo => "REPLIES_TO",
            RelationType::SentBy => "SENT_BY",
            RelationType::ReactedTo => "REACTED_TO",
            RelationType::InteractsWith => "INTERACTS_WITH",
            RelationType::SimilarTo => "SIMILAR_TO",
            RelationType::Mentions => "MENTIONS",
            RelationType::InChat => "IN_CHAT",
        }
    }
}

/// User node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNode {
    /// Telegram user ID
    pub user_id: i64,
    /// Display name
    pub name: String,
    /// Username (if available)
    pub username: Option<String>,
    /// Total messages sent
    pub message_count: u32,
    /// Total reactions received
    pub reactions_received: u32,
    /// Average sentiment of messages
    pub avg_sentiment: Option<f32>,
    /// Most active in chats
    pub active_chats: Vec<i64>,
}

/// Chat node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatNode {
    /// Telegram chat ID
    pub chat_id: i64,
    /// Chat name/title
    pub name: String,
    /// Chat type (channel, group, user)
    pub chat_type: String,
    /// Total messages
    pub message_count: u32,
    /// Active participants
    pub participant_count: u32,
}

/// Search result from vector database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Message data
    pub message: AnalyzedMessage,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_analyzed_message_with_defaults() {
        let ts = chrono::Utc::now();
        let msg = AnalyzedMessage::new(
            10,
            20,
            "chat".to_string(),
            30,
            "Alice".to_string(),
            "Hello world".to_string(),
            ts,
        );

        assert_eq!(msg.telegram_id, 10);
        assert_eq!(msg.chat_id, 20);
        assert_eq!(msg.sender_id, 30);
        assert_eq!(msg.sender_name, "Alice");
        assert_eq!(msg.text, "Hello world");
        assert_eq!(msg.reply_to_id, None);
        assert_eq!(msg.reaction_count, 0);
        assert!(msg.reactions.is_empty());
        assert!(!msg.id.is_nil());
        assert!(!msg.is_outgoing);
        assert!(msg.embedding.is_none());
        assert!(msg.topics.is_empty());
        assert!(msg.sentiment.is_none());
        assert_eq!(msg.timestamp, ts);
    }

    #[test]
    fn relation_type_as_str_matches_expected_values() {
        assert_eq!(RelationType::RepliesTo.as_str(), "REPLIES_TO");
        assert_eq!(RelationType::SentBy.as_str(), "SENT_BY");
        assert_eq!(RelationType::ReactedTo.as_str(), "REACTED_TO");
        assert_eq!(RelationType::InteractsWith.as_str(), "INTERACTS_WITH");
        assert_eq!(RelationType::SimilarTo.as_str(), "SIMILAR_TO");
        assert_eq!(RelationType::Mentions.as_str(), "MENTIONS");
        assert_eq!(RelationType::InChat.as_str(), "IN_CHAT");
    }
}
