//! External integrations module.
//!
//! Provides clients for:
//! - OpenAI (chat, whisper, TTS)
//! - Google Gemini (chat, vision)
//! - Anthropic Claude (chat, vision)
//! - Yandex SpeechKit (TTS, STT)
//! - Ollama (local LLM)

pub mod claude;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod yandex_tts;

pub use claude::ClaudeClient;
pub use gemini::GeminiClient;
pub use ollama::OllamaClient;
pub use openai::OpenAIClient;
pub use yandex_tts::YandexTTSClient;
