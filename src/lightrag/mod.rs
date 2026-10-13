//! LightRAG - Lightweight Retrieval-Augmented Generation for Telegram data.
//!
//! This module provides an in-crate implementation inspired by
//! [LightRAG](https://github.com/HKUDS/LightRAG): it combines
//! semantic search over text chunks with a lightweight knowledge
//! graph built from extracted entities and their co-occurrences.
//!
//! The design goals:
//! - zero external services by default (local embeddings fallback)
//! - optional OpenAI embeddings when `OPENAI_API_KEY` is available
//! - fast ingestion + retrieval with small, composable pieces

pub mod chunker;
pub mod entity_extractor;
pub mod graph;
pub mod retriever;

pub use chunker::{Chunk, Chunker, ChunkingStrategy};
pub use entity_extractor::{Entity, EntityExtractor, Relation};
pub use graph::{Edge, KnowledgeGraph, Node};
pub use retriever::{LightRAGConfig, LightRAGRetriever, RetrievalMode, RetrievalResult};
