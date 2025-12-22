//! Telegram Chat Reader & Auto-responder Library
//!
//! This library provides tools to:
//! - Read and export Telegram chat messages to markdown files
//! - Track message reactions and engagement metrics
//! - Automatically respond to messages using AI (OpenAI integration)
//! - Manage multiple chat configurations and sessions
//! - Store messages in vector database for semantic search
//! - Build relationship graphs in Neo4j for analysis
//! - A/B testing and bot analytics
//! - N8N monitoring and backup

pub mod analysis;
pub mod analytics;
pub mod chat;
pub mod config;
pub mod error;
pub mod export;
pub mod integrations;
pub mod lightrag;
pub mod linear;
pub mod metrics;
pub mod n8n;
pub mod prompts;
pub mod reactions;
pub mod session;

// Re-export common types
pub use config::{ChatEntity, Config, KNOWN_SENDERS};
pub use error::{Error, Result};
pub use integrations::{ClaudeClient, GeminiClient, OllamaClient, OpenAIClient, YandexTTSClient};
pub use prompts::{load_prompt, Prompt};
pub use session::{check_session_exists, get_client, SessionLock};

// Commands module uses re-exported types, so it must be declared after the re-exports
pub mod commands;
