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
        })
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
            .post(TTS_URL)
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
            .post(STT_URL)
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
            .post(TTS_URL)
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
