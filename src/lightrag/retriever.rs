use anyhow::{Context, Result};
use std::collections::HashSet;

use tracing::{debug, info, warn};

use super::chunker::{Chunk, Chunker};
use super::entity_extractor::{Entity, EntityExtractor};
use super::graph::KnowledgeGraph;
use crate::analysis::embeddings::EmbeddingService;

/// Retrieval strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    /// Combine vector similarity with graph boosts (default)
    Hybrid,
    /// Only vector similarity
    VectorOnly,
    /// Only graph/entity matching
    GraphOnly,
}

/// Result of a LightRAG retrieval.
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub chunk: Chunk,
    pub score: f32,
    pub matched_entities: Vec<String>,
    pub related_entities: Vec<String>,
}

/// LightRAG configuration.
#[derive(Debug, Clone)]
pub struct LightRAGConfig {
    /// Chunk size (words)
    pub chunk_size: usize,
    /// Overlap between chunks (words)
    pub chunk_overlap: usize,
    /// Top-K vector results
    pub vector_top_k: usize,
    /// Max related entities to return
    pub graph_depth: usize,
    /// Embedding dimension for local embeddings
    pub embedding_dim: usize,
}

impl Default for LightRAGConfig {
    fn default() -> Self {
        Self {
            chunk_size: 128,
            chunk_overlap: 16,
            vector_top_k: 8,
            graph_depth: 4,
            embedding_dim: 256,
        }
    }
}

/// Stored chunk with metadata.
#[derive(Debug, Clone)]
struct IndexedChunk {
    chunk: Chunk,
    embedding: Vec<f32>,
    entities: Vec<Entity>,
}

#[allow(clippy::large_enum_variant)]
enum EmbedBackend {
    OpenAI(EmbeddingService),
    Local(LocalEmbedder),
}

impl EmbedBackend {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        match self {
            EmbedBackend::OpenAI(service) => service.embed_batch(texts).await,
            EmbedBackend::Local(local) => Ok(texts.iter().map(|t| local.embed(t)).collect()),
        }
    }

    fn dimension(&self) -> usize {
        match self {
            EmbedBackend::OpenAI(service) => service.dimension(),
            EmbedBackend::Local(local) => local.dimension(),
        }
    }
}

/// Deterministic, fast embedding for offline/local use.
#[derive(Debug, Clone)]
struct LocalEmbedder {
    dim: usize,
}

impl LocalEmbedder {
    fn new(dim: usize) -> Self {
        Self { dim: dim.max(8) }
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut vec = vec![0.0f32; self.dim];
        for token in text.split_whitespace() {
            let mut hasher = DefaultHasher::new();
            token.to_lowercase().hash(&mut hasher);
            let idx = (hasher.finish() as usize) % self.dim;
            vec[idx] += 1.0;
        }

        normalize(&mut vec);
        vec
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

/// Main LightRAG retriever.
pub struct LightRAGRetriever {
    config: LightRAGConfig,
    chunker: Chunker,
    extractor: EntityExtractor,
    graph: KnowledgeGraph,
    backend: EmbedBackend,
    index: Vec<IndexedChunk>,
}

impl LightRAGRetriever {
    /// Create retriever using OpenAI embeddings if available, otherwise local.
    pub fn new(config: LightRAGConfig) -> Self {
        let backend = match EmbeddingService::new() {
            Ok(service) => {
                info!("LightRAG: using OpenAI embeddings");
                EmbedBackend::OpenAI(service)
            }
            Err(err) => {
                warn!("LightRAG: falling back to local embeddings ({err})");
                EmbedBackend::Local(LocalEmbedder::new(config.embedding_dim))
            }
        };

        Self::new_with_backend(config, backend)
    }

    /// Create retriever with forced local embeddings (useful for tests or offline).
    pub fn with_local(config: LightRAGConfig) -> Self {
        let backend = EmbedBackend::Local(LocalEmbedder::new(config.embedding_dim));
        Self::new_with_backend(config, backend)
    }

    fn new_with_backend(config: LightRAGConfig, backend: EmbedBackend) -> Self {
        Self {
            chunker: Chunker::new(config.chunk_size, config.chunk_overlap),
            extractor: EntityExtractor::new(),
            graph: KnowledgeGraph::new(),
            config,
            backend,
            index: Vec::new(),
        }
    }

    /// Number of indexed chunks.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Ingest text into LightRAG (chunk -> extract -> embed -> graph).
    pub async fn ingest(&mut self, source: &str, text: &str) -> Result<usize> {
        if text.trim().is_empty() {
            return Ok(0);
        }

        self.ingest_documents(&[(source.to_string(), text.to_string())])
            .await
    }

    /// Retrieve relevant chunks with optional graph boost.
    pub async fn retrieve(
        &self,
        query: &str,
        limit: usize,
        mode: RetrievalMode,
    ) -> Result<Vec<RetrievalResult>> {
        if self.index.is_empty() {
            return Ok(Vec::new());
        }

        let query_embedding = if mode == RetrievalMode::GraphOnly {
            vec![0.0; self.backend.dimension()]
        } else {
            self.backend
                .embed(&[query.to_string()])
                .await?
                .into_iter()
                .next()
                .unwrap_or_default()
        };

        let query_entities: HashSet<String> =
            self.extractor.extract_keywords(query).into_iter().collect();

        let mut scored = Vec::new();

        for entry in &self.index {
            let vector_score = if mode == RetrievalMode::GraphOnly {
                0.0
            } else {
                cosine_similarity(&query_embedding, &entry.embedding)
            };

            let matched_entities: Vec<String> = entry
                .entities
                .iter()
                .filter_map(|e| {
                    if query_entities.contains(&e.normalized) {
                        Some(e.name.clone())
                    } else {
                        None
                    }
                })
                .collect();

            let related_entities = self
                .graph
                .neighbors_for_entities(&entry.entities, self.config.graph_depth);

            let graph_score = if matched_entities.is_empty() {
                (related_entities.len() as f32) * 0.01
            } else {
                matched_entities.len() as f32 * 0.05 + related_entities.len() as f32 * 0.01
            };

            let score = match mode {
                RetrievalMode::VectorOnly => vector_score,
                RetrievalMode::GraphOnly => graph_score,
                RetrievalMode::Hybrid => vector_score + graph_score,
            };

            scored.push(RetrievalResult {
                chunk: entry.chunk.clone(),
                score,
                matched_entities,
                related_entities,
            });
        }

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_k = limit.min(scored.len());
        scored.truncate(top_k);
        debug!("LightRAG returned {} results", scored.len());

        Ok(scored)
    }

    /// Ingest multiple documents in one batch to minimize embedding calls.
    pub async fn ingest_documents(&mut self, docs: &[(String, String)]) -> Result<usize> {
        if docs.is_empty() {
            return Ok(0);
        }

        let mut chunk_entities = Vec::new();

        for (source, text) in docs {
            if text.trim().is_empty() {
                continue;
            }

            let chunks = self.chunker.chunk(text, source.clone());
            if chunks.is_empty() {
                continue;
            }

            for chunk in chunks {
                let (entities, relations) = self.extractor.extract(&chunk);
                self.graph.add_entities(&entities);
                self.graph.add_relations(&relations);
                chunk_entities.push((chunk, entities));
            }
        }

        if chunk_entities.is_empty() {
            return Ok(0);
        }

        let embeddings = self
            .backend
            .embed(
                &chunk_entities
                    .iter()
                    .map(|(chunk, _)| chunk.text.clone())
                    .collect::<Vec<_>>(),
            )
            .await
            .context("failed to embed chunks")?;

        for ((chunk, entities), embedding) in chunk_entities.into_iter().zip(embeddings) {
            self.index.push(IndexedChunk {
                chunk,
                embedding,
                entities,
            });
        }

        Ok(self.index.len())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

fn normalize(vec: &mut [f32]) {
    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_handles_edge_cases() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[0.0, 0.0]), 0.0);

        let aligned = cosine_similarity(&[1.0, 0.0], &[2.0, 0.0]);
        assert!((aligned - 1.0).abs() < 1e-6);

        let orthogonal = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(orthogonal.abs() < 1e-6);
    }

    #[test]
    fn normalize_scales_vector_to_unit_length() {
        let mut vec = vec![3.0, 4.0];
        normalize(&mut vec);
        let norm = (vec[0].powi(2) + vec[1].powi(2)).sqrt();

        assert!((norm - 1.0).abs() < 1e-6);
        assert!(vec[1] > vec[0]); // preserves proportions
    }

    #[tokio::test]
    async fn retrieves_relevant_chunks_locally() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig {
            chunk_size: 8,
            vector_top_k: 4,
            ..Default::default()
        });

        rag.ingest(
            "doc1",
            "Alice loves Rust programming and open source projects.",
        )
        .await
        .unwrap();
        rag.ingest(
            "doc2",
            "Gardening on weekends helps Bob relax and enjoy nature.",
        )
        .await
        .unwrap();

        let results = rag
            .retrieve("What does Alice love to code?", 3, RetrievalMode::Hybrid)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].chunk.source, "doc1");
    }

    #[tokio::test]
    async fn ingest_documents_skips_empty_and_counts_chunks() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig {
            chunk_size: 2,
            chunk_overlap: 0,
            ..Default::default()
        });

        let docs = vec![
            ("doc1".to_string(), "one two three four".to_string()),
            ("blank".to_string(), "   ".to_string()),
            ("doc2".to_string(), "five six".to_string()),
        ];

        let indexed = rag.ingest_documents(&docs).await.unwrap();

        assert_eq!(indexed, 3);
        assert_eq!(rag.len(), 3);

        let sources: Vec<String> = rag.index.iter().map(|c| c.chunk.source.clone()).collect();
        assert_eq!(sources.iter().filter(|s| s.as_str() == "doc1").count(), 2);
        assert_eq!(sources.iter().filter(|s| s.as_str() == "doc2").count(), 1);
    }

    #[tokio::test]
    async fn ingest_ignores_whitespace_only_texts() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig::default());
        let initial_len = rag.len();

        let added = rag.ingest("src", "   ").await.unwrap();

        assert_eq!(added, 0);
        assert_eq!(rag.len(), initial_len);
    }

    #[test]
    fn local_embedder_produces_consistent_embeddings() {
        let embedder = LocalEmbedder::new(64);
        let text = "hello world rust programming";

        let emb1 = embedder.embed(text);
        let emb2 = embedder.embed(text);

        assert_eq!(emb1, emb2);
        assert_eq!(emb1.len(), 64);
    }

    #[test]
    fn local_embedder_different_texts_different_embeddings() {
        let embedder = LocalEmbedder::new(64);

        let emb1 = embedder.embed("hello world");
        let emb2 = embedder.embed("goodbye world");

        assert_ne!(emb1, emb2);
    }

    #[test]
    fn local_embedder_respects_minimum_dimension() {
        let embedder = LocalEmbedder::new(0);
        assert_eq!(embedder.dimension(), 8); // minimum is 8
    }

    #[test]
    fn local_embedder_empty_text() {
        let embedder = LocalEmbedder::new(32);
        let emb = embedder.embed("");

        // All zeros (normalized -> all zeros)
        assert_eq!(emb.len(), 32);
        assert!(emb.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn lightrag_config_default_values() {
        let config = LightRAGConfig::default();

        assert_eq!(config.chunk_size, 128);
        assert_eq!(config.chunk_overlap, 16);
        assert_eq!(config.vector_top_k, 8);
        assert_eq!(config.graph_depth, 4);
        assert_eq!(config.embedding_dim, 256);
    }

    #[test]
    fn retrieval_mode_equality() {
        assert_eq!(RetrievalMode::Hybrid, RetrievalMode::Hybrid);
        assert_eq!(RetrievalMode::VectorOnly, RetrievalMode::VectorOnly);
        assert_eq!(RetrievalMode::GraphOnly, RetrievalMode::GraphOnly);
        assert_ne!(RetrievalMode::Hybrid, RetrievalMode::VectorOnly);
    }

    #[tokio::test]
    async fn retrieve_from_empty_index_returns_empty() {
        let rag = LightRAGRetriever::with_local(LightRAGConfig::default());
        assert!(rag.is_empty());

        let results = rag
            .retrieve("test query", 5, RetrievalMode::Hybrid)
            .await
            .unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn retrieve_with_vector_only_mode() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig {
            chunk_size: 8,
            ..Default::default()
        });

        rag.ingest("doc1", "Machine learning is a subset of artificial intelligence")
            .await
            .unwrap();
        rag.ingest("doc2", "Cooking recipes for healthy meals")
            .await
            .unwrap();

        let results = rag
            .retrieve("AI and machine learning", 2, RetrievalMode::VectorOnly)
            .await
            .unwrap();

        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn retrieve_with_graph_only_mode() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig {
            chunk_size: 8,
            ..Default::default()
        });

        rag.ingest("doc1", "Alice collaborates with Bob on Rust projects")
            .await
            .unwrap();

        let results = rag
            .retrieve("Alice", 2, RetrievalMode::GraphOnly)
            .await
            .unwrap();

        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn lightrag_len_and_is_empty() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig {
            chunk_size: 4,
            chunk_overlap: 0,
            ..Default::default()
        });

        assert!(rag.is_empty());
        assert_eq!(rag.len(), 0);

        rag.ingest("doc", "one two three four").await.unwrap();

        assert!(!rag.is_empty());
        assert!(rag.len() > 0);
    }

    #[test]
    fn retrieval_result_construction() {
        let chunk = Chunk::new("test text".to_string(), 0, 2, "source");
        let result = RetrievalResult {
            chunk,
            score: 0.95,
            matched_entities: vec!["entity1".to_string()],
            related_entities: vec!["entity2".to_string(), "entity3".to_string()],
        };

        assert_eq!(result.chunk.text, "test text");
        assert!((result.score - 0.95).abs() < 0.001);
        assert_eq!(result.matched_entities.len(), 1);
        assert_eq!(result.related_entities.len(), 2);
    }

    #[tokio::test]
    async fn ingest_documents_empty_vec_returns_zero() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig::default());

        let count = rag.ingest_documents(&[]).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn retrieve_respects_limit() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig {
            chunk_size: 2,
            chunk_overlap: 0,
            ..Default::default()
        });

        // Ingest text that creates multiple chunks
        rag.ingest("doc", "alpha beta gamma delta epsilon zeta eta theta")
            .await
            .unwrap();

        let results = rag
            .retrieve("alpha", 2, RetrievalMode::Hybrid)
            .await
            .unwrap();

        assert!(results.len() <= 2);
    }

    #[test]
    fn cosine_similarity_mismatched_lengths() {
        assert_eq!(cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn normalize_zero_vector() {
        let mut vec = vec![0.0, 0.0, 0.0];
        normalize(&mut vec);
        // Should remain all zeros without panicking
        assert!(vec.iter().all(|&v| v == 0.0));
    }

    #[tokio::test]
    async fn retrieval_result_clone() {
        let mut rag = LightRAGRetriever::with_local(LightRAGConfig {
            chunk_size: 8,
            ..Default::default()
        });

        rag.ingest("doc", "Rust is a systems programming language")
            .await
            .unwrap();

        let results = rag
            .retrieve("Rust programming", 1, RetrievalMode::Hybrid)
            .await
            .unwrap();

        if let Some(result) = results.first() {
            let cloned = result.clone();
            assert_eq!(result.chunk.text, cloned.chunk.text);
            assert_eq!(result.score, cloned.score);
        }
    }
}
