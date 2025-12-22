//! Google Gemini API Client.
//!
//! Поддерживает:
//! - Gemini 2.0/2.5/3.0 Flash и Pro
//! - Потоковые ответы
//! - Vision (изображения)

use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Google Gemini client.
#[derive(Debug, Clone)]
pub struct GeminiClient {
    http: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl GeminiClient {
    /// Создать клиент из переменной окружения GOOGLE_API_KEY.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("GOOGLE_API_KEY")
            .map_err(|_| Error::InvalidArgument("GOOGLE_API_KEY не установлен".to_string()))?;
        Self::new(api_key, "gemini-2.0-flash")
    }

    /// Создать клиент с API ключом и моделью.
    pub fn new<S: Into<String>>(api_key: S, model: &str) -> Result<Self> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(Error::InvalidArgument("GOOGLE_API_KEY пустой".to_string()));
        }

        let http = Client::builder()
            .user_agent("telegram_reader/0.1.0")
            .build()
            .map_err(|e| Error::InvalidArgument(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            http,
            api_key,
            base_url: GEMINI_API_URL.to_string(),
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
        let mut payload = GeminiRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text {
                    text: message.to_string(),
                }],
            }],
            generation_config: Some(GenerationConfig {
                temperature: 0.7,
                max_output_tokens: 4096,
            }),
            system_instruction: None,
        };

        if let Some(sys) = system {
            payload.system_instruction = Some(SystemInstruction {
                parts: vec![Part::Text {
                    text: sys.to_string(),
                }],
            });
        }

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let response = self
            .http
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Gemini request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::InvalidArgument(format!(
                "Gemini error {}: {}",
                status, text
            )));
        }

        let gemini_response: GeminiResponse = serde_json::from_str(&text).map_err(|e| {
            Error::InvalidArgument(format!("Invalid Gemini response: {} - {}", e, text))
        })?;

        gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                Part::InlineData { .. } => None,
            })
            .ok_or_else(|| Error::InvalidArgument("Empty response from Gemini".to_string()))
    }

    /// Анализ изображения.
    pub async fn analyze_image(
        &self,
        image_data: &[u8],
        prompt: &str,
        mime_type: &str,
    ) -> Result<String> {
        use base64::Engine;
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(image_data);

        let payload = GeminiRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type: mime_type.to_string(),
                            data: image_base64,
                        },
                    },
                    Part::Text {
                        text: prompt.to_string(),
                    },
                ],
            }],
            generation_config: Some(GenerationConfig {
                temperature: 0.7,
                max_output_tokens: 4096,
            }),
            system_instruction: None,
        };

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let response = self
            .http
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Gemini request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::InvalidArgument(format!(
                "Gemini Vision error {}: {}",
                status, text
            )));
        }

        let gemini_response: GeminiResponse = serde_json::from_str(&text)
            .map_err(|e| Error::InvalidArgument(format!("Invalid response: {}", e)))?;

        gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                Part::InlineData { .. } => None,
            })
            .ok_or_else(|| Error::InvalidArgument("Empty response from Gemini".to_string()))
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
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemInstruction")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize, Deserialize)]
struct InlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

// === Структуры ответа ===

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Content,
}

/// Доступные модели Gemini (ноябрь 2025).
pub const GEMINI_MODELS: &[&str] = &[
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
    "gemini-2.5-pro",
    "gemini-3.0-pro",
];

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn test_new_rejects_empty_key() {
        let err = GeminiClient::new("   ", "gemini-2.0-flash").unwrap_err();
        assert!(format!("{}", err).contains("пустой"));
    }

    #[test]
    fn test_with_model() {
        let client = GeminiClient::new("test_key", "gemini-2.0-flash")
            .unwrap()
            .with_model("gemini-2.5-pro");
        assert_eq!(client.model, "gemini-2.5-pro");
    }

    fn client(server: &MockServer) -> GeminiClient {
        let mut client = GeminiClient::new("test_key", "gemini-2.0-flash").expect("client");
        client.base_url = server.base_url();
        client
    }

    #[tokio::test]
    async fn chat_with_system_returns_text() {
        let server = MockServer::start_async().await;

        let chat_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/models/gemini-2.0-flash:generateContent")
                .is_true(|req| {
                    let body = String::from_utf8_lossy(req.body().as_ref());
                    body.contains("SYS_PROMPT")
                });
            then.status(200).json_body(json!({
                "candidates": [
                    {
                        "content": {
                            "role": "model",
                            "parts": [
                                { "text": "Hello from Gemini" }
                            ]
                        }
                    }
                ]
            }));
        });

        let reply = client(&server)
            .chat_with_system("Hi", Some("SYS_PROMPT"))
            .await
            .unwrap();

        assert_eq!(reply, "Hello from Gemini");
        chat_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn chat_returns_error_on_non_success_status() {
        let server = MockServer::start_async().await;

        let chat_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/models/gemini-2.0-flash:generateContent");
            then.status(500).body("boom");
        });

        let err = client(&server).chat("Hi").await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Gemini error 500"));
        assert!(msg.contains("boom"));
        chat_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn chat_returns_error_on_empty_candidates() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST)
                .path("/models/gemini-2.0-flash:generateContent");
            then.status(200).json_body(json!({ "candidates": [] }));
        });

        let err = client(&server).chat("Hi").await.unwrap_err();
        assert!(err.to_string().contains("Empty response from Gemini"));
    }

    #[tokio::test]
    async fn analyze_image_sends_inline_data_and_returns_text() {
        let server = MockServer::start_async().await;

        let vision_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/models/gemini-2.0-flash:generateContent")
                .is_true(|req| {
                    let body = String::from_utf8_lossy(req.body().as_ref());
                    body.contains("AQID") && body.contains("mimeType") && body.contains("image/png")
                });
            then.status(200).json_body(json!({
                "candidates": [
                    {
                        "content": {
                            "role": "model",
                            "parts": [
                                { "text": "Looks good" }
                            ]
                        }
                    }
                ]
            }));
        });

        let reply = client(&server)
            .analyze_image(&[1, 2, 3], "What is this?", "image/png")
            .await
            .unwrap();

        assert_eq!(reply, "Looks good");
        vision_mock.assert_calls(1);
    }

    #[test]
    fn gemini_models_not_empty() {
        assert!(!GEMINI_MODELS.is_empty());
    }

    #[test]
    fn gemini_models_contain_expected_variants() {
        assert!(GEMINI_MODELS.iter().any(|m| m.contains("flash")));
        assert!(GEMINI_MODELS.iter().any(|m| m.contains("pro")));
    }

    #[test]
    fn gemini_api_url_constant() {
        assert_eq!(GEMINI_API_URL, "https://generativelanguage.googleapis.com/v1beta");
    }

    #[test]
    fn gemini_client_debug() {
        let client = GeminiClient::new("test_key", "gemini-2.0-flash").unwrap();
        let debug_str = format!("{:?}", client);
        
        assert!(debug_str.contains("GeminiClient"));
    }

    #[test]
    fn gemini_client_clone() {
        let client = GeminiClient::new("key123", "gemini-2.5-pro").unwrap();
        let cloned = client.clone();
        
        assert_eq!(cloned.model, "gemini-2.5-pro");
    }

    #[test]
    fn gemini_client_with_model_chaining() {
        let client = GeminiClient::new("key", "model1")
            .unwrap()
            .with_model("model2")
            .with_model("model3");
        
        assert_eq!(client.model, "model3");
    }

    #[test]
    fn gemini_client_stores_api_key() {
        let client = GeminiClient::new("my_secret_key", "gemini-2.0-flash").unwrap();
        assert_eq!(client.api_key, "my_secret_key");
    }
}
