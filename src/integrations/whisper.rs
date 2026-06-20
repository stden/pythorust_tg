//! OpenAI Whisper integration for transcribing voice messages.

use std::path::Path;
use anyhow::Result;
use async_openai::{
    types::{AudioResponseFormat, CreateTranscriptionRequestArgs},
    Client,
};
use tokio::fs::File;

pub struct WhisperClient {
    client: Client<async_openai::config::OpenAIConfig>,
    model: String,
}

impl WhisperClient {
    pub fn new() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        Self {
            client: Client::with_config(config),
            model: "whisper-1".to_string(),
        }
    }

    pub async fn transcribe_file<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let request = CreateTranscriptionRequestArgs::default()
            .file(path.as_ref().to_path_buf())
            .model(&self.model)
            .response_format(AudioResponseFormat::Text)
            .build()?;

        let response = self.client.audio().transcribe(request).await?;
        Ok(response.text)
    }

    pub async fn transcribe_bytes(&self, bytes: Vec<u8>, filename: &str) -> Result<String> {
        // async-openai's audio API expects a file path currently in some versions,
        // or it might support bytes in newer ones. 
        // Let's use a temporary file to be safe and compatible.
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(filename);
        tokio::fs::write(&temp_path, bytes).await?;
        
        let result = self.transcribe_file(&temp_path).await;
        
        let _ = tokio::fs::remove_file(&temp_path).await;
        result
    }
}

pub async fn transcribe<P: AsRef<Path>>(path: P) -> Result<String> {
    let whisper = WhisperClient::new();
    whisper.transcribe_file(path).await
}
