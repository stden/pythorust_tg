//! OpenAI API Client for voice AI salesman and other AI tasks.

use std::env;
use std::path::Path;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const OPENAI_API_URL: &str = "https://api.openai.com/v1";

/// OpenAI client.
#[derive(Debug, Clone)]
pub struct OpenAIClient {
    http: Client,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
    /// Create client from environment variable.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| Error::InvalidArgument("OPENAI_API_KEY не установлен".to_string()))?;
        Self::new(api_key)
    }

    /// Create client with API key.
    pub fn new<S: Into<String>>(api_key: S) -> Result<Self> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(Error::InvalidArgument("OPENAI_API_KEY пустой".to_string()));
        }

        let http = Client::builder()
            .user_agent("telegram_reader/0.1.0")
            .build()
            .map_err(|e| Error::InvalidArgument(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            http,
            api_key,
            base_url: OPENAI_API_URL.to_string(),
        })
    }

    /// Chat completion.
    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens,
        };

        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("OpenAI request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::InvalidArgument(format!(
                "OpenAI error {}: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = serde_json::from_str(&text)
            .map_err(|e| Error::InvalidArgument(format!("Invalid response: {}", e)))?;

        chat_response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| Error::InvalidArgument("Empty response from OpenAI".to_string()))
    }

    /// Продающий агент (использует промпт из файла).
    pub async fn sales_agent_response(&self, user_message: &str, context: &str) -> Result<String> {
        let mut system_prompt = crate::Prompt::SalesAgent
            .load()
            .unwrap_or_else(|_| "Ты продавец-консультант.".to_string());

        if !context.is_empty() {
            system_prompt.push_str(&format!("\n\nКонтекст: {}", context));
        }

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(system_prompt),
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(user_message.to_string()),
            },
        ];

        self.chat_completion(messages, "gpt-4o-mini", 0.8, 1000)
            .await
    }

    /// Transcribe audio using Whisper.
    pub async fn transcribe_audio(&self, audio_path: &Path, language: &str) -> Result<String> {
        let file_bytes = tokio::fs::read(audio_path)
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read audio file: {}", e)))?;

        let file_name = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.ogg");

        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .text("language", language.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_bytes).file_name(file_name.to_string()),
            );

        let response = self
            .http
            .post(format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Whisper request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::InvalidArgument(format!(
                "Whisper error {}: {}",
                status, text
            )));
        }

        let transcription: TranscriptionResponse = serde_json::from_str(&text).map_err(|e| {
            Error::InvalidArgument(format!("Invalid transcription response: {}", e))
        })?;

        Ok(transcription.text)
    }

    /// Text to speech.
    pub async fn text_to_speech(&self, text: &str, output_path: &Path, voice: &str) -> Result<()> {
        let request = TTSRequest {
            model: "tts-1".to_string(),
            voice: voice.to_string(),
            input: text.to_string(),
        };

        let response = self
            .http
            .post(format!("{}/audio/speech", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("TTS request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::InvalidArgument(format!(
                "TTS error {}: {}",
                status, text
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read audio: {}", e)))?;

        tokio::fs::write(output_path, &bytes)
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to write file: {}", e)))?;

        Ok(())
    }
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

#[derive(Debug, Serialize)]
struct TTSRequest {
    model: String,
    voice: String,
    input: String,
}

/// Available TTS voices.
pub const TTS_VOICES: &[&str] = &["alloy", "echo", "fable", "onyx", "nova", "shimmer"];

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_new_rejects_empty_key() {
        let err = OpenAIClient::new("   ").unwrap_err();
        assert!(format!("{}", err).contains("пустой"));
    }

    fn client(server: &MockServer) -> OpenAIClient {
        let mut client = OpenAIClient::new("test_key").expect("client");
        client.base_url = server.base_url();
        client
    }

    #[tokio::test]
    async fn chat_completion_returns_first_choice_content() {
        let server = MockServer::start_async().await;

        let completion_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .header("Authorization", "Bearer test_key");
            then.status(200).json_body(json!({
                "choices": [
                    { "message": { "role": "assistant", "content": "Hello!" } }
                ]
            }));
        });

        let reply = client(&server)
            .chat_completion(
                vec![ChatMessage {
                    role: "user".to_string(),
                    content: Some("Hi".to_string()),
                }],
                "gpt-4o-mini",
                0.2,
                32,
            )
            .await
            .unwrap();

        assert_eq!(reply, "Hello!");
        completion_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn chat_completion_returns_error_on_non_success_status() {
        let server = MockServer::start_async().await;

        let completion_mock = server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(429).body("rate limited");
        });

        let err = client(&server)
            .chat_completion(vec![], "gpt-4o-mini", 0.2, 32)
            .await
            .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("OpenAI error 429"));
        assert!(msg.contains("rate limited"));
        completion_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn chat_completion_returns_error_on_invalid_json() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(200).body("not json");
        });

        let err = client(&server)
            .chat_completion(vec![], "gpt-4o-mini", 0.2, 32)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("Invalid response"));
    }

    #[tokio::test]
    async fn chat_completion_returns_error_on_empty_choices() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(200).json_body(json!({ "choices": [] }));
        });

        let err = client(&server)
            .chat_completion(vec![], "gpt-4o-mini", 0.2, 32)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("Empty response from OpenAI"));
    }

    #[tokio::test]
    async fn chat_completion_returns_error_on_missing_message_content() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(200).json_body(json!({
                "choices": [
                    { "message": { "role": "assistant", "content": null } }
                ]
            }));
        });

        let err = client(&server)
            .chat_completion(vec![], "gpt-4o-mini", 0.2, 32)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("Empty response from OpenAI"));
    }

    #[tokio::test]
    async fn sales_agent_response_includes_context_in_request_body() {
        let server = MockServer::start_async().await;

        let completion_mock = server.mock(|when, then| {
            when.method(POST).path("/chat/completions").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("early adopters")
            });
            then.status(200).json_body(json!({
                "choices": [
                    { "message": { "role": "assistant", "content": "Ok" } }
                ]
            }));
        });

        let reply = client(&server)
            .sales_agent_response("Need info", "early adopters")
            .await
            .unwrap();

        assert_eq!(reply, "Ok");
        completion_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn transcribe_audio_returns_text() {
        let server = MockServer::start_async().await;

        let transcription_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/audio/transcriptions")
                .header("Authorization", "Bearer test_key");
            then.status(200).json_body(json!({ "text": "hello" }));
        });

        let dir = tempdir().expect("tempdir");
        let audio_path = dir.path().join("audio.ogg");
        std::fs::write(&audio_path, b"audio-bytes").expect("write audio");

        let text = client(&server)
            .transcribe_audio(&audio_path, "ru")
            .await
            .unwrap();

        assert_eq!(text, "hello");
        transcription_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn text_to_speech_writes_bytes_to_output_file() {
        let server = MockServer::start_async().await;

        let tts_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/audio/speech")
                .header("Authorization", "Bearer test_key");
            then.status(200).body("abc");
        });

        let dir = tempdir().expect("tempdir");
        let out_path = dir.path().join("out.mp3");

        client(&server)
            .text_to_speech("hi", &out_path, "alloy")
            .await
            .unwrap();

        let bytes = std::fs::read(&out_path).expect("read out");
        assert_eq!(bytes, b"abc");
        tts_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn text_to_speech_returns_error_on_non_success_status() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/audio/speech");
            then.status(400).body("bad request");
        });

        let dir = tempdir().expect("tempdir");
        let out_path = dir.path().join("out.mp3");

        let err = client(&server)
            .text_to_speech("hi", &out_path, "alloy")
            .await
            .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("TTS error 400"));
        assert!(msg.contains("bad request"));
    }
}
