//! Vector database integration with Qdrant

use anyhow::Result;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, FieldCondition, Filter, Match, PointStruct,
    SearchPointsBuilder, UpsertPointsBuilder, Value as QdrantValue, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

use super::models::{AnalyzedMessage, SearchResult};

const COLLECTION_NAME: &str = "telegram_messages";

/// Vector store backed by Qdrant
pub struct VectorStore {
    client: Qdrant,
    dimension: usize,
}

impl VectorStore {
    /// Connect to Qdrant server
    pub async fn new(url: &str) -> Result<Self> {
        let client = Qdrant::from_url(url).build()?;

        Ok(Self {
            client,
            dimension: 1536, // text-embedding-3-small dimension
        })
    }

    /// Connect with custom dimension
    pub async fn with_dimension(url: &str, dimension: usize) -> Result<Self> {
        let mut store = Self::new(url).await?;
        store.dimension = dimension;
        Ok(store)
    }

    /// Initialize the collection if it doesn't exist
    pub async fn init_collection(&self) -> Result<()> {
        let collections = self.client.list_collections().await?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == COLLECTION_NAME);

        if !exists {
            info!("Creating collection '{}'", COLLECTION_NAME);

            self.client
                .create_collection(
                    CreateCollectionBuilder::new(COLLECTION_NAME).vectors_config(
                        VectorParamsBuilder::new(self.dimension as u64, Distance::Cosine),
                    ),
                )
                .await?;

            info!("Collection created successfully");
        } else {
            debug!("Collection '{}' already exists", COLLECTION_NAME);
        }

        Ok(())
    }

    /// Upsert messages into the vector store
    pub async fn upsert_messages(&self, messages: &[AnalyzedMessage]) -> Result<usize> {
        let points: Vec<PointStruct> = messages
            .iter()
            .filter_map(|msg| {
                let embedding = msg.embedding.as_ref()?;
                if embedding.is_empty() {
                    return None;
                }

                let mut payload: HashMap<String, QdrantValue> = HashMap::new();
                payload.insert("telegram_id".into(), (msg.telegram_id as i64).into());
                payload.insert("chat_id".into(), msg.chat_id.into());
                payload.insert("chat_name".into(), msg.chat_name.clone().into());
                payload.insert("sender_id".into(), msg.sender_id.into());
                payload.insert("sender_name".into(), msg.sender_name.clone().into());
                payload.insert("text".into(), msg.text.clone().into());
                payload.insert("timestamp".into(), msg.timestamp.to_rfc3339().into());
                payload.insert("reaction_count".into(), (msg.reaction_count as i64).into());
                payload.insert("is_outgoing".into(), msg.is_outgoing.into());

                Some(PointStruct::new(
                    msg.id.to_string(),
                    embedding.clone(),
                    payload,
                ))
            })
            .collect();

        if points.is_empty() {
            return Ok(0);
        }

        let count = points.len();
        debug!("Upserting {} points to Qdrant", count);

        self.client
            .upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, points))
            .await?;

        info!("Successfully upserted {} messages", count);
        Ok(count)
    }

    /// Search for similar messages
    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        limit: u64,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>> {
        let mut search_builder =
            SearchPointsBuilder::new(COLLECTION_NAME, query_embedding, limit).with_payload(true);

        if let Some(f) = filter {
            search_builder = search_builder.filter(f.into_qdrant_filter());
        }

        let results = self.client.search_points(search_builder).await?;

        let search_results: Vec<SearchResult> = results
            .result
            .into_iter()
            .filter_map(|point| {
                let payload = point.payload;
                let score = point.score;

                // Parse payload back to AnalyzedMessage
                let msg = AnalyzedMessage {
                    id: point
                        .id
                        .and_then(|id| {
                            if let qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid_str) =
                                id.point_id_options?
                            {
                                Uuid::parse_str(&uuid_str).ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(Uuid::new_v4),
                    telegram_id: payload.get("telegram_id")?.as_integer()? as i32,
                    chat_id: payload.get("chat_id")?.as_integer()?,
                    chat_name: payload.get("chat_name")?.as_str()?.to_string(),
                    sender_id: payload.get("sender_id")?.as_integer()?,
                    sender_name: payload.get("sender_name")?.as_str()?.to_string(),
                    text: payload.get("text")?.as_str()?.to_string(),
                    timestamp: chrono::DateTime::parse_from_rfc3339(
                        payload.get("timestamp")?.as_str()?,
                    )
                    .ok()?
                    .with_timezone(&chrono::Utc),
                    reply_to_id: payload
                        .get("reply_to_id")
                        .and_then(|v| v.as_integer())
                        .map(|v| v as i32),
                    reaction_count: payload
                        .get("reaction_count")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0) as u32,
                    reactions: payload
                        .get("reactions")
                        .and_then(|v| v.as_list())
                        .map(|list| {
                            list.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    is_outgoing: payload
                        .get("is_outgoing")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    embedding: None, // Don't include embedding in results
                    topics: payload
                        .get("topics")
                        .and_then(|v| v.as_list())
                        .map(|list| {
                            list.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    sentiment: payload
                        .get("sentiment")
                        .and_then(|v| v.as_double())
                        .map(|v| v as f32),
                };

                Some(SearchResult {
                    message: msg,
                    score,
                })
            })
            .collect();

        Ok(search_results)
    }

    /// Delete messages by chat ID
    pub async fn delete_by_chat(&self, chat_id: i64) -> Result<()> {
        use qdrant_client::qdrant::DeletePointsBuilder;

        let filter = Filter::must([FieldCondition {
            key: "chat_id".to_string(),
            r#match: Some(Match {
                match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Integer(chat_id)),
            }),
            ..Default::default()
        }
        .into()]);

        let delete_request = DeletePointsBuilder::new(COLLECTION_NAME).points(filter);

        self.client.delete_points(delete_request).await?;

        info!("Deleted messages for chat {}", chat_id);
        Ok(())
    }

    /// Get collection statistics
    pub async fn stats(&self) -> Result<CollectionStats> {
        let info = self.client.collection_info(COLLECTION_NAME).await?;

        Ok(CollectionStats {
            points_count: info
                .result
                .map(|r| r.points_count.unwrap_or(0))
                .unwrap_or(0),
            dimension: self.dimension,
        })
    }
}

/// Filter for vector search
#[derive(Debug, Default)]
pub struct SearchFilter {
    pub chat_id: Option<i64>,
    pub sender_id: Option<i64>,
    pub is_outgoing: Option<bool>,
    pub min_reactions: Option<u32>,
}

impl SearchFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chat(mut self, chat_id: i64) -> Self {
        self.chat_id = Some(chat_id);
        self
    }

    pub fn sender(mut self, sender_id: i64) -> Self {
        self.sender_id = Some(sender_id);
        self
    }

    pub fn outgoing(mut self, is_outgoing: bool) -> Self {
        self.is_outgoing = Some(is_outgoing);
        self
    }

    fn into_qdrant_filter(self) -> Filter {
        let mut conditions = Vec::new();

        if let Some(chat_id) = self.chat_id {
            conditions.push(
                FieldCondition {
                    key: "chat_id".to_string(),
                    r#match: Some(Match {
                        match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Integer(
                            chat_id,
                        )),
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        if let Some(sender_id) = self.sender_id {
            conditions.push(
                FieldCondition {
                    key: "sender_id".to_string(),
                    r#match: Some(Match {
                        match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Integer(
                            sender_id,
                        )),
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        if let Some(is_outgoing) = self.is_outgoing {
            conditions.push(
                FieldCondition {
                    key: "is_outgoing".to_string(),
                    r#match: Some(Match {
                        match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Boolean(
                            is_outgoing,
                        )),
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        Filter::must(conditions)
    }
}

/// Collection statistics
#[derive(Debug)]
pub struct CollectionStats {
    pub points_count: u64,
    pub dimension: usize,
}

trait QdrantValueExt {
    fn as_integer(&self) -> Option<i64>;
    fn as_str(&self) -> Option<&str>;
    fn as_bool(&self) -> Option<bool>;
    fn as_double(&self) -> Option<f64>;
    fn as_list(&self) -> Option<&qdrant_client::qdrant::ListValue>;
}

impl QdrantValueExt for QdrantValue {
    fn as_integer(&self) -> Option<i64> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::IntegerValue(v)) => Some(*v),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::StringValue(v)) => Some(v),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::BoolValue(v)) => Some(*v),
            _ => None,
        }
    }

    fn as_double(&self) -> Option<f64> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::DoubleValue(v)) => Some(*v),
            _ => None,
        }
    }

    fn as_list(&self) -> Option<&qdrant_client::qdrant::ListValue> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::ListValue(v)) => Some(v),
            _ => None,
        }
    }
}
