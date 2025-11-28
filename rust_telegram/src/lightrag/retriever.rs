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

    /// Ingest text into LightRAG (chunk -> extract -> embed -> graph).
    pub async fn ingest(&mut self, source: &str, text: &str) -> Result<usize> {
        if text.trim().is_empty() {
            return Ok(0);
        }

        let chunks = self.chunker.chunk(text, source);
        if chunks.is_empty() {
            return Ok(0);
        }

        let embeddings = self
            .backend
            .embed(&chunks.iter().map(|c| c.text.clone()).collect::<Vec<_>>())
            .await
            .context("failed to embed chunks")?;

        for (chunk, embedding) in chunks.into_iter().zip(embeddings) {
            let (entities, relations) = self.extractor.extract(&chunk);
            self.graph.add_entities(&entities);
            self.graph.add_relations(&relations);

            self.index.push(IndexedChunk {
                chunk,
                embedding,
                entities,
            });
        }

        Ok(self.index.len())
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

        let query_entities: HashSet<String> = self
            .extractor
            .extract_keywords(query)
            .into_iter()
            .collect();

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

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let top_k = limit.min(scored.len());
        scored.truncate(top_k);
        debug!("LightRAG returned {} results", scored.len());

        Ok(scored)
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

        rag.ingest("doc1", "Alice loves Rust programming and open source projects.")
            .await
            .unwrap();
        rag.ingest("doc2", "Gardening on weekends helps Bob relax and enjoy nature.")
            .await
            .unwrap();

        let results = rag
            .retrieve("What does Alice love to code?", 3, RetrievalMode::Hybrid)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].chunk.source, "doc1");
    }
}
