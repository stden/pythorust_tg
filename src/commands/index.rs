//! Index messages to vector and graph databases

use anyhow::Result;
use chrono::{DateTime, Utc};
use grammers_client::types::Message;
use grammers_client::Client;
use tracing::{debug, error, info, warn};

use crate::analysis::{
    embeddings::EmbeddingService,
    graph_db::GraphStore,
    models::{AnalyzedMessage, ChatNode, UserNode},
    vector_db::VectorStore,
};
use crate::chat::resolve_chat;
use crate::config::{ChatEntity, Config};
use crate::session::SessionLock;
use crate::{get_client, KNOWN_SENDERS};

/// Index configuration
pub struct IndexConfig {
    /// Qdrant URL
    pub qdrant_url: String,
    /// Whether to index to vector DB
    pub use_vector_db: bool,
    /// Whether to index to graph DB
    pub use_graph_db: bool,
    /// Message limit per chat
    pub limit: usize,
    /// Generate embeddings
    pub generate_embeddings: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            qdrant_url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6333".to_string()),
            use_vector_db: true,
            use_graph_db: true,
            limit: 1000,
            generate_embeddings: true,
        }
    }
}

/// Index messages from a chat to databases
pub async fn index_chat(
    client: &Client,
    chat_entity: &ChatEntity,
    chat_name: &str,
    config: &IndexConfig,
) -> Result<IndexResult> {
    info!("Indexing chat: {}", chat_name);

    let peer = resolve_chat(client, chat_entity).await?;
    let chat_id: i64 = peer.id().to_string().parse().unwrap_or(0);
    let mut messages_iter = client.iter_messages(&peer);

    // Collect messages
    let mut analyzed_messages = Vec::new();
    let mut count = 0;
    let mut user_stats: std::collections::HashMap<i64, UserStats> =
        std::collections::HashMap::new();

    while let Some(message) = messages_iter.next().await? {
        if count >= config.limit {
            break;
        }

        if let Some(analyzed) = analyze_message(&message, chat_name, chat_id).await {
            // Update user stats
            let stats = user_stats
                .entry(analyzed.sender_id)
                .or_insert_with(|| UserStats {
                    name: analyzed.sender_name.clone(),
                    message_count: 0,
                    reactions_received: 0,
                });
            stats.message_count += 1;
            stats.reactions_received += analyzed.reaction_count;

            analyzed_messages.push(analyzed);
            count += 1;
        }
    }

    info!(
        "Collected {} messages from {}",
        analyzed_messages.len(),
        chat_name
    );

    let mut result = IndexResult {
        chat_name: chat_name.to_string(),
        messages_processed: analyzed_messages.len(),
        embeddings_generated: 0,
        vector_db_indexed: 0,
        graph_db_indexed: 0,
    };

    // Generate embeddings if enabled
    if config.generate_embeddings && !analyzed_messages.is_empty() {
        match generate_embeddings(&mut analyzed_messages).await {
            Ok(count) => {
                result.embeddings_generated = count;
                info!("Generated {} embeddings", count);
            }
            Err(e) => {
                warn!("Failed to generate embeddings: {}", e);
            }
        }
    }

    // Index to vector database
    if config.use_vector_db {
        match index_to_vector_db(&config.qdrant_url, &analyzed_messages).await {
            Ok(count) => {
                result.vector_db_indexed = count;
                info!("Indexed {} messages to Qdrant", count);
            }
            Err(e) => {
                error!("Failed to index to vector DB: {}", e);
            }
        }
    }

    // Index to graph database
    if config.use_graph_db {
        match index_to_graph_db(&analyzed_messages, &user_stats, chat_name).await {
            Ok(count) => {
                result.graph_db_indexed = count;
                info!("Indexed {} messages to Neo4j", count);
            }
            Err(e) => {
                error!("Failed to index to graph DB: {}", e);
            }
        }
    }

    Ok(result)
}

/// Analyze a single message
async fn analyze_message(
    message: &Message,
    chat_name: &str,
    chat_id: i64,
) -> Option<AnalyzedMessage> {
    let text = message.text();
    if text.is_empty() {
        return None; // Skip media-only messages for now
    }

    let sender = message.sender()?;
    let sender_id: i64 = sender.id().to_string().parse().unwrap_or(0);
    let sender_name: String = match KNOWN_SENDERS.get(&sender_id) {
        Some(name) => name.to_string(),
        None => sender.name().unwrap_or("Unknown").to_string(),
    };

    let mut analyzed = AnalyzedMessage::new(
        message.id(),
        chat_id,
        chat_name.to_string(),
        sender_id,
        sender_name,
        text.to_string(),
        DateTime::from_timestamp(message.date().timestamp(), 0).unwrap_or_else(Utc::now),
    );

    analyzed.is_outgoing = message.outgoing();

    // Extract reply info
    if let Some(reply) = message.reply_to_message_id() {
        analyzed.reply_to_id = Some(reply);
    }

    // Simple topic extraction (keywords)
    analyzed.topics = extract_topics(text);

    // Simple sentiment (positive/negative keywords)
    analyzed.sentiment = Some(estimate_sentiment(text));

    Some(analyzed)
}

/// Extract topics from text (simple keyword extraction)
fn extract_topics(text: &str) -> Vec<String> {
    let mut topics = Vec::new();
    let text_lower = text.to_lowercase();

    let topic_keywords = [
        ("работа", "work"),
        ("деньги", "money"),
        ("здоровье", "health"),
        ("семья", "family"),
        ("путешеств", "travel"),
        ("еда", "food"),
        ("погода", "weather"),
        ("политик", "politics"),
        ("технолог", "tech"),
        ("ai", "ai"),
        ("программ", "programming"),
        ("встреч", "meetup"),
        ("тусовк", "party"),
    ];

    for (ru, en) in topic_keywords {
        if text_lower.contains(ru) {
            topics.push(en.to_string());
        }
    }

    topics
}

/// Simple sentiment estimation
fn estimate_sentiment(text: &str) -> f32 {
    let text_lower = text.to_lowercase();

    let positive = [
        "хорошо",
        "отлично",
        "супер",
        "круто",
        "спасибо",
        "люблю",
        "рад",
        "счаст",
        "❤️",
        "👍",
        "🔥",
        "😊",
        "😍",
    ];
    let negative = [
        "плохо",
        "ужас",
        "грустно",
        "жаль",
        "злой",
        "ненавиж",
        "проблем",
        "😢",
        "😭",
        "👎",
        "😠",
    ];

    let mut score = 0.0f32;

    for word in positive {
        if text_lower.contains(word) {
            score += 0.2;
        }
    }

    for word in negative {
        if text_lower.contains(word) {
            score -= 0.2;
        }
    }

    score.clamp(-1.0, 1.0)
}

/// Generate embeddings for messages
async fn generate_embeddings(messages: &mut [AnalyzedMessage]) -> Result<usize> {
    let embedding_service = EmbeddingService::new()?;

    // Batch texts for embedding
    let texts: Vec<String> = messages.iter().map(|m| m.text.clone()).collect();

    // Process in batches of 100
    let mut count = 0;
    for (chunk_idx, chunk) in texts.chunks(100).enumerate() {
        debug!("Processing embedding batch {}", chunk_idx + 1);

        let embeddings = embedding_service.embed_batch(chunk).await?;

        for (i, embedding) in embeddings.into_iter().enumerate() {
            let msg_idx = chunk_idx * 100 + i;
            if msg_idx < messages.len() && !embedding.is_empty() {
                messages[msg_idx].embedding = Some(embedding);
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Index messages to Qdrant
async fn index_to_vector_db(url: &str, messages: &[AnalyzedMessage]) -> Result<usize> {
    let store = VectorStore::new(url).await?;
    store.init_collection().await?;
    store.upsert_messages(messages).await
}

/// Index messages to Neo4j
async fn index_to_graph_db(
    messages: &[AnalyzedMessage],
    user_stats: &std::collections::HashMap<i64, UserStats>,
    chat_name: &str,
) -> Result<usize> {
    let store = GraphStore::from_env().await?;
    store.init_schema().await?;

    // Upsert chat
    let chat = ChatNode {
        chat_id: messages.first().map(|m| m.chat_id).unwrap_or(0),
        name: chat_name.to_string(),
        chat_type: "group".to_string(),
        message_count: messages.len() as u32,
        participant_count: user_stats.len() as u32,
    };
    store.upsert_chat(&chat).await?;

    // Upsert users
    for (user_id, stats) in user_stats {
        let user = UserNode {
            user_id: *user_id,
            name: stats.name.clone(),
            username: None,
            message_count: stats.message_count,
            reactions_received: stats.reactions_received,
            avg_sentiment: None,
            active_chats: vec![chat.chat_id],
        };
        store.upsert_user(&user).await?;
    }

    // Upsert messages
    store.upsert_messages(messages).await
}

struct UserStats {
    name: String,
    message_count: u32,
    reactions_received: u32,
}

/// Result of indexing operation
#[derive(Debug)]
pub struct IndexResult {
    pub chat_name: String,
    pub messages_processed: usize,
    pub embeddings_generated: usize,
    pub vector_db_indexed: usize,
    pub graph_db_indexed: usize,
}

/// Index all configured chats
pub async fn index_all_chats(config: &IndexConfig) -> Result<Vec<IndexResult>> {
    let app_config = Config::new();
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let mut results = Vec::new();

    for (name, entity) in &app_config.chats {
        match index_chat(&client, entity, name, config).await {
            Ok(result) => {
                info!("Successfully indexed {}: {:?}", name, result);
                results.push(result);
            }
            Err(e) => {
                error!("Failed to index {}: {}", name, e);
            }
        }
    }

    Ok(results)
}

/// Search similar messages using vector DB
pub async fn search_similar(query: &str, limit: u64) -> Result<Vec<AnalyzedMessage>> {
    let embedding_service = EmbeddingService::new()?;
    let query_embedding = embedding_service.embed(query).await?;

    let qdrant_url =
        std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());

    let store = VectorStore::new(&qdrant_url).await?;
    let results = store.search(query_embedding, limit, None).await?;

    Ok(results.into_iter().map(|r| r.message).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_config_default() {
        let config = IndexConfig::default();
        assert!(config.use_vector_db);
        assert!(config.use_graph_db);
        assert_eq!(config.limit, 1000);
        assert!(config.generate_embeddings);
    }

    #[test]
    fn test_extract_topics_work() {
        let topics = extract_topics("Сегодня работа была сложной");
        assert!(topics.contains(&"work".to_string()));
    }

    #[test]
    fn test_extract_topics_tech() {
        let topics = extract_topics("Изучаю новые технологии и AI");
        assert!(topics.contains(&"tech".to_string()));
        assert!(topics.contains(&"ai".to_string()));
    }

    #[test]
    fn test_extract_topics_programming() {
        let topics = extract_topics("Написал программу на Rust");
        assert!(topics.contains(&"programming".to_string()));
    }

    #[test]
    fn test_extract_topics_empty() {
        let topics = extract_topics("Просто текст без тем");
        assert!(topics.is_empty());
    }

    #[test]
    fn test_estimate_sentiment_positive() {
        let sentiment = estimate_sentiment("Отлично! Супер круто! 👍");
        assert!(sentiment > 0.0);
    }

    #[test]
    fn test_estimate_sentiment_negative() {
        let sentiment = estimate_sentiment("Ужас, всё плохо 😢");
        assert!(sentiment < 0.0);
    }

    #[test]
    fn test_estimate_sentiment_neutral() {
        let sentiment = estimate_sentiment("Обычный нейтральный текст");
        assert!((sentiment - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_sentiment_clamped() {
        // Many positive words should be clamped to 1.0
        let sentiment = estimate_sentiment(
            "хорошо отлично супер круто спасибо люблю рад счастье ❤️ 👍 🔥 😊 😍",
        );
        assert!(sentiment <= 1.0);
        assert!(sentiment >= -1.0);
    }

    #[test]
    fn test_index_result_debug() {
        let result = IndexResult {
            chat_name: "Test".to_string(),
            messages_processed: 100,
            embeddings_generated: 50,
            vector_db_indexed: 100,
            graph_db_indexed: 100,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("Test"));
        assert!(debug.contains("100"));
    }
}
