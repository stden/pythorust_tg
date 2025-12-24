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
    use serde_json::json;

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

    #[test]
    fn analyzed_message_default_has_nil_uuid() {
        let msg = AnalyzedMessage::default();
        assert!(msg.id.is_nil());
        assert_eq!(msg.telegram_id, 0);
        assert_eq!(msg.chat_id, 0);
        assert!(msg.chat_name.is_empty());
        assert_eq!(msg.sender_id, 0);
        assert!(msg.sender_name.is_empty());
        assert!(msg.text.is_empty());
        assert!(msg.reactions.is_empty());
        assert!(!msg.is_outgoing);
    }

    #[test]
    fn analyzed_message_clone() {
        let ts = chrono::Utc::now();
        let msg = AnalyzedMessage::new(
            1,
            2,
            "test".to_string(),
            3,
            "Bob".to_string(),
            "Hello".to_string(),
            ts,
        );
        let cloned = msg.clone();
        
        assert_eq!(cloned.telegram_id, msg.telegram_id);
        assert_eq!(cloned.chat_id, msg.chat_id);
        assert_eq!(cloned.sender_name, msg.sender_name);
        assert_eq!(cloned.id, msg.id);
    }

    #[test]
    fn analyzed_message_debug() {
        let msg = AnalyzedMessage::default();
        let debug_str = format!("{:?}", msg);
        
        assert!(debug_str.contains("AnalyzedMessage"));
        assert!(debug_str.contains("telegram_id"));
    }

    #[test]
    fn analyzed_message_serialization() {
        let ts = chrono::Utc::now();
        let mut msg = AnalyzedMessage::new(
            42,
            100,
            "test_chat".to_string(),
            200,
            "Sender".to_string(),
            "Test message".to_string(),
            ts,
        );
        msg.reaction_count = 5;
        msg.is_outgoing = true;
        msg.topics = vec!["topic1".to_string(), "topic2".to_string()];
        msg.sentiment = Some(0.8);
        
        let json = serde_json::to_string(&msg).unwrap();
        
        assert!(json.contains("\"telegram_id\":42"));
        assert!(json.contains("\"chat_id\":100"));
        assert!(json.contains("\"is_outgoing\":true"));
        assert!(json.contains("\"reaction_count\":5"));
    }

    #[test]
    fn analyzed_message_deserialization() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000000",
            "telegram_id": 123,
            "chat_id": 456,
            "chat_name": "Test Chat",
            "sender_id": 789,
            "sender_name": "Tester",
            "text": "Hello!",
            "timestamp": "2024-01-01T00:00:00Z",
            "reaction_count": 3,
            "reactions": ["👍", "❤️"],
            "is_outgoing": false,
            "topics": ["tech"],
            "sentiment": 0.5
        }"#;
        
        let msg: AnalyzedMessage = serde_json::from_str(json).unwrap();
        
        assert_eq!(msg.telegram_id, 123);
        assert_eq!(msg.chat_id, 456);
        assert_eq!(msg.sender_id, 789);
        assert_eq!(msg.reaction_count, 3);
        assert_eq!(msg.reactions.len(), 2);
        assert_eq!(msg.sentiment, Some(0.5));
    }

    #[test]
    fn analyzed_message_embedding_skipped_when_none() {
        let msg = AnalyzedMessage::default();
        let json = serde_json::to_string(&msg).unwrap();
        
        // embedding should not appear in JSON when None
        assert!(!json.contains("embedding"));
    }

    #[test]
    fn analyzed_message_with_embedding() {
        let mut msg = AnalyzedMessage::default();
        msg.embedding = Some(vec![0.1, 0.2, 0.3]);
        
        let json = serde_json::to_string(&msg).unwrap();
        
        assert!(json.contains("embedding"));
        assert!(json.contains("0.1"));
    }

    #[test]
    fn message_relation_creation() {
        let relation = MessageRelation {
            from_id: "msg1".to_string(),
            to_id: "msg2".to_string(),
            relation_type: RelationType::RepliesTo,
            weight: 1.0,
            properties: json!({"key": "value"}),
        };
        
        assert_eq!(relation.from_id, "msg1");
        assert_eq!(relation.to_id, "msg2");
        assert_eq!(relation.relation_type, RelationType::RepliesTo);
        assert_eq!(relation.weight, 1.0);
    }

    #[test]
    fn message_relation_clone() {
        let relation = MessageRelation {
            from_id: "a".to_string(),
            to_id: "b".to_string(),
            relation_type: RelationType::SentBy,
            weight: 0.5,
            properties: json!({}),
        };
        
        let cloned = relation.clone();
        
        assert_eq!(cloned.from_id, relation.from_id);
        assert_eq!(cloned.relation_type, relation.relation_type);
    }

    #[test]
    fn message_relation_debug() {
        let relation = MessageRelation {
            from_id: "x".to_string(),
            to_id: "y".to_string(),
            relation_type: RelationType::ReactedTo,
            weight: 0.75,
            properties: json!(null),
        };
        
        let debug_str = format!("{:?}", relation);
        
        assert!(debug_str.contains("MessageRelation"));
        assert!(debug_str.contains("ReactedTo"));
    }

    #[test]
    fn message_relation_serialization() {
        let relation = MessageRelation {
            from_id: "from".to_string(),
            to_id: "to".to_string(),
            relation_type: RelationType::Mentions,
            weight: 0.9,
            properties: json!({"count": 5}),
        };
        
        let json = serde_json::to_string(&relation).unwrap();
        
        assert!(json.contains("\"from_id\":\"from\""));
        assert!(json.contains("\"to_id\":\"to\""));
        assert!(json.contains("Mentions"));
    }

    #[test]
    fn relation_type_clone() {
        let rt = RelationType::InteractsWith;
        let cloned = rt.clone();
        
        assert_eq!(cloned, RelationType::InteractsWith);
    }

    #[test]
    fn relation_type_debug() {
        let rt = RelationType::SimilarTo;
        let debug_str = format!("{:?}", rt);
        
        assert!(debug_str.contains("SimilarTo"));
    }

    #[test]
    fn relation_type_partial_eq() {
        assert_eq!(RelationType::InChat, RelationType::InChat);
        assert_ne!(RelationType::InChat, RelationType::RepliesTo);
    }

    #[test]
    fn user_node_creation() {
        let user = UserNode {
            user_id: 12345,
            name: "Test User".to_string(),
            username: Some("testuser".to_string()),
            message_count: 100,
            reactions_received: 50,
            avg_sentiment: Some(0.7),
            active_chats: vec![1, 2, 3],
        };
        
        assert_eq!(user.user_id, 12345);
        assert_eq!(user.name, "Test User");
        assert_eq!(user.username, Some("testuser".to_string()));
        assert_eq!(user.message_count, 100);
        assert_eq!(user.reactions_received, 50);
        assert_eq!(user.avg_sentiment, Some(0.7));
        assert_eq!(user.active_chats.len(), 3);
    }

    #[test]
    fn user_node_clone() {
        let user = UserNode {
            user_id: 1,
            name: "Name".to_string(),
            username: None,
            message_count: 10,
            reactions_received: 5,
            avg_sentiment: None,
            active_chats: vec![],
        };
        
        let cloned = user.clone();
        
        assert_eq!(cloned.user_id, user.user_id);
        assert_eq!(cloned.name, user.name);
    }

    #[test]
    fn user_node_debug() {
        let user = UserNode {
            user_id: 42,
            name: "Debug".to_string(),
            username: Some("dbg".to_string()),
            message_count: 0,
            reactions_received: 0,
            avg_sentiment: None,
            active_chats: vec![],
        };
        
        let debug_str = format!("{:?}", user);
        
        assert!(debug_str.contains("UserNode"));
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("Debug"));
    }

    #[test]
    fn user_node_serialization() {
        let user = UserNode {
            user_id: 999,
            name: "Serialized".to_string(),
            username: Some("serial".to_string()),
            message_count: 200,
            reactions_received: 100,
            avg_sentiment: Some(-0.5),
            active_chats: vec![10, 20],
        };
        
        let json = serde_json::to_string(&user).unwrap();
        
        assert!(json.contains("\"user_id\":999"));
        assert!(json.contains("Serialized"));
        assert!(json.contains("active_chats"));
    }

    #[test]
    fn chat_node_creation() {
        let chat = ChatNode {
            chat_id: 777,
            name: "Test Group".to_string(),
            chat_type: "group".to_string(),
            message_count: 5000,
            participant_count: 50,
        };
        
        assert_eq!(chat.chat_id, 777);
        assert_eq!(chat.name, "Test Group");
        assert_eq!(chat.chat_type, "group");
        assert_eq!(chat.message_count, 5000);
        assert_eq!(chat.participant_count, 50);
    }

    #[test]
    fn chat_node_clone() {
        let chat = ChatNode {
            chat_id: 1,
            name: "Clone".to_string(),
            chat_type: "channel".to_string(),
            message_count: 100,
            participant_count: 10,
        };
        
        let cloned = chat.clone();
        
        assert_eq!(cloned.chat_id, chat.chat_id);
        assert_eq!(cloned.chat_type, chat.chat_type);
    }

    #[test]
    fn chat_node_debug() {
        let chat = ChatNode {
            chat_id: 123,
            name: "Debug Chat".to_string(),
            chat_type: "group".to_string(),
            message_count: 50,
            participant_count: 5,
        };
        
        let debug_str = format!("{:?}", chat);
        
        assert!(debug_str.contains("ChatNode"));
        assert!(debug_str.contains("123"));
    }

    #[test]
    fn chat_node_serialization() {
        let chat = ChatNode {
            chat_id: 555,
            name: "Serialized Chat".to_string(),
            chat_type: "channel".to_string(),
            message_count: 10000,
            participant_count: 1000,
        };
        
        let json = serde_json::to_string(&chat).unwrap();
        
        assert!(json.contains("\"chat_id\":555"));
        assert!(json.contains("channel"));
        assert!(json.contains("10000"));
    }

    #[test]
    fn search_result_creation() {
        let msg = AnalyzedMessage::default();
        let result = SearchResult {
            message: msg.clone(),
            score: 0.95,
        };
        
        assert_eq!(result.score, 0.95);
        assert_eq!(result.message.id, msg.id);
    }

    #[test]
    fn search_result_clone() {
        let msg = AnalyzedMessage::default();
        let result = SearchResult {
            message: msg,
            score: 0.8,
        };
        
        let cloned = result.clone();
        
        assert_eq!(cloned.score, result.score);
    }

    #[test]
    fn search_result_debug() {
        let msg = AnalyzedMessage::default();
        let result = SearchResult {
            message: msg,
            score: 0.77,
        };
        
        let debug_str = format!("{:?}", result);
        
        assert!(debug_str.contains("SearchResult"));
        assert!(debug_str.contains("0.77"));
    }

    #[test]
    fn search_result_serialization() {
        let msg = AnalyzedMessage::default();
        let result = SearchResult {
            message: msg,
            score: 0.99,
        };
        
        let json = serde_json::to_string(&result).unwrap();
        
        assert!(json.contains("score"));
        assert!(json.contains("0.99"));
        assert!(json.contains("message"));
    }

    #[test]
    fn analyzed_message_with_reply() {
        let ts = chrono::Utc::now();
        let mut msg = AnalyzedMessage::new(
            1,
            2,
            "chat".to_string(),
            3,
            "sender".to_string(),
            "text".to_string(),
            ts,
        );
        msg.reply_to_id = Some(999);
        
        assert_eq!(msg.reply_to_id, Some(999));
    }

    #[test]
    fn analyzed_message_with_reactions() {
        let ts = chrono::Utc::now();
        let mut msg = AnalyzedMessage::new(
            1,
            2,
            "chat".to_string(),
            3,
            "sender".to_string(),
            "text".to_string(),
            ts,
        );
        msg.reactions = vec!["👍".to_string(), "❤️".to_string(), "🔥".to_string()];
        msg.reaction_count = 30;
        
        assert_eq!(msg.reactions.len(), 3);
        assert_eq!(msg.reaction_count, 30);
    }

    #[test]
    fn analyzed_message_outgoing() {
        let ts = chrono::Utc::now();
        let mut msg = AnalyzedMessage::new(
            1,
            2,
            "chat".to_string(),
            3,
            "sender".to_string(),
            "text".to_string(),
            ts,
        );
        msg.is_outgoing = true;
        
        assert!(msg.is_outgoing);
    }
}
