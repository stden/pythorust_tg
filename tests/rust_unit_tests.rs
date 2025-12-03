//! Rust unit tests for the telegram_reader project.
//! These tests should be integrated into the Rust project's test structure.

// tests/config_tests.rs
#[cfg(test)]
mod config_tests {
    use super::*;
    use std::env;
    use std::io::Write;
    use telegram_reader::config::{ChatConfig, ChatType, Config};
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_from_file() {
        let yaml_content = r#"
chats:
  test_chat:
    type: channel
    id: 1234567890
  test_group:
    type: group
    id: 9876543210
    title: Test Group
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml_content).unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();

        assert_eq!(config.chats.len(), 2);
        assert!(config.chats.contains_key("test_chat"));

        let test_chat = &config.chats["test_chat"];
        assert_eq!(test_chat.chat_type, ChatType::Channel);
        assert_eq!(test_chat.id, Some(1234567890));
    }

    #[test]
    fn test_config_from_env() {
        env::set_var("TELEGRAM_API_ID", "12345");
        env::set_var("TELEGRAM_API_HASH", "test_hash");
        env::set_var("TELEGRAM_PHONE", "+1234567890");

        let config = Config::from_env();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.api_id, 12345);
        assert_eq!(config.api_hash, "test_hash");
        assert_eq!(config.phone, "+1234567890");
    }

    #[test]
    fn test_chat_config_validation() {
        let chat = ChatConfig {
            chat_type: ChatType::Username,
            id: None,
            username: Some("test_user".to_string()),
            title: None,
        };

        assert!(chat.validate().is_ok());

        let invalid_chat = ChatConfig {
            chat_type: ChatType::Username,
            id: None,
            username: None,
            title: None,
        };

        assert!(invalid_chat.validate().is_err());
    }
}

// tests/session_tests.rs
#[cfg(test)]
mod session_tests {
    use super::*;
    use mockito::{mock, Mock};
    use telegram_reader::session::{SessionConfig, TelegramSession};
    use tokio;

    #[tokio::test]
    async fn test_session_creation() {
        let config = SessionConfig {
            api_id: 12345,
            api_hash: "test_hash".to_string(),
            phone: "+1234567890".to_string(),
            session_name: "test_session".to_string(),
        };

        let session = TelegramSession::new(config);
        assert!(session.is_ok());
    }

    #[tokio::test]
    async fn test_session_connect() {
        let config = SessionConfig::default();
        let session = TelegramSession::new(config).unwrap();

        // This would need proper mocking of grammers Client
        // For now, we test the structure exists
        assert!(!session.is_connected());
    }

    #[tokio::test]
    async fn test_session_disconnect() {
        let config = SessionConfig::default();
        let mut session = TelegramSession::new(config).unwrap();

        // Test disconnect doesn't panic on unconnected session
        let result = session.disconnect().await;
        assert!(result.is_ok());
    }
}

// tests/commands/read_tests.rs
#[cfg(test)]
mod read_tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use grammers_client::types::Message;
    use telegram_reader::commands::read::{MessageFormatter, ReadCommand};

    #[test]
    fn test_message_formatter() {
        let formatter = MessageFormatter::new();

        // Test date formatting
        let date = DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z").unwrap();
        let formatted_date = formatter.format_date(&date.with_timezone(&Utc));
        assert_eq!(formatted_date, "01.01.2025 12:00:00");
    }

    #[test]
    fn test_reaction_formatting() {
        let reactions = vec![("👍", 10), ("❤️", 5), ("🔥", 3)];

        let formatter = MessageFormatter::new();
        let formatted = formatter.format_reactions(&reactions);

        assert!(formatted.contains("👍10"));
        assert!(formatted.contains("❤️5"));
        assert!(formatted.contains("🔥3"));
    }

    #[test]
    fn test_message_filtering() {
        struct MockMessage {
            reactions_count: u32,
            has_media: bool,
        }

        let messages = vec![
            MockMessage {
                reactions_count: 0,
                has_media: false,
            },
            MockMessage {
                reactions_count: 50,
                has_media: false,
            },
            MockMessage {
                reactions_count: 150,
                has_media: true,
            },
        ];

        let filtered: Vec<_> = messages
            .iter()
            .filter(|m| m.reactions_count >= 100)
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].reactions_count, 150);
    }
}

// tests/commands/linear_tests.rs
#[cfg(test)]
mod linear_tests {
    use super::*;
    use mockito::{mock, Mock};
    use telegram_reader::commands::linear::{LinearClient, LinearCommand};
    use telegram_reader::linear::{Issue, Team, User};

    #[tokio::test]
    async fn test_create_issue() {
        let _m = mock("POST", "/graphql")
            .with_status(200)
            .with_body(
                r#"{
                "data": {
                    "issueCreate": {
                        "success": true,
                        "issue": {
                            "id": "123",
                            "title": "Test Issue",
                            "identifier": "TEST-1"
                        }
                    }
                }
            }"#,
            )
            .create();

        let client = LinearClient::new("test_key", &mockito::server_url());

        let result = client
            .create_issue("Test Issue", "team_id", Some("Description"), None)
            .await;

        assert!(result.is_ok());
        let issue = result.unwrap();
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.identifier, "TEST-1");
    }

    #[test]
    fn test_issue_struct() {
        let issue = Issue {
            id: "123".to_string(),
            title: "Test".to_string(),
            description: Some("Description".to_string()),
            identifier: "TEST-1".to_string(),
            url: "https://linear.app".to_string(),
            state: None,
            assignee: None,
            labels: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(issue.id, "123");
        assert!(issue.description.is_some());
    }
}

// tests/integrations/openai_tests.rs
#[cfg(test)]
mod openai_tests {
    use super::*;
    use mockito::{mock, Mock};
    use telegram_reader::integrations::openai::{OpenAIClient, OpenAIConfig};

    #[tokio::test]
    async fn test_openai_chat_completion() {
        let _m = mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_body(
                r#"{
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Test response"
                    }
                }],
                "usage": {
                    "total_tokens": 100
                }
            }"#,
            )
            .create();

        let config = OpenAIConfig {
            api_key: "test_key".to_string(),
            api_url: mockito::server_url(),
            model: "gpt-4".to_string(),
        };

        let client = OpenAIClient::new(config);
        let response = client.chat_completion("Test prompt").await;

        assert!(response.is_ok());
        assert_eq!(response.unwrap(), "Test response");
    }

    #[test]
    fn test_openai_config_from_env() {
        std::env::set_var("OPENAI_API_KEY", "test_key");
        std::env::set_var("OPENAI_MODEL", "gpt-4");

        let config = OpenAIConfig::from_env();
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.api_key, "test_key");
        assert_eq!(config.model, "gpt-4");
    }
}

// tests/error_tests.rs
#[cfg(test)]
mod error_tests {
    use super::*;
    use telegram_reader::error::{Error, ErrorKind};

    #[test]
    fn test_error_creation() {
        let error = Error::new(ErrorKind::ConfigError, "Invalid config");
        assert_eq!(error.kind(), ErrorKind::ConfigError);
        assert_eq!(error.message(), "Invalid config");
    }

    #[test]
    fn test_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error: Error = io_error.into();
        assert_eq!(error.kind(), ErrorKind::IoError);
    }

    #[test]
    fn test_error_display() {
        let error = Error::new(ErrorKind::ApiError, "API request failed");
        let display = format!("{}", error);
        assert!(display.contains("API request failed"));
    }
}

// tests/chat_tests.rs
#[cfg(test)]
mod chat_tests {
    use super::*;
    use chrono::Utc;
    use telegram_reader::chat::{Chat, ChatType, Message};

    #[test]
    fn test_chat_creation() {
        let chat = Chat {
            id: 123456789,
            title: "Test Chat".to_string(),
            chat_type: ChatType::Channel,
            username: Some("test_channel".to_string()),
            member_count: Some(1000),
        };

        assert_eq!(chat.id, 123456789);
        assert_eq!(chat.title, "Test Chat");
        assert!(chat.username.is_some());
    }

    #[test]
    fn test_message_creation() {
        let message = Message {
            id: 1,
            date: Utc::now(),
            sender_id: Some(12345),
            sender_name: Some("User".to_string()),
            text: Some("Hello".to_string()),
            reply_to: None,
            media_type: None,
            reactions: Vec::new(),
            views: Some(100),
            forwards: Some(10),
        };

        assert_eq!(message.id, 1);
        assert!(message.text.is_some());
        assert_eq!(message.reactions.len(), 0);
    }

    #[test]
    fn test_message_has_reactions() {
        let mut message = Message::default();
        assert!(!message.has_reactions());

        message.reactions.push(("👍".to_string(), 5));
        assert!(message.has_reactions());
    }

    #[test]
    fn test_message_total_reactions() {
        let mut message = Message::default();
        message.reactions = vec![
            ("👍".to_string(), 10),
            ("❤️".to_string(), 5),
            ("🔥".to_string(), 3),
        ];

        assert_eq!(message.total_reactions(), 18);
    }
}

// tests/metrics_tests.rs
#[cfg(test)]
mod metrics_tests {
    use super::*;
    use chrono::{DateTime, Duration, Utc};
    use telegram_reader::metrics::{ChatMetrics, MessageStats, UserActivity};

    #[test]
    fn test_chat_metrics_calculation() {
        let mut metrics = ChatMetrics::new("test_chat");

        // Add some messages
        for i in 0..10 {
            metrics.add_message(MessageStats {
                timestamp: Utc::now() - Duration::hours(i),
                sender_id: i as i64,
                reactions_count: i as u32 * 10,
                is_reply: i % 2 == 0,
                has_media: i % 3 == 0,
            });
        }

        assert_eq!(metrics.total_messages(), 10);
        assert_eq!(metrics.unique_users(), 10);
        assert_eq!(metrics.reply_rate(), 0.5);
        assert_eq!(metrics.media_rate(), 0.4);
    }

    #[test]
    fn test_user_activity_tracking() {
        let mut activity = UserActivity::new();

        activity.add_message(12345, Utc::now());
        activity.add_message(12345, Utc::now());
        activity.add_message(67890, Utc::now());

        assert_eq!(activity.get_message_count(12345), 2);
        assert_eq!(activity.get_message_count(67890), 1);
        assert_eq!(activity.get_message_count(11111), 0);
    }

    #[test]
    fn test_peak_hours_calculation() {
        let mut metrics = ChatMetrics::new("test_chat");

        // Add messages at specific hours
        for hour in vec![9, 9, 10, 14, 14, 14, 20, 20] {
            let timestamp = Utc::now().with_hour(hour).unwrap().with_minute(0).unwrap();

            metrics.add_message(MessageStats {
                timestamp,
                sender_id: 1,
                reactions_count: 0,
                is_reply: false,
                has_media: false,
            });
        }

        let peak_hours = metrics.get_peak_hours(3);
        assert_eq!(peak_hours.len(), 3);
        assert!(peak_hours.contains(&14)); // Hour with most messages
    }
}

// tests/integration_tests.rs
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;
    use telegram_reader::{Config, TelegramReader};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_full_read_workflow() {
        // This is a placeholder for integration testing
        // Would require a test telegram account or mocking

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        // Test that output path is created correctly
        assert!(!output_path.exists());

        // In a real test, we would:
        // 1. Create a session
        // 2. Read messages from a test chat
        // 3. Format and save to file
        // 4. Verify output
    }

    #[test]
    fn test_config_loading_integration() {
        let config_content = r#"
telegram:
  api_id: 12345
  api_hash: test_hash
  phone: +1234567890

chats:
  test:
    type: channel
    id: 123456

output:
  format: markdown
  media_dir: ./media
"#;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yml");
        std::fs::write(&config_path, config_content).unwrap();

        let config = Config::from_file(&config_path);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.telegram.api_id, 12345);
        assert!(config.chats.contains_key("test"));
    }
}
