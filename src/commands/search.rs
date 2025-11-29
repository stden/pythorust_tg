//! Semantic search in indexed messages

use anyhow::Result;
use tracing::info;

use crate::analysis::{
    embeddings::EmbeddingService,
    graph_db::GraphStore,
    models::AnalyzedMessage,
    vector_db::{SearchFilter, VectorStore},
};

/// Search configuration
pub struct SearchConfig {
    /// Qdrant URL
    pub qdrant_url: String,
    /// Number of results to return
    pub limit: u64,
    /// Filter by chat ID
    pub chat_id: Option<i64>,
    /// Filter by sender ID
    pub sender_id: Option<i64>,
    /// Filter to only outgoing messages
    pub outgoing_only: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            qdrant_url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6333".to_string()),
            limit: 10,
            chat_id: None,
            sender_id: None,
            outgoing_only: false,
        }
    }
}

/// Semantic search for similar messages
pub async fn search_messages(query: &str, config: &SearchConfig) -> Result<Vec<SearchResult>> {
    info!("Searching for: '{}'", query);

    // Generate embedding for query
    let embedding_service = EmbeddingService::new()?;
    let query_embedding = embedding_service.embed(query).await?;

    // Build filter
    let mut filter = SearchFilter::new();
    if let Some(chat_id) = config.chat_id {
        filter = filter.chat(chat_id);
    }
    if let Some(sender_id) = config.sender_id {
        filter = filter.sender(sender_id);
    }
    if config.outgoing_only {
        filter = filter.outgoing(true);
    }

    let filter_option =
        if config.chat_id.is_some() || config.sender_id.is_some() || config.outgoing_only {
            Some(filter)
        } else {
            None
        };

    // Search vector DB
    let store = VectorStore::new(&config.qdrant_url).await?;
    let results = store
        .search(query_embedding, config.limit, filter_option)
        .await?;

    info!("Found {} results", results.len());

    Ok(results
        .into_iter()
        .map(|r| SearchResult {
            message: r.message,
            score: r.score,
        })
        .collect())
}

/// Find conversation context for a message
pub async fn find_conversation(message_uuid: &str, depth: usize) -> Result<Vec<AnalyzedMessage>> {
    info!("Finding conversation for message: {}", message_uuid);

    let graph = GraphStore::from_env().await?;
    let messages = graph.find_conversation_thread(message_uuid, depth).await?;

    info!("Found {} messages in conversation", messages.len());

    Ok(messages)
}

/// Find users who interact most with a given user
pub async fn find_contacts(user_id: i64, limit: usize) -> Result<Vec<ContactResult>> {
    info!("Finding contacts for user: {}", user_id);

    let graph = GraphStore::from_env().await?;
    let users = graph.find_interacting_users(user_id, limit).await?;

    Ok(users
        .into_iter()
        .map(|u| ContactResult {
            user_id: u.user_id,
            name: u.name,
            message_count: u.message_count,
        })
        .collect())
}

/// Search result with score
#[derive(Debug)]
pub struct SearchResult {
    pub message: AnalyzedMessage,
    pub score: f32,
}

/// Contact result
#[derive(Debug)]
pub struct ContactResult {
    pub user_id: i64,
    pub name: String,
    pub message_count: u32,
}

/// Get database statistics
pub async fn get_stats() -> Result<Stats> {
    let qdrant_url =
        std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());

    let vector_store = VectorStore::new(&qdrant_url).await?;
    let vector_stats = vector_store.stats().await?;

    let graph_stats = match GraphStore::from_env().await {
        Ok(graph) => Some(graph.stats().await?),
        Err(_) => None,
    };

    Ok(Stats {
        vector_points: vector_stats.points_count,
        vector_dimension: vector_stats.dimension,
        graph_users: graph_stats.as_ref().map(|s| s.user_count),
        graph_chats: graph_stats.as_ref().map(|s| s.chat_count),
        graph_messages: graph_stats.as_ref().map(|s| s.message_count),
        graph_relations: graph_stats.as_ref().map(|s| s.relation_count),
    })
}

/// Combined statistics
#[derive(Debug)]
pub struct Stats {
    pub vector_points: u64,
    pub vector_dimension: usize,
    pub graph_users: Option<u64>,
    pub graph_chats: Option<u64>,
    pub graph_messages: Option<u64>,
    pub graph_relations: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.limit, 10);
        assert!(config.chat_id.is_none());
        assert!(config.sender_id.is_none());
        assert!(!config.outgoing_only);
    }

    #[test]
    fn test_search_config_custom() {
        let config = SearchConfig {
            qdrant_url: "http://custom:6333".to_string(),
            limit: 50,
            chat_id: Some(123),
            sender_id: Some(456),
            outgoing_only: true,
        };
        assert_eq!(config.qdrant_url, "http://custom:6333");
        assert_eq!(config.limit, 50);
        assert_eq!(config.chat_id, Some(123));
        assert_eq!(config.sender_id, Some(456));
        assert!(config.outgoing_only);
    }

    #[test]
    fn test_search_result_debug() {
        let msg = AnalyzedMessage::default();
        let result = SearchResult {
            message: msg,
            score: 0.95,
        };
        assert!(format!("{:?}", result).contains("0.95"));
    }

    #[test]
    fn test_contact_result_debug() {
        let contact = ContactResult {
            user_id: 123,
            name: "Test User".to_string(),
            message_count: 42,
        };
        assert!(format!("{:?}", contact).contains("123"));
        assert!(format!("{:?}", contact).contains("Test User"));
        assert!(format!("{:?}", contact).contains("42"));
    }

    #[test]
    fn test_stats_debug() {
        let stats = Stats {
            vector_points: 1000,
            vector_dimension: 1536,
            graph_users: Some(100),
            graph_chats: Some(50),
            graph_messages: Some(5000),
            graph_relations: Some(10000),
        };
        assert!(format!("{:?}", stats).contains("1000"));
        assert!(format!("{:?}", stats).contains("1536"));
    }

    #[test]
    fn test_stats_with_none_values() {
        let stats = Stats {
            vector_points: 500,
            vector_dimension: 768,
            graph_users: None,
            graph_chats: None,
            graph_messages: None,
            graph_relations: None,
        };
        assert!(format!("{:?}", stats).contains("None"));
    }
}
