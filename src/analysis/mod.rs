//! Message analysis module
//!
//! Provides tools for:
//! - Generating embeddings for messages using OpenAI
//! - Storing messages in vector database (Qdrant)
//! - Building relationship graphs in Neo4j

pub mod embeddings;
pub mod graph_db;
pub mod models;
pub mod vector_db;

pub use embeddings::EmbeddingService;
pub use graph_db::GraphStore;
pub use models::{AnalyzedMessage, MessageRelation, UserNode};
pub use vector_db::VectorStore;
