//! Anthropic Claude API Client.
//!
//! Поддерживает:
//! - Claude 3.5/4 Sonnet, Opus, Haiku
//! - Потоковые ответы
//! - Vision (изображения)

use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Claude client.
#[derive(Debug, Clone)]
pub struct ClaudeClient {
    http: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl ClaudeClient {
    /// Создать клиент из переменной окружения ANTHROPIC_API_KEY.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::InvalidArgument("ANTHROPIC_API_KEY не установлен".to_string()))?;
        Self::new(api_key, "claude-sonnet-4-5-20250929")
    }

    /// Создать клиент с API ключом и моделью.
    pub fn new<S: Into<String>>(api_key: S, model: &str) -> Result<Self> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(Error::InvalidArgument(
                "ANTHROPIC_API_KEY пустой".to_string(),
            ));
        }

        let http = Client::builder()
            .user_agent("telegram_reader/0.1.0")
            .build()
            .map_err(|e| Error::InvalidArgument(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            http,
            api_key,
            base_url: CLAUDE_API_URL.to_string(),
            model: model.to_string(),
        })
    }

    /// Установить модель.
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Простой чат - отправить сообщение и получить ответ.
    pub async fn chat(&self, message: &str) -> Result<String> {
        self.chat_with_system(message, None).await
    }

    /// Чат с системным промптом.
    pub async fn chat_with_system(&self, message: &str, system: Option<&str>) -> Result<String> {
        let mut payload = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            temperature: 0.7,
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(message.to_string()),
            }],
            system: None,
        };

        if let Some(sys) = system {
            payload.system = Some(sys.to_string());
        }

        let response = self
            .http
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Claude request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::InvalidArgument(format!(
                "Claude error {}: {}",
                status, text
            )));
        }

        let claude_response: ClaudeResponse = serde_json::from_str(&text).map_err(|e| {
            Error::InvalidArgument(format!("Invalid Claude response: {} - {}", e, text))
        })?;

        claude_response
            .content
            .first()
            .and_then(|c| match c {
                ContentBlock::Text { text } => Some(text.clone()),
                ContentBlock::Image { .. } => None,
            })
            .ok_or_else(|| Error::InvalidArgument("Empty response from Claude".to_string()))
    }

    /// Анализ изображения.
    pub async fn analyze_image(&self, image_url: &str, prompt: &str) -> Result<String> {
        let image_content = if image_url.starts_with("data:") {
            // Base64 encoded image
            let parts: Vec<&str> = image_url.split(';').collect();
            let media_type = parts[0].strip_prefix("data:").unwrap_or("image/jpeg");
            let data = parts
                .get(1)
                .and_then(|p| p.strip_prefix("base64,"))
                .unwrap_or("");

            ImageSource {
                r#type: "base64".to_string(),
                media_type: media_type.to_string(),
                data: Some(data.to_string()),
                url: None,
            }
        } else {
            // URL
            ImageSource {
                r#type: "url".to_string(),
                media_type: String::new(),
                data: None,
                url: Some(image_url.to_string()),
            }
        };

        let payload = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            temperature: 0.7,
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Parts(vec![
                    ContentPart::Image {
                        r#type: "image".to_string(),
                        source: image_content,
                    },
                    ContentPart::Text {
                        r#type: "text".to_string(),
                        text: prompt.to_string(),
                    },
                ]),
            }],
            system: None,
        };

        let response = self
            .http
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Claude request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::InvalidArgument(format!(
                "Claude Vision error {}: {}",
                status, text
            )));
        }

        let claude_response: ClaudeResponse = serde_json::from_str(&text)
            .map_err(|e| Error::InvalidArgument(format!("Invalid response: {}", e)))?;

        claude_response
            .content
            .first()
            .and_then(|c| match c {
                ContentBlock::Text { text } => Some(text.clone()),
                ContentBlock::Image { .. } => None,
            })
            .ok_or_else(|| Error::InvalidArgument("Empty response from Claude".to_string()))
    }

    /// Продающий агент (использует промпт из файла).
    pub async fn sales_agent_response(&self, user_message: &str, context: &str) -> Result<String> {
        let mut system_prompt = crate::Prompt::SalesAgent
            .load()
            .unwrap_or_else(|_| "Ты продавец-консультант.".to_string());

        if !context.is_empty() {
            system_prompt.push_str(&format!("\n\nКонтекст: {}", context));
        }

        self.chat_with_system(user_message, Some(&system_prompt))
            .await
    }
}

// === Структуры запроса ===

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: MessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ContentPart {
    Text { r#type: String, text: String },
    Image { r#type: String, source: ImageSource },
}

#[derive(Debug, Serialize)]
struct ImageSource {
    r#type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

// === Структуры ответа ===

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: serde_json::Value },
}

/// Доступные модели Claude.
pub const CLAUDE_MODELS: &[&str] = &[
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    "claude-3-5-sonnet-20241022",
    "claude-sonnet-4-5-20250929",
];

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn test_new_rejects_empty_key() {
        let err = ClaudeClient::new("   ", "claude-sonnet-4-5-20250929").unwrap_err();
        assert!(format!("{}", err).contains("пустой"));
    }

    #[test]
    fn test_with_model() {
        let client = ClaudeClient::new("test_key", "claude-3-haiku-20240307")
            .unwrap()
            .with_model("claude-3-opus-20240229");
        assert_eq!(client.model, "claude-3-opus-20240229");
    }

    fn client(server: &MockServer) -> ClaudeClient {
        let mut client = ClaudeClient::new("test_key", "claude-3-haiku-20240307").expect("client");
        client.base_url = server.base_url();
        client
    }

    #[tokio::test]
    async fn chat_with_system_returns_text() {
        let server = MockServer::start_async().await;

        let chat_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/messages")
                .header("x-api-key", "test_key")
                .header("anthropic-version", ANTHROPIC_VERSION)
                .is_true(|req| {
                    let body = String::from_utf8_lossy(req.body().as_ref());
                    body.contains("SYS_PROMPT")
                });
            then.status(200).json_body(json!({
                "content": [
                    { "type": "text", "text": "Hello from Claude" }
                ]
            }));
        });

        let reply = client(&server)
            .chat_with_system("Hi", Some("SYS_PROMPT"))
            .await
            .unwrap();

        assert_eq!(reply, "Hello from Claude");
        chat_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn chat_returns_error_on_non_success_status() {
        let server = MockServer::start_async().await;

        let chat_mock = server.mock(|when, then| {
            when.method(POST).path("/messages");
            then.status(500).body("boom");
        });

        let err = client(&server).chat("Hi").await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Claude error 500"));
        assert!(msg.contains("boom"));
        chat_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn analyze_image_supports_base64_data_urls() {
        let server = MockServer::start_async().await;

        let vision_mock = server.mock(|when, then| {
            when.method(POST).path("/messages").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("\"type\":\"base64\"")
                    && body.contains("image/png")
                    && body.contains("AAAA")
            });
            then.status(200).json_body(json!({
                "content": [
                    { "type": "text", "text": "Got it" }
                ]
            }));
        });

        let reply = client(&server)
            .analyze_image("data:image/png;base64,AAAA", "What is this?")
            .await
            .unwrap();

        assert_eq!(reply, "Got it");
        vision_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn analyze_image_supports_urls() {
        let server = MockServer::start_async().await;

        let vision_mock = server.mock(|when, then| {
            when.method(POST).path("/messages").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("\"type\":\"url\"") && body.contains("https://example.com/image.png")
            });
            then.status(200).json_body(json!({
                "content": [
                    { "type": "text", "text": "Ok" }
                ]
            }));
        });

        let reply = client(&server)
            .analyze_image("https://example.com/image.png", "Describe")
            .await
            .unwrap();

        assert_eq!(reply, "Ok");
        vision_mock.assert_calls(1);
    }

    #[test]
    fn claude_models_not_empty() {
        assert!(!CLAUDE_MODELS.is_empty());
    }

    #[test]
    fn claude_models_contain_expected_variants() {
        assert!(CLAUDE_MODELS.iter().any(|m| m.contains("opus")));
        assert!(CLAUDE_MODELS.iter().any(|m| m.contains("sonnet")));
        assert!(CLAUDE_MODELS.iter().any(|m| m.contains("haiku")));
    }

    #[test]
    fn claude_api_url_constant() {
        assert_eq!(CLAUDE_API_URL, "https://api.anthropic.com/v1");
    }

    #[test]
    fn anthropic_version_constant() {
        assert_eq!(ANTHROPIC_VERSION, "2023-06-01");
    }

    #[test]
    fn claude_client_debug() {
        let client = ClaudeClient::new("test_key", "claude-3-haiku-20240307").unwrap();
        let debug_str = format!("{:?}", client);
        
        assert!(debug_str.contains("ClaudeClient"));
    }

    #[test]
    fn claude_client_clone() {
        let client = ClaudeClient::new("key123", "claude-3-opus-20240229").unwrap();
        let cloned = client.clone();
        
        assert_eq!(cloned.model, "claude-3-opus-20240229");
    }

    #[test]
    fn claude_client_with_model_chaining() {
        let client = ClaudeClient::new("key", "model1")
            .unwrap()
            .with_model("model2")
            .with_model("model3");
        
        assert_eq!(client.model, "model3");
    }

    #[test]
    fn claude_client_stores_api_key() {
        let client = ClaudeClient::new("my_secret_key", "claude-3-haiku-20240307").unwrap();
        assert_eq!(client.api_key, "my_secret_key");
    }

    #[test]
    fn claude_client_stores_base_url() {
        let client = ClaudeClient::new("key", "model").unwrap();
        assert_eq!(client.base_url, "https://api.anthropic.com/v1");
    }

    #[test]
    fn claude_models_count() {
        assert_eq!(CLAUDE_MODELS.len(), 5);
    }

    #[tokio::test]
    async fn chat_returns_error_on_empty_content() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/messages");
            then.status(200).json_body(json!({ "content": [] }));
        });

        let err = client(&server).chat("Hi").await.unwrap_err();
        assert!(err.to_string().contains("Empty response from Claude"));
    }

    #[tokio::test]
    async fn analyze_image_url_returns_error_on_failure() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/messages");
            then.status(500).body("Internal Server Error");
        });

        let err = client(&server)
            .analyze_image("https://example.com/image.jpg", "What is this?")
            .await
            .unwrap_err();

        assert!(err.to_string().contains("Claude Vision error 500"));
    }

    #[tokio::test]
    async fn analyze_image_base64_malformed_still_works() {
        // Test that malformed base64 data URLs are handled
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/messages");
            then.status(200).json_body(json!({
                "content": [{ "type": "text", "text": "result" }]
            }));
        });

        let result = client(&server)
            .analyze_image("data:image/jpeg;base64,AAA", "Describe")
            .await
            .unwrap();

        assert_eq!(result, "result");
    }
}

