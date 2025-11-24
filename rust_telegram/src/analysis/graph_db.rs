//! Graph database integration with Neo4j

use anyhow::Result;
use neo4rs::{query, Graph, Node};
use tracing::{debug, info};

use super::models::{AnalyzedMessage, ChatNode, MessageRelation, UserNode};

/// Graph store backed by Neo4j
pub struct GraphStore {
    graph: Graph,
}

impl GraphStore {
    /// Connect to Neo4j server
    pub async fn new(uri: &str, user: &str, password: &str) -> Result<Self> {
        let graph = Graph::new(uri, user, password).await?;

        Ok(Self { graph })
    }

    /// Connect using environment variables
    pub async fn from_env() -> Result<Self> {
        let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
        let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let password = std::env::var("NEO4J_PASSWORD")
            .map_err(|_| anyhow::anyhow!("NEO4J_PASSWORD not set"))?;

        Self::new(&uri, &user, &password).await
    }

    /// Initialize schema with constraints and indexes
    pub async fn init_schema(&self) -> Result<()> {
        info!("Initializing Neo4j schema...");

        // Create constraints for unique IDs
        let constraints = [
            "CREATE CONSTRAINT user_id IF NOT EXISTS FOR (u:User) REQUIRE u.user_id IS UNIQUE",
            "CREATE CONSTRAINT chat_id IF NOT EXISTS FOR (c:Chat) REQUIRE c.chat_id IS UNIQUE",
            "CREATE CONSTRAINT message_id IF NOT EXISTS FOR (m:Message) REQUIRE m.uuid IS UNIQUE",
        ];

        for constraint in constraints {
            self.graph.run(query(constraint)).await?;
        }

        // Create indexes for common queries
        let indexes = [
            "CREATE INDEX user_name IF NOT EXISTS FOR (u:User) ON (u.name)",
            "CREATE INDEX chat_name IF NOT EXISTS FOR (c:Chat) ON (c.name)",
            "CREATE INDEX message_timestamp IF NOT EXISTS FOR (m:Message) ON (m.timestamp)",
            "CREATE INDEX message_telegram_id IF NOT EXISTS FOR (m:Message) ON (m.telegram_id)",
        ];

        for index in indexes {
            self.graph.run(query(index)).await?;
        }

        info!("Schema initialized successfully");
        Ok(())
    }

    /// Create or update a user node
    pub async fn upsert_user(&self, user: &UserNode) -> Result<()> {
        let q = query(
            "MERGE (u:User {user_id: $user_id})
             SET u.name = $name,
                 u.username = $username,
                 u.message_count = $message_count,
                 u.reactions_received = $reactions_received,
                 u.avg_sentiment = $avg_sentiment,
                 u.updated_at = datetime()",
        )
        .param("user_id", user.user_id)
        .param("name", user.name.clone())
        .param("username", user.username.clone())
        .param("message_count", user.message_count as i64)
        .param("reactions_received", user.reactions_received as i64)
        .param("avg_sentiment", user.avg_sentiment);

        self.graph.run(q).await?;
        debug!("Upserted user: {} ({})", user.name, user.user_id);
        Ok(())
    }

    /// Create or update a chat node
    pub async fn upsert_chat(&self, chat: &ChatNode) -> Result<()> {
        let q = query(
            "MERGE (c:Chat {chat_id: $chat_id})
             SET c.name = $name,
                 c.chat_type = $chat_type,
                 c.message_count = $message_count,
                 c.participant_count = $participant_count,
                 c.updated_at = datetime()",
        )
        .param("chat_id", chat.chat_id)
        .param("name", chat.name.clone())
        .param("chat_type", chat.chat_type.clone())
        .param("message_count", chat.message_count as i64)
        .param("participant_count", chat.participant_count as i64);

        self.graph.run(q).await?;
        debug!("Upserted chat: {} ({})", chat.name, chat.chat_id);
        Ok(())
    }

    /// Create or update a message node with all relationships
    pub async fn upsert_message(&self, msg: &AnalyzedMessage) -> Result<()> {
        // Create message node
        let q = query(
            "MERGE (m:Message {uuid: $uuid})
             SET m.telegram_id = $telegram_id,
                 m.text = $text,
                 m.timestamp = datetime($timestamp),
                 m.reaction_count = $reaction_count,
                 m.reactions = $reactions,
                 m.is_outgoing = $is_outgoing,
                 m.topics = $topics,
                 m.sentiment = $sentiment",
        )
        .param("uuid", msg.id.to_string())
        .param("telegram_id", msg.telegram_id)
        .param("text", msg.text.clone())
        .param("timestamp", msg.timestamp.to_rfc3339())
        .param("reaction_count", msg.reaction_count as i64)
        .param("reactions", msg.reactions.clone())
        .param("is_outgoing", msg.is_outgoing)
        .param("topics", msg.topics.clone())
        .param("sentiment", msg.sentiment);

        self.graph.run(q).await?;

        // Create SENT_BY relationship
        let sent_by = query(
            "MATCH (m:Message {uuid: $uuid})
             MERGE (u:User {user_id: $sender_id})
             ON CREATE SET u.name = $sender_name
             MERGE (u)-[:SENT]->(m)",
        )
        .param("uuid", msg.id.to_string())
        .param("sender_id", msg.sender_id)
        .param("sender_name", msg.sender_name.clone());

        self.graph.run(sent_by).await?;

        // Create IN_CHAT relationship
        let in_chat = query(
            "MATCH (m:Message {uuid: $uuid})
             MERGE (c:Chat {chat_id: $chat_id})
             ON CREATE SET c.name = $chat_name
             MERGE (m)-[:IN_CHAT]->(c)",
        )
        .param("uuid", msg.id.to_string())
        .param("chat_id", msg.chat_id)
        .param("chat_name", msg.chat_name.clone());

        self.graph.run(in_chat).await?;

        // Create REPLIES_TO relationship if applicable
        if let Some(reply_to_id) = msg.reply_to_id {
            let replies_to = query(
                "MATCH (m:Message {uuid: $uuid})
                 MATCH (target:Message {telegram_id: $reply_to_id})
                 WHERE (target)-[:IN_CHAT]->(:Chat {chat_id: $chat_id})
                 MERGE (m)-[:REPLIES_TO]->(target)",
            )
            .param("uuid", msg.id.to_string())
            .param("reply_to_id", reply_to_id)
            .param("chat_id", msg.chat_id);

            self.graph.run(replies_to).await?;
        }

        debug!("Upserted message: {}", msg.id);
        Ok(())
    }

    /// Bulk upsert messages
    pub async fn upsert_messages(&self, messages: &[AnalyzedMessage]) -> Result<usize> {
        let mut count = 0;
        for msg in messages {
            self.upsert_message(msg).await?;
            count += 1;
        }
        info!("Upserted {} messages to Neo4j", count);
        Ok(count)
    }

    /// Create a relationship between nodes
    pub async fn create_relation(&self, relation: &MessageRelation) -> Result<()> {
        let rel_type = relation.relation_type.as_str();

        // Dynamic relationship type requires string interpolation
        // This is safe because rel_type comes from our enum
        let cypher = format!(
            "MATCH (a {{uuid: $from_id}})
             MATCH (b {{uuid: $to_id}})
             MERGE (a)-[r:{}]->(b)
             SET r.weight = $weight,
                 r.properties = $properties",
            rel_type
        );

        let q = query(&cypher)
            .param("from_id", relation.from_id.clone())
            .param("to_id", relation.to_id.clone())
            .param("weight", relation.weight as f64)
            .param("properties", relation.properties.to_string());

        self.graph.run(q).await?;
        debug!(
            "Created relation: {} -[{}]-> {}",
            relation.from_id, rel_type, relation.to_id
        );
        Ok(())
    }

    /// Find users who interact most with a given user
    pub async fn find_interacting_users(&self, user_id: i64, limit: usize) -> Result<Vec<UserNode>> {
        let q = query(
            "MATCH (u:User {user_id: $user_id})-[:SENT]->(m:Message)-[:IN_CHAT]->(c:Chat)
             MATCH (other:User)-[:SENT]->(om:Message)-[:IN_CHAT]->(c)
             WHERE other.user_id <> $user_id
             WITH other, count(om) as interactions
             ORDER BY interactions DESC
             LIMIT $limit
             RETURN other",
        )
        .param("user_id", user_id)
        .param("limit", limit as i64);

        let mut result = self.graph.execute(q).await?;
        let mut users = Vec::new();

        while let Some(row) = result.next().await? {
            if let Ok(node) = row.get::<Node>("other") {
                let user = UserNode {
                    user_id: node.get("user_id").unwrap_or(0),
                    name: node.get("name").unwrap_or_default(),
                    username: node.get("username").ok(),
                    message_count: node.get::<i64>("message_count").unwrap_or(0) as u32,
                    reactions_received: node.get::<i64>("reactions_received").unwrap_or(0) as u32,
                    avg_sentiment: node.get("avg_sentiment").ok(),
                    active_chats: Vec::new(),
                };
                users.push(user);
            }
        }

        Ok(users)
    }

    /// Find messages in a conversation thread (replies chain)
    pub async fn find_conversation_thread(
        &self,
        message_uuid: &str,
        depth: usize,
    ) -> Result<Vec<AnalyzedMessage>> {
        let q = query(
            "MATCH path = (start:Message {uuid: $uuid})-[:REPLIES_TO*0..$depth]-(related)
             UNWIND nodes(path) as m
             WITH DISTINCT m
             MATCH (sender:User)-[:SENT]->(m)
             MATCH (m)-[:IN_CHAT]->(chat:Chat)
             RETURN m, sender, chat
             ORDER BY m.timestamp",
        )
        .param("uuid", message_uuid)
        .param("depth", depth as i64);

        let mut result = self.graph.execute(q).await?;
        let mut messages = Vec::new();

        while let Some(row) = result.next().await? {
            if let (Ok(m), Ok(sender), Ok(chat)) = (
                row.get::<Node>("m"),
                row.get::<Node>("sender"),
                row.get::<Node>("chat"),
            ) {
                let msg = AnalyzedMessage {
                    id: m
                        .get::<String>("uuid")
                        .ok()
                        .and_then(|s| uuid::Uuid::parse_str(&s).ok())
                        .unwrap_or_else(uuid::Uuid::new_v4),
                    telegram_id: m.get("telegram_id").unwrap_or(0),
                    chat_id: chat.get("chat_id").unwrap_or(0),
                    chat_name: chat.get("name").unwrap_or_default(),
                    sender_id: sender.get("user_id").unwrap_or(0),
                    sender_name: sender.get("name").unwrap_or_default(),
                    text: m.get("text").unwrap_or_default(),
                    timestamp: chrono::DateTime::parse_from_rfc3339(
                        &m.get::<String>("timestamp").unwrap_or_default(),
                    )
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                    reply_to_id: m.get("reply_to_id").ok(),
                    reaction_count: m.get::<i64>("reaction_count").unwrap_or(0) as u32,
                    reactions: m.get("reactions").unwrap_or_default(),
                    is_outgoing: m.get("is_outgoing").unwrap_or(false),
                    embedding: None,
                    topics: m.get("topics").unwrap_or_default(),
                    sentiment: m.get("sentiment").ok(),
                };
                messages.push(msg);
            }
        }

        Ok(messages)
    }

    /// Get graph statistics
    pub async fn stats(&self) -> Result<GraphStats> {
        let counts = query(
            "MATCH (u:User) WITH count(u) as users
             MATCH (c:Chat) WITH users, count(c) as chats
             MATCH (m:Message) WITH users, chats, count(m) as messages
             MATCH ()-[r]->() WITH users, chats, messages, count(r) as relations
             RETURN users, chats, messages, relations",
        );

        let mut result = self.graph.execute(counts).await?;

        if let Some(row) = result.next().await? {
            return Ok(GraphStats {
                user_count: row.get::<i64>("users").unwrap_or(0) as u64,
                chat_count: row.get::<i64>("chats").unwrap_or(0) as u64,
                message_count: row.get::<i64>("messages").unwrap_or(0) as u64,
                relation_count: row.get::<i64>("relations").unwrap_or(0) as u64,
            });
        }

        Ok(GraphStats::default())
    }
}

/// Graph statistics
#[derive(Debug, Default)]
pub struct GraphStats {
    pub user_count: u64,
    pub chat_count: u64,
    pub message_count: u64,
    pub relation_count: u64,
}
