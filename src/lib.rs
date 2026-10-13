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
//! - Python bindings via PyO3 for MCP server integration

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
pub mod python;
pub mod reactions;
pub mod session;

// Re-export common types
pub use config::{ChatEntity, Config, KNOWN_SENDERS};
pub use error::{Error, Result};
pub use integrations::{ClaudeClient, GeminiClient, OllamaClient, OpenAIClient, YandexTTSClient};
pub use prompts::{load_prompt, Prompt};
pub use session::{check_session_exists, get_client, SessionLock};

// Re-export dependencies for binaries that need them
pub use grammers_client;
pub use mysql_async;

// Commands module uses re-exported types, so it must be declared after the re-exports
pub mod commands;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_modules_accessible() {
        // Verify all public modules are accessible
        let _ = config::SESSION_NAME;
        let _ = error::Error::SessionLocked;
        let _ = prompts::Prompt::Calculator;
    }

    #[test]
    fn test_config_re_exports() {
        let entity = ChatEntity::channel(12345);
        assert!(matches!(entity, ChatEntity::Channel(12345)));

        let _ = Config::new();
        let _ = &*KNOWN_SENDERS;
    }

    #[test]
    fn test_error_re_exports() {
        let err = Error::Unknown("test".to_string());
        assert!(err.to_string().contains("Unknown"));

        fn returns_result() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(returns_result().unwrap(), 42);
    }

    #[test]
    fn test_prompt_re_exports() {
        let prompt = Prompt::Calculator;
        assert_eq!(prompt.filename(), "calculator.md");

        // load_prompt is re-exported
        let _ = load_prompt;
    }

    #[test]
    fn test_session_re_exports() {
        // These are re-exported from session module
        let _ = check_session_exists;
        let _ = get_client;
        let _ = SessionLock::acquire; // Method exists
    }

    #[test]
    fn test_integration_clients_re_exported() {
        // Verify integration clients are re-exported
        fn _uses_openai(_: OpenAIClient) {}
        fn _uses_gemini(_: GeminiClient) {}
        fn _uses_ollama(_: OllamaClient) {}
        fn _uses_claude(_: ClaudeClient) {}
        fn _uses_yandex(_: YandexTTSClient) {}
    }

    #[test]
    fn test_grammers_client_re_export() {
        // Verify grammers_client is re-exported for binaries
        use grammers_client::Client;
        fn _uses_client(_: &Client) {}
    }

    #[test]
    fn test_mysql_async_re_export() {
        // Verify mysql_async is re-exported for binaries
        use mysql_async::Pool;
        fn _uses_pool(_: &Pool) {}
    }
}
