//! Ollama Client for local LLM inference.

use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const OLLAMA_URL: &str = "http://localhost:11434";

/// Ollama client for local LLM.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    http: Client,
    base_url: String,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaClient {
    /// Create new client with default URL.
    pub fn new() -> Self {
        Self::with_url(OLLAMA_URL)
    }

    /// Create client with custom URL.
    pub fn with_url(base_url: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http,
            base_url: base_url.to_string(),
        }
    }

    /// Check if Ollama server is running.
    pub async fn is_running(&self) -> bool {
        self.http
            .get(format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let response = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Ollama request failed: {}", e)))?;

        let tags: TagsResponse = response
            .json()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Invalid response: {}", e)))?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    /// Generate text.
    pub async fn generate(
        &self,
        prompt: &str,
        model: &str,
        system: Option<&str>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            system: system.map(|s| s.to_string()),
            stream: false,
            options: GenerateOptions {
                temperature,
                num_predict: max_tokens,
            },
        };

        let response = self
            .http
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Ollama request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::InvalidArgument(format!(
                "Ollama error {}: {}",
                status, text
            )));
        }

        let result: GenerateResponse = response
            .json()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Invalid response: {}", e)))?;

        Ok(result.response)
    }

    /// Chat with model.
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        temperature: f32,
    ) -> Result<String> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            options: ChatOptions { temperature },
        };

        let response = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Ollama request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::InvalidArgument(format!(
                "Ollama error {}: {}",
                status, text
            )));
        }

        let result: ChatResponse = response
            .json()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Invalid response: {}", e)))?;

        Ok(result.message.content)
    }

    /// Sales agent response.
    pub async fn sales_agent_response(
        &self,
        user_message: &str,
        context: &str,
        model: &str,
    ) -> Result<String> {
        let mut system_prompt = r#"Ты — профессиональный продавец-консультант.
Твоя задача — помочь клиенту и убедить его в ценности продукта.
Будь вежливым, но настойчивым. Отвечай кратко (1-3 предложения).
Используй техники продаж: SPIN, AIDA.
Если клиент возражает — обрабатывай возражения.
Если клиент согласен — закрывай сделку."#
            .to_string();

        if !context.is_empty() {
            system_prompt.push_str(&format!("\n\nКонтекст: {}", context));
        }

        self.generate(user_message, model, Some(&system_prompt), 0.8, 500)
            .await
    }

    /// Pull (download) a model.
    pub async fn pull_model(&self, model: &str) -> Result<bool> {
        tracing::info!("Downloading {}...", model);

        let request = PullRequest {
            name: model.to_string(),
        };

        let response = self
            .http
            .post(format!("{}/api/pull", self.base_url))
            .json(&request)
            .timeout(Duration::from_secs(3600)) // 1 hour for large models
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Ollama pull failed: {}", e)))?;

        Ok(response.status().is_success())
    }
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    options: ChatOptions,
}

#[derive(Debug, Serialize)]
struct ChatOptions {
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Debug, Serialize)]
struct PullRequest {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    fn client(server: &MockServer) -> OllamaClient {
        OllamaClient::with_url(server.base_url())
    }

    #[tokio::test]
    async fn list_models_returns_names() {
        let server = MockServer::start_async().await;

        let tags_mock = server.mock(|when, then| {
            when.method(GET).path("/api/tags");
            then.status(200).json_body(json!({
                "models": [
                    { "name": "llama3" },
                    { "name": "mistral" }
                ]
            }));
        });

        let models = client(&server).list_models().await.unwrap();

        assert_eq!(models, vec!["llama3".to_string(), "mistral".to_string()]);
        tags_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn generate_reports_error_on_http_failure() {
        let server = MockServer::start_async().await;

        let gen_mock = server.mock(|when, then| {
            when.method(POST).path("/api/generate");
            then.status(500).body("boom");
        });

        let err = client(&server)
            .generate("hi", "llama3", None, 0.2, 64)
            .await
            .unwrap_err();

        let msg = format!("{err}");
        assert!(msg.contains("Ollama error 500"));
        assert!(msg.contains("boom"));
        gen_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn chat_returns_assistant_message() {
        let server = MockServer::start_async().await;

        let chat_mock = server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200).json_body(json!({
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                }
            }));
        });

        let reply = client(&server)
            .chat(
                vec![ChatMessage {
                    role: "user".into(),
                    content: "Hi".into(),
                }],
                "llama3",
                0.3,
            )
            .await
            .unwrap();

        assert_eq!(reply, "Hello!");
        chat_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn sales_agent_response_passes_context_into_system_prompt() {
        let server = MockServer::start_async().await;

        let generate_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/generate")
                .matches(|req| {
                    let body: serde_json::Value =
                        serde_json::from_slice(req.body().as_ref()).unwrap();
                    let system = body.get("system").and_then(|v| v.as_str()).unwrap_or("");
                    system.contains("Контекст: early adopters") && system.contains("SPIN")
                });
            then.status(200).json_body(json!({ "response": "Offer" }));
        });

        let response = client(&server)
            .sales_agent_response("Need info", "early adopters", "llama3")
            .await
            .unwrap();

        assert_eq!(response, "Offer");
        generate_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn is_running_respects_http_status() {
        let healthy = MockServer::start_async().await;
        healthy.mock(|when, then| {
            when.method(GET).path("/api/tags");
            then.status(200);
        });

        let failing = MockServer::start_async().await;
        failing.mock(|when, then| {
            when.method(GET).path("/api/tags");
            then.status(503);
        });

        assert!(client(&healthy).is_running().await);
        assert!(!client(&failing).is_running().await);
    }
}

/// Recommended models.
pub const RECOMMENDED_MODELS: &[(&str, &str)] = &[
    ("qwen2.5:3b", "1.5GB, быстрая"),
    ("llama3.1:8b", "4.5GB, качественная"),
    ("deepseek-coder:6.7b", "для кода"),
];
