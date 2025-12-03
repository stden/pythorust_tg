//! Telegram Chat Reader & Auto-responder Library
//!
//! This library provides tools to:
//! - Read and export Telegram chat messages to markdown files
//! - Track message reactions and engagement metrics
//! - Automatically respond to messages using AI (OpenAI integration)
//! - Manage multiple chat configurations and sessions
//! - Store messages in vector database for semantic search
//! - Build relationship graphs in Neo4j for analysis

// Clippy lints - allow some patterns that are intentional in tests and API structures
#![allow(clippy::items_after_test_module)]
#![allow(clippy::const_is_empty)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(clippy::print_literal)]
#![allow(clippy::type_complexity)]
#![allow(clippy::infallible_destructuring_match)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_lifetimes)]

pub mod analysis;
pub mod chat;
pub mod config;
pub mod error;
pub mod export;
pub mod integrations;
pub mod lightrag;
pub mod linear;
pub mod metrics;
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
