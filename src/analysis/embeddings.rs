//! Embedding generation service using OpenAI

use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{CreateEmbeddingRequestArgs, EmbeddingInput},
    Client as OpenAIClient,
};
use tracing::{debug, info};

/// Service for generating text embeddings
pub struct EmbeddingService {
    client: OpenAIClient<OpenAIConfig>,
    model: String,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;

        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = OpenAIClient::with_config(config);

        Ok(Self {
            client,
            model: "text-embedding-3-small".to_string(),
        })
    }

    /// Create with custom model
    pub fn with_model(model: impl Into<String>) -> Result<Self> {
        let mut service = Self::new()?;
        service.model = model.into();
        Ok(service)
    }

    /// Generate embedding for a single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(&[text.to_string()]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }

    /// Generate embeddings for multiple texts in batch
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Generating embeddings for {} texts", texts.len());

        // Filter out empty texts and truncate long ones
        let processed: Vec<String> = texts
            .iter()
            .map(|t| {
                let trimmed = t.trim();
                if trimmed.len() > 8000 {
                    trimmed[..8000].to_string()
                } else {
                    trimmed.to_string()
                }
            })
            .filter(|t| !t.is_empty())
            .collect();

        if processed.is_empty() {
            return Ok(vec![Vec::new(); texts.len()]);
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(EmbeddingInput::StringArray(processed.clone()))
            .build()?;

        let response = self.client.embeddings().create(request).await?;

        info!(
            "Generated {} embeddings, tokens used: {}",
            response.data.len(),
            response.usage.total_tokens
        );

        // Map back to original indices (empty texts get empty vectors)
        let mut result = Vec::with_capacity(texts.len());
        let mut embed_iter = response.data.into_iter();

        for text in texts {
            if text.trim().is_empty() {
                result.push(Vec::new());
            } else if let Some(embed) = embed_iter.next() {
                result.push(embed.embedding);
            }
        }

        Ok(result)
    }

    /// Get the embedding dimension for the current model
    pub fn dimension(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536, // default
        }
    }
}

impl Default for EmbeddingService {
    fn default() -> Self {
        Self::new().expect("Failed to create embedding service")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OpenAiKeyGuard {
        original: Option<String>,
    }

    impl OpenAiKeyGuard {
        fn set_dummy() -> Self {
            let original = std::env::var("OPENAI_API_KEY").ok();
            std::env::set_var("OPENAI_API_KEY", "test_key");
            Self { original }
        }
    }

    impl Drop for OpenAiKeyGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                std::env::set_var("OPENAI_API_KEY", value);
            } else {
                std::env::remove_var("OPENAI_API_KEY");
            }
        }
    }

    #[test]
    fn dimension_returns_expected_values() {
        let _guard = OpenAiKeyGuard::set_dummy();

        let default = EmbeddingService::new().unwrap();
        assert_eq!(default.dimension(), 1536);

        let large = EmbeddingService::with_model("text-embedding-3-large").unwrap();
        assert_eq!(large.dimension(), 3072);

        let ada = EmbeddingService::with_model("text-embedding-ada-002").unwrap();
        assert_eq!(ada.dimension(), 1536);

        let custom = EmbeddingService::with_model("custom-model").unwrap();
        assert_eq!(custom.dimension(), 1536);
    }

    #[tokio::test]
    async fn embed_batch_short_circuits_on_empty_texts() {
        let _guard = OpenAiKeyGuard::set_dummy();
        let service = EmbeddingService::new().unwrap();

        let embeddings = service
            .embed_batch(&["   ".to_string(), "\n".to_string()])
            .await
            .unwrap();

        assert_eq!(embeddings.len(), 2);
        assert!(embeddings.iter().all(|e| e.is_empty()));
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_embed_single() {
        dotenvy::dotenv().ok();
        let service = EmbeddingService::new().unwrap();
        let embedding = service.embed("Hello, world!").await.unwrap();
        assert_eq!(embedding.len(), 1536);
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_embed_batch() {
        dotenvy::dotenv().ok();
        let service = EmbeddingService::new().unwrap();
        let texts = vec!["Hello".to_string(), "World".to_string()];
        let embeddings = service.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 2);
    }
}
