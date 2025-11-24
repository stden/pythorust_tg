//! External integrations module.
//!
//! Provides clients for:
//! - OpenAI (chat, whisper, TTS)
//! - Yandex SpeechKit (TTS, STT)
//! - Ollama (local LLM)

pub mod ollama;
pub mod openai;
pub mod yandex_tts;

pub use ollama::OllamaClient;
pub use openai::OpenAIClient;
pub use yandex_tts::YandexTTSClient;
