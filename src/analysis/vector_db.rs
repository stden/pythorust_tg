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

#[cfg(test)]
mod tests {
    use super::*;
    use qdrant_client::qdrant::condition::ConditionOneOf;
    use qdrant_client::qdrant::r#match::MatchValue;
    use qdrant_client::qdrant::value::Kind;
    use qdrant_client::qdrant::{Condition, FieldCondition, ListValue};

    fn extract_field(condition: &Condition) -> &FieldCondition {
        match condition
            .condition_one_of
            .as_ref()
            .expect("condition should exist")
        {
            ConditionOneOf::Field(field) => field,
            _ => panic!("expected field condition"),
        }
    }

    #[test]
    fn search_filter_builds_expected_conditions() {
        let filter = SearchFilter::new()
            .chat(42)
            .sender(7)
            .outgoing(true)
            .into_qdrant_filter();

        assert_eq!(filter.must.len(), 3);

        let mut seen_chat = false;
        let mut seen_sender = false;
        let mut seen_outgoing = false;

        for condition in &filter.must {
            let field = extract_field(condition);
            let match_value = field
                .r#match
                .as_ref()
                .and_then(|m| m.match_value.as_ref())
                .expect("match value");

            match field.key.as_str() {
                "chat_id" => {
                    assert!(matches!(match_value, MatchValue::Integer(v) if *v == 42));
                    seen_chat = true;
                }
                "sender_id" => {
                    assert!(matches!(match_value, MatchValue::Integer(v) if *v == 7));
                    seen_sender = true;
                }
                "is_outgoing" => {
                    assert!(matches!(match_value, MatchValue::Boolean(v) if *v));
                    seen_outgoing = true;
                }
                other => panic!("unexpected field key {}", other),
            }
        }

        assert!(seen_chat && seen_sender && seen_outgoing);
        assert!(filter.should.is_empty());
        assert!(filter.must_not.is_empty());
    }

    #[test]
    fn empty_search_filter_has_no_conditions() {
        let filter = SearchFilter::new().into_qdrant_filter();
        assert!(filter.must.is_empty());
        assert!(filter.should.is_empty());
        assert!(filter.must_not.is_empty());
    }

    #[test]
    fn qdrant_value_helpers_extract_values() {
        let int_value = QdrantValue {
            kind: Some(Kind::IntegerValue(10)),
        };
        assert_eq!(int_value.as_integer(), Some(10));
        assert!(int_value.as_str().is_none());

        let string_value = QdrantValue {
            kind: Some(Kind::StringValue("hello".into())),
        };
        assert_eq!(string_value.as_str().map(String::as_str), Some("hello"));
        assert!(string_value.as_bool().is_none());

        let bool_value = QdrantValue {
            kind: Some(Kind::BoolValue(true)),
        };
        assert_eq!(bool_value.as_bool(), Some(true));

        let double_value = QdrantValue {
            kind: Some(Kind::DoubleValue(1.5)),
        };
        assert_eq!(double_value.as_double(), Some(1.5));
        assert!(double_value.as_integer().is_none());

        let list_value = QdrantValue {
            kind: Some(Kind::ListValue(ListValue {
                values: vec![
                    QdrantValue {
                        kind: Some(Kind::StringValue("a".into())),
                    },
                    QdrantValue {
                        kind: Some(Kind::IntegerValue(2)),
                    },
                ],
            })),
        };
        let list = list_value.as_list().expect("list value");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].as_str().map(String::as_str), Some("a"));
        assert_eq!(list[1].as_integer(), Some(2));
    }

    #[test]
    fn search_filter_default_is_empty() {
        let filter = SearchFilter::default();
        assert!(filter.chat_id.is_none());
        assert!(filter.sender_id.is_none());
        assert!(filter.is_outgoing.is_none());
        assert!(filter.min_reactions.is_none());
    }

    #[test]
    fn search_filter_new_equals_default() {
        let new_filter = SearchFilter::new();
        let default_filter = SearchFilter::default();
        
        assert_eq!(new_filter.chat_id, default_filter.chat_id);
        assert_eq!(new_filter.sender_id, default_filter.sender_id);
    }

    #[test]
    fn search_filter_chat_builder() {
        let filter = SearchFilter::new().chat(12345);
        assert_eq!(filter.chat_id, Some(12345));
    }

    #[test]
    fn search_filter_sender_builder() {
        let filter = SearchFilter::new().sender(67890);
        assert_eq!(filter.sender_id, Some(67890));
    }

    #[test]
    fn search_filter_outgoing_builder() {
        let filter = SearchFilter::new().outgoing(true);
        assert_eq!(filter.is_outgoing, Some(true));
        
        let filter2 = SearchFilter::new().outgoing(false);
        assert_eq!(filter2.is_outgoing, Some(false));
    }

    #[test]
    fn search_filter_chaining() {
        let filter = SearchFilter::new()
            .chat(100)
            .sender(200)
            .outgoing(true);
        
        assert_eq!(filter.chat_id, Some(100));
        assert_eq!(filter.sender_id, Some(200));
        assert_eq!(filter.is_outgoing, Some(true));
    }

    #[test]
    fn search_filter_debug() {
        let filter = SearchFilter::new().chat(42);
        let debug_str = format!("{:?}", filter);
        
        assert!(debug_str.contains("SearchFilter"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn collection_stats_debug() {
        let stats = CollectionStats {
            points_count: 1000,
            dimension: 1536,
        };
        
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("CollectionStats"));
        assert!(debug_str.contains("1000"));
        assert!(debug_str.contains("1536"));
    }

    #[test]
    fn collection_stats_fields() {
        let stats = CollectionStats {
            points_count: 5000,
            dimension: 768,
        };
        
        assert_eq!(stats.points_count, 5000);
        assert_eq!(stats.dimension, 768);
    }

    #[test]
    fn collection_name_constant() {
        assert_eq!(COLLECTION_NAME, "telegram_messages");
    }

    #[test]
    fn single_condition_filter() {
        let filter = SearchFilter::new().chat(99).into_qdrant_filter();
        
        assert_eq!(filter.must.len(), 1);
        let field = extract_field(&filter.must[0]);
        assert_eq!(field.key, "chat_id");
    }

    #[test]
    fn search_filter_sender_only() {
        let filter = SearchFilter::new().sender(12345);
        
        assert!(filter.chat_id.is_none());
        assert_eq!(filter.sender_id, Some(12345));
        assert!(filter.is_outgoing.is_none());
    }

    #[test]
    fn search_filter_outgoing_only() {
        let filter = SearchFilter::new().outgoing(false);
        
        assert!(filter.chat_id.is_none());
        assert!(filter.sender_id.is_none());
        assert_eq!(filter.is_outgoing, Some(false));
    }

    #[test]
    fn search_filter_into_qdrant_filter_sender() {
        let filter = SearchFilter::new().sender(999).into_qdrant_filter();
        
        assert_eq!(filter.must.len(), 1);
        let field = extract_field(&filter.must[0]);
        assert_eq!(field.key, "sender_id");
    }

    #[test]
    fn search_filter_into_qdrant_filter_outgoing() {
        let filter = SearchFilter::new().outgoing(true).into_qdrant_filter();
        
        assert_eq!(filter.must.len(), 1);
        let field = extract_field(&filter.must[0]);
        assert_eq!(field.key, "is_outgoing");
    }

    #[test]
    fn search_filter_three_conditions() {
        let filter = SearchFilter::new()
            .chat(1)
            .sender(2)
            .outgoing(true)
            .into_qdrant_filter();
        
        assert_eq!(filter.must.len(), 3);
    }

    #[test]
    fn collection_stats_zero_values() {
        let stats = CollectionStats {
            points_count: 0,
            dimension: 0,
        };
        
        assert_eq!(stats.points_count, 0);
        assert_eq!(stats.dimension, 0);
    }

    #[test]
    fn collection_stats_large_points() {
        let stats = CollectionStats {
            points_count: u64::MAX,
            dimension: 4096,
        };
        
        assert_eq!(stats.points_count, u64::MAX);
        assert_eq!(stats.dimension, 4096);
    }

    #[test]
    fn search_filter_negative_chat_id() {
        // Telegram can have negative chat IDs for groups
        let filter = SearchFilter::new().chat(-1234567890);
        
        assert_eq!(filter.chat_id, Some(-1234567890));
        
        let qdrant_filter = filter.into_qdrant_filter();
        assert_eq!(qdrant_filter.must.len(), 1);
    }

    #[test]
    fn collection_stats_typical_values() {
        let stats = CollectionStats {
            points_count: 100000,
            dimension: 1536,
        };
        
        assert!(stats.points_count > 0);
        assert!(stats.dimension == 1536 || stats.dimension == 768 || stats.dimension == 3072);
    }

    #[test]
    fn search_filter_chain_override() {
        let filter = SearchFilter::new()
            .chat(1)
            .chat(2)
            .chat(3);
        
        // Last value should be used
        assert_eq!(filter.chat_id, Some(3));
    }

    #[test]
    fn search_filter_empty_generates_empty_must() {
        let filter = SearchFilter::new().into_qdrant_filter();
        
        assert!(filter.must.is_empty());
    }
}

