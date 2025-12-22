//! Yandex SpeechKit TTS (Text-to-Speech) Client.

use std::collections::HashMap;
use std::env;
use std::path::Path;

use reqwest::Client;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::{Error, Result};

const TTS_URL: &str = "https://tts.api.cloud.yandex.net/speech/v1/tts:synthesize";
const STT_URL: &str = "https://stt.api.cloud.yandex.net/speech/v1/stt:recognize";

/// Available Russian voices.
pub const VOICES_RU: &[(&str, &str)] = &[
    ("alena", "Алёна (нейтральный женский)"),
    ("filipp", "Филипп (нейтральный мужской)"),
    ("ermil", "Ермиль (нейтральный мужской)"),
    ("jane", "Джейн (нейтральный женский)"),
    ("madirus", "Мадирус (нейтральный мужской)"),
    ("omazh", "Омаж (нейтральный женский)"),
    ("zahar", "Захар (нейтральный мужской)"),
    ("dasha", "Даша (нейтральный женский)"),
    ("julia", "Юлия (строгий женский)"),
    ("lera", "Лера (дружелюбный женский)"),
    ("marina", "Марина (мягкий женский)"),
    ("alexander", "Александр (хороший мужской)"),
    ("kirill", "Кирилл (строгий мужской)"),
    ("anton", "Антон (добрый мужской)"),
];

/// TTS emotion.
#[derive(Debug, Clone, Copy)]
pub enum Emotion {
    Neutral,
    Good,
    Evil,
}

impl Emotion {
    fn as_str(&self) -> &'static str {
        match self {
            Emotion::Neutral => "neutral",
            Emotion::Good => "good",
            Emotion::Evil => "evil",
        }
    }
}

/// Audio format.
#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Lpcm,
    OggOpus,
    Mp3,
}

impl AudioFormat {
    fn as_str(&self) -> &'static str {
        match self {
            AudioFormat::Lpcm => "lpcm",
            AudioFormat::OggOpus => "oggopus",
            AudioFormat::Mp3 => "mp3",
        }
    }
}

/// STT topic.
#[derive(Debug, Clone, Copy)]
pub enum STTTopic {
    General,
    Maps,
    Dates,
    Names,
    Numbers,
}

impl STTTopic {
    fn as_str(&self) -> &'static str {
        match self {
            STTTopic::General => "general",
            STTTopic::Maps => "maps",
            STTTopic::Dates => "dates",
            STTTopic::Names => "names",
            STTTopic::Numbers => "numbers",
        }
    }
}

/// Yandex TTS client.
#[derive(Debug, Clone)]
pub struct YandexTTSClient {
    http: Client,
    api_key: Option<String>,
    iam_token: Option<String>,
    folder_id: String,
    tts_url: String,
    stt_url: String,
}

impl YandexTTSClient {
    /// Create client from environment variables.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("YANDEX_API_KEY").ok();
        let iam_token = env::var("YANDEX_IAM_TOKEN").ok();
        let folder_id = env::var("YANDEX_FOLDER_ID")
            .map_err(|_| Error::InvalidArgument("YANDEX_FOLDER_ID не установлен".to_string()))?;

        if api_key.is_none() && iam_token.is_none() {
            return Err(Error::InvalidArgument(
                "Установите YANDEX_API_KEY или YANDEX_IAM_TOKEN".to_string(),
            ));
        }

        let http = Client::builder()
            .build()
            .map_err(|e| Error::InvalidArgument(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            http,
            api_key,
            iam_token,
            folder_id,
            tts_url: TTS_URL.to_string(),
            stt_url: STT_URL.to_string(),
        })
    }

    /// Create client with credentials.
    pub fn new(
        api_key: Option<String>,
        iam_token: Option<String>,
        folder_id: String,
    ) -> Result<Self> {
        if api_key.is_none() && iam_token.is_none() {
            return Err(Error::InvalidArgument(
                "Нужен API_KEY или IAM_TOKEN".to_string(),
            ));
        }

        let http = Client::builder()
            .build()
            .map_err(|e| Error::InvalidArgument(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            http,
            api_key,
            iam_token,
            folder_id,
            tts_url: TTS_URL.to_string(),
            stt_url: STT_URL.to_string(),
        })
    }

    /// Override API endpoints (primarily for tests).
    pub fn with_urls<S1: Into<String>, S2: Into<String>>(
        mut self,
        tts_url: S1,
        stt_url: S2,
    ) -> Self {
        self.tts_url = tts_url.into();
        self.stt_url = stt_url.into();
        self
    }

    fn get_auth_header(&self) -> String {
        if let Some(ref token) = self.iam_token {
            format!("Bearer {}", token)
        } else if let Some(ref key) = self.api_key {
            format!("Api-Key {}", key)
        } else {
            String::new()
        }
    }

    /// Synthesize speech.
    pub async fn text_to_speech(
        &self,
        text: &str,
        output_path: &Path,
        voice: &str,
        emotion: Emotion,
        speed: f32,
        format: AudioFormat,
    ) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("text", text.to_string());
        params.insert("lang", "ru-RU".to_string());
        params.insert("voice", voice.to_string());
        params.insert("emotion", emotion.as_str().to_string());
        params.insert("speed", speed.to_string());
        params.insert("format", format.as_str().to_string());
        params.insert("folderId", self.folder_id.clone());

        let response = self
            .http
            .post(&self.tts_url)
            .header("Authorization", self.get_auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Yandex TTS request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::InvalidArgument(format!(
                "Yandex TTS error {}: {}",
                status, text
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read audio: {}", e)))?;

        let mut file = File::create(output_path)
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to create file: {}", e)))?;

        file.write_all(&bytes)
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    /// Recognize speech.
    pub async fn speech_to_text(
        &self,
        audio_path: &Path,
        language: &str,
        topic: STTTopic,
    ) -> Result<String> {
        let audio_data = tokio::fs::read(audio_path)
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read audio file: {}", e)))?;

        let response = self
            .http
            .post(&self.stt_url)
            .header("Authorization", self.get_auth_header())
            .query(&[
                ("lang", language),
                ("topic", topic.as_str()),
                ("folderId", &self.folder_id),
            ])
            .body(audio_data)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Yandex STT request failed: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::InvalidArgument(format!(
                "Yandex STT error {}: {}",
                status, text
            )));
        }

        let result: STTResponse = serde_json::from_str(&text)
            .map_err(|e| Error::InvalidArgument(format!("Invalid STT response: {}", e)))?;

        Ok(result.result.unwrap_or_default())
    }

    /// Synthesize speech with SSML.
    pub async fn text_to_speech_ssml(
        &self,
        ssml: &str,
        output_path: &Path,
        voice: &str,
    ) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("ssml", ssml.to_string());
        params.insert("lang", "ru-RU".to_string());
        params.insert("voice", voice.to_string());
        params.insert("format", "mp3".to_string());
        params.insert("folderId", self.folder_id.clone());

        let response = self
            .http
            .post(&self.tts_url)
            .header("Authorization", self.get_auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Yandex TTS request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::InvalidArgument(format!(
                "Yandex TTS error {}: {}",
                status, text
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to read audio: {}", e)))?;

        let mut file = File::create(output_path)
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to create file: {}", e)))?;

        file.write_all(&bytes)
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to write file: {}", e)))?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct STTResponse {
    result: Option<String>,
}

/// List available voices.
pub fn list_voices() {
    println!("Доступные голоса для русского языка:");
    println!("{}", "-".repeat(40));
    for (voice_id, description) in VOICES_RU {
        println!("  {}: {}", voice_id, description);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;
    use std::sync::{LazyLock, Mutex};
    use tempfile::tempdir;
    use tokio::fs;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn test_emotion_as_str() {
        assert_eq!(Emotion::Neutral.as_str(), "neutral");
        assert_eq!(Emotion::Good.as_str(), "good");
        assert_eq!(Emotion::Evil.as_str(), "evil");
    }

    #[test]
    fn test_audio_format_as_str() {
        assert_eq!(AudioFormat::Lpcm.as_str(), "lpcm");
        assert_eq!(AudioFormat::OggOpus.as_str(), "oggopus");
        assert_eq!(AudioFormat::Mp3.as_str(), "mp3");
    }

    #[test]
    fn test_stt_topic_as_str() {
        assert_eq!(STTTopic::General.as_str(), "general");
        assert_eq!(STTTopic::Maps.as_str(), "maps");
        assert_eq!(STTTopic::Dates.as_str(), "dates");
        assert_eq!(STTTopic::Names.as_str(), "names");
        assert_eq!(STTTopic::Numbers.as_str(), "numbers");
    }

    #[test]
    fn test_voices_ru_not_empty() {
        assert!(!VOICES_RU.is_empty());
        assert!(VOICES_RU.len() >= 10);
    }

    #[test]
    fn test_new_requires_credentials() {
        let result = YandexTTSClient::new(None, None, "folder".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_new_with_api_key() {
        let result = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_with_iam_token() {
        let result = YandexTTSClient::new(None, Some("token".to_string()), "folder".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_auth_header_with_iam_token() {
        let client = YandexTTSClient::new(
            Some("key".to_string()),
            Some("token".to_string()),
            "folder".to_string(),
        )
        .unwrap();
        // IAM token takes precedence
        assert_eq!(client.get_auth_header(), "Bearer token");
    }

    #[test]
    fn test_get_auth_header_with_api_key() {
        let client =
            YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string()).unwrap();
        assert_eq!(client.get_auth_header(), "Api-Key key");
    }

    #[test]
    fn test_emotion_clone() {
        let e = Emotion::Neutral;
        let cloned = e;
        assert_eq!(e.as_str(), cloned.as_str());
    }

    #[test]
    fn test_audio_format_clone() {
        let f = AudioFormat::Mp3;
        let cloned = f;
        assert_eq!(f.as_str(), cloned.as_str());
    }

    #[test]
    fn test_stt_topic_clone() {
        let t = STTTopic::General;
        let cloned = t;
        assert_eq!(t.as_str(), cloned.as_str());
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn from_env_requires_folder_id() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _key = EnvGuard::set("YANDEX_API_KEY", "key");
        let _token = EnvGuard::remove("YANDEX_IAM_TOKEN");
        let _folder = EnvGuard::remove("YANDEX_FOLDER_ID");

        let err = YandexTTSClient::from_env().unwrap_err();
        assert!(format!("{err}").contains("YANDEX_FOLDER_ID"));
    }

    #[test]
    fn from_env_requires_credentials() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _key = EnvGuard::remove("YANDEX_API_KEY");
        let _token = EnvGuard::remove("YANDEX_IAM_TOKEN");
        let _folder = EnvGuard::set("YANDEX_FOLDER_ID", "folder");

        let err = YandexTTSClient::from_env().unwrap_err();
        assert!(format!("{err}").contains("YANDEX_API_KEY"));
    }

    #[tokio::test]
    async fn text_to_speech_writes_bytes_to_output_file() {
        let server = MockServer::start_async().await;
        let synth_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/tts")
                .header("Authorization", "Api-Key key")
                .is_true(|req| {
                    let body = String::from_utf8_lossy(req.body().as_ref());
                    body.contains("text=Hello")
                        && body.contains("lang=ru-RU")
                        && body.contains("voice=alena")
                        && body.contains("emotion=good")
                        && body.contains("speed=1.1")
                        && body.contains("format=mp3")
                        && body.contains("folderId=folder")
                });
            then.status(200).body("abc");
        });

        let client = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string())
            .unwrap()
            .with_urls(server.url("/tts"), server.url("/stt"));

        let dir = tempdir().expect("tempdir");
        let out_path = dir.path().join("out.mp3");

        client
            .text_to_speech(
                "Hello",
                &out_path,
                "alena",
                Emotion::Good,
                1.1,
                AudioFormat::Mp3,
            )
            .await
            .unwrap();

        let bytes = fs::read(&out_path).await.expect("read file");
        assert_eq!(bytes, b"abc");
        synth_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn text_to_speech_returns_error_on_non_success_status() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/tts");
            then.status(400).body("bad");
        });

        let client = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string())
            .unwrap()
            .with_urls(server.url("/tts"), server.url("/stt"));

        let dir = tempdir().expect("tempdir");
        let out_path = dir.path().join("out.mp3");

        let err = client
            .text_to_speech(
                "Hello",
                &out_path,
                "alena",
                Emotion::Neutral,
                1.0,
                AudioFormat::Mp3,
            )
            .await
            .unwrap_err();

        assert!(format!("{err}").contains("Yandex TTS error"));
        assert!(format!("{err}").contains("bad"));
    }

    #[tokio::test]
    async fn speech_to_text_returns_result_text() {
        let server = MockServer::start_async().await;
        let stt_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/stt")
                .header("Authorization", "Api-Key key");
            then.status(200).json_body(json!({ "result": "hello" }));
        });

        let client = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string())
            .unwrap()
            .with_urls(server.url("/tts"), server.url("/stt"));

        let dir = tempdir().expect("tempdir");
        let audio_path = dir.path().join("audio.ogg");
        fs::write(&audio_path, b"audio").await.expect("write audio");

        let text = client
            .speech_to_text(&audio_path, "ru-RU", STTTopic::General)
            .await
            .unwrap();

        assert_eq!(text, "hello");
        stt_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn speech_to_text_returns_empty_when_result_missing() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/stt");
            then.status(200).json_body(json!({}));
        });

        let client = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string())
            .unwrap()
            .with_urls(server.url("/tts"), server.url("/stt"));

        let dir = tempdir().expect("tempdir");
        let audio_path = dir.path().join("audio.ogg");
        fs::write(&audio_path, b"audio").await.expect("write audio");

        let text = client
            .speech_to_text(&audio_path, "ru-RU", STTTopic::General)
            .await
            .unwrap();
        assert!(text.is_empty());
    }

    #[tokio::test]
    async fn speech_to_text_returns_error_on_invalid_json() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/stt");
            then.status(200).body("not-json");
        });

        let client = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string())
            .unwrap()
            .with_urls(server.url("/tts"), server.url("/stt"));

        let dir = tempdir().expect("tempdir");
        let audio_path = dir.path().join("audio.ogg");
        fs::write(&audio_path, b"audio").await.expect("write audio");

        let err = client
            .speech_to_text(&audio_path, "ru-RU", STTTopic::General)
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("Invalid STT response"));
    }

    #[tokio::test]
    async fn speech_to_text_returns_error_on_non_success_status() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(POST).path("/stt");
            then.status(500).body("boom");
        });

        let client = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string())
            .unwrap()
            .with_urls(server.url("/tts"), server.url("/stt"));

        let dir = tempdir().expect("tempdir");
        let audio_path = dir.path().join("audio.ogg");
        fs::write(&audio_path, b"audio").await.expect("write audio");

        let err = client
            .speech_to_text(&audio_path, "ru-RU", STTTopic::General)
            .await
            .unwrap_err();

        assert!(format!("{err}").contains("Yandex STT error"));
        assert!(format!("{err}").contains("boom"));
    }

    #[tokio::test]
    async fn text_to_speech_ssml_writes_bytes_to_output_file() {
        let server = MockServer::start_async().await;
        let synth_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/tts")
                .header("Authorization", "Api-Key key")
                .is_true(|req| {
                    let body = String::from_utf8_lossy(req.body().as_ref());
                    body.contains("ssml=%3Cspeak%3Ehi%3C%2Fspeak%3E")
                        && body.contains("lang=ru-RU")
                        && body.contains("voice=alena")
                        && body.contains("format=mp3")
                        && body.contains("folderId=folder")
                });
            then.status(200).body("ssml-bytes");
        });

        let client = YandexTTSClient::new(Some("key".to_string()), None, "folder".to_string())
            .unwrap()
            .with_urls(server.url("/tts"), server.url("/stt"));

        let dir = tempdir().expect("tempdir");
        let out_path = dir.path().join("out.mp3");

        client
            .text_to_speech_ssml("<speak>hi</speak>", &out_path, "alena")
            .await
            .unwrap();

        let bytes = fs::read(&out_path).await.expect("read file");
        assert_eq!(bytes, b"ssml-bytes");
        synth_mock.assert_calls(1);
    }
}
