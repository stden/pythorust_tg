//! Integration tests for telegram_reader library
//!
//! These tests verify the public API and module interactions.

use telegram_reader::{
    config::{ChatEntity, Config, DEFAULT_LIMIT, SESSION_NAME},
    error::{Error, Result},
    lightrag::chunker::{Chunk, Chunker},
    prompts::{list_prompts, Prompt},
};

// ============================================================================
// Config Tests
// ============================================================================

#[test]
fn test_config_new_loads_or_defaults() {
    let config = Config::new();
    // Config should have reasonable defaults
    assert!(!config.session_name.is_empty());
    assert!(!config.lock_file.is_empty());
}

#[test]
fn test_config_default_limit() {
    assert_eq!(DEFAULT_LIMIT, 3000);
}

#[test]
fn test_config_session_name() {
    assert_eq!(SESSION_NAME, "telegram_session");
}

#[test]
fn test_config_env_placeholders_resolved() {
    let yaml = r#"
telegram:
  api_id: "${TELEGRAM_API_ID}"
  api_hash: "${TELEGRAM_API_HASH}"
  phone: "${TELEGRAM_PHONE}"
"#;

    let temp_file = std::env::temp_dir().join("test_config_env.yml");
    std::fs::write(&temp_file, yaml).unwrap();

    let original_api_id = std::env::var("TELEGRAM_API_ID").ok();
    let original_api_hash = std::env::var("TELEGRAM_API_HASH").ok();
    let original_phone = std::env::var("TELEGRAM_PHONE").ok();

    std::env::set_var("TELEGRAM_API_ID", "777");
    std::env::set_var("TELEGRAM_API_HASH", "from_env_hash");
    std::env::set_var("TELEGRAM_PHONE", "+19998887766");

    let config = Config::load_from_file(&temp_file).unwrap();

    assert_eq!(config.api_id, 777);
    assert_eq!(config.api_hash, "from_env_hash");
    assert_eq!(config.phone, "+19998887766");

    match original_api_id {
        Some(val) => std::env::set_var("TELEGRAM_API_ID", val),
        None => std::env::remove_var("TELEGRAM_API_ID"),
    }
    match original_api_hash {
        Some(val) => std::env::set_var("TELEGRAM_API_HASH", val),
        None => std::env::remove_var("TELEGRAM_API_HASH"),
    }
    match original_phone {
        Some(val) => std::env::set_var("TELEGRAM_PHONE", val),
        None => std::env::remove_var("TELEGRAM_PHONE"),
    }

    std::fs::remove_file(temp_file).ok();
}

#[test]
fn test_config_parses_numeric_strings() {
    let yaml = r#"
telegram:
  api_id: "123456"
  api_hash: "hash"
  phone: "+1111"
user:
  id: "999"
  name: "Tester"
"#;

    let temp_file = std::env::temp_dir().join("test_config_numeric.yml");
    std::fs::write(&temp_file, yaml).unwrap();

    let original_api_id = std::env::var("TELEGRAM_API_ID").ok();
    let original_user_id = std::env::var("USER_ID").ok();
    std::env::remove_var("TELEGRAM_API_ID");
    std::env::remove_var("USER_ID");

    let config = Config::load_from_file(&temp_file).unwrap();

    assert_eq!(config.api_id, 123456);
    assert_eq!(config.my_user_id, 999);

    match original_api_id {
        Some(val) => std::env::set_var("TELEGRAM_API_ID", val),
        None => std::env::remove_var("TELEGRAM_API_ID"),
    }
    match original_user_id {
        Some(val) => std::env::set_var("USER_ID", val),
        None => std::env::remove_var("USER_ID"),
    }

    std::fs::remove_file(temp_file).ok();
}

#[test]
fn test_chat_entity_variants() {
    // Channel
    let channel = ChatEntity::channel(12345);
    assert!(matches!(channel, ChatEntity::Channel(12345)));

    // Chat (group)
    let chat = ChatEntity::chat(67890);
    assert!(matches!(chat, ChatEntity::Chat(67890)));

    // Username with @
    let user = ChatEntity::username("@john_doe");
    assert!(matches!(user, ChatEntity::Username(ref s) if s == "john_doe"));

    // Username without @
    let user2 = ChatEntity::username("jane_doe");
    assert!(matches!(user2, ChatEntity::Username(ref s) if s == "jane_doe"));

    // User ID
    let user_id = ChatEntity::user_id(999);
    assert!(matches!(user_id, ChatEntity::UserId(999)));
}

#[test]
fn test_config_get_chat_nonexistent() {
    let config = Config::new();
    assert!(config.get_chat("nonexistent_chat_12345").is_none());
}

// ============================================================================
// Error Tests
// ============================================================================

#[test]
fn test_error_variants_display() {
    let errors = vec![
        Error::SessionNotFound("test.session".into()),
        Error::SessionLocked,
        Error::LockError("lock failed".into()),
        Error::TelegramError("api error".into()),
        Error::ChatNotFound("chat123".into()),
        Error::SerializationError("json error".into()),
        Error::OpenAiError("rate limit".into()),
        Error::LinearError("auth error".into()),
        Error::InvalidArgument("bad arg".into()),
        Error::ConnectionError("timeout".into()),
        Error::AuthorizationRequired,
        Error::Unknown("mystery".into()),
    ];

    for err in errors {
        let msg = err.to_string();
        assert!(!msg.is_empty(), "Error message should not be empty");
    }
}

#[test]
fn test_result_type_alias() {
    fn returns_ok() -> Result<i32> {
        Ok(42)
    }

    fn returns_err() -> Result<i32> {
        Err(Error::Unknown("test".into()))
    }

    assert!(returns_ok().is_ok());
    assert!(returns_err().is_err());
}

// ============================================================================
// Chunker Tests
// ============================================================================

#[test]
fn test_chunker_basic_chunking() {
    let chunker = Chunker::new(3, 1);
    let text = "one two three four five";
    let chunks = chunker.chunk(text, "test");

    assert!(!chunks.is_empty());
    // First chunk should have 3 words
    assert!(chunks[0].text.split_whitespace().count() <= 3);
}

#[test]
fn test_chunker_empty_input() {
    let chunker = Chunker::new(5, 2);
    let chunks = chunker.chunk("", "test");
    assert!(chunks.is_empty());
}

#[test]
fn test_chunker_single_word() {
    let chunker = Chunker::new(10, 3);
    let chunks = chunker.chunk("hello", "test");
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].text, "hello");
}

#[test]
fn test_chunk_has_metadata() {
    let chunk = Chunk::new("test text".into(), 0, 2, "my_source");
    assert!(!chunk.id.is_nil());
    assert_eq!(chunk.text, "test text");
    assert_eq!(chunk.start, 0);
    assert_eq!(chunk.end, 2);
    assert_eq!(chunk.source, "my_source");
}

#[test]
fn test_chunker_overlap() {
    let chunker = Chunker::new(4, 2);
    let text = "a b c d e f g h";
    let chunks = chunker.chunk(text, "test");

    // With size 4 and overlap 2, step is 2
    // Should produce overlapping chunks
    if chunks.len() > 1 {
        // Check that chunks overlap by checking words
        let words1: Vec<&str> = chunks[0].text.split_whitespace().collect();
        let words2: Vec<&str> = chunks[1].text.split_whitespace().collect();
        // Last 2 words of chunk 0 should be first 2 words of chunk 1 (overlap)
        if words1.len() >= 4 && words2.len() >= 2 {
            assert_eq!(words1[2], words2[0]);
            assert_eq!(words1[3], words2[1]);
        }
    }
}

// ============================================================================
// Prompts Tests
// ============================================================================

#[test]
fn test_list_prompts_returns_all() {
    let prompts = list_prompts();
    assert!(!prompts.is_empty());
    assert!(prompts.len() >= 5); // At least 5 prompts defined
}

#[test]
fn test_prompt_filenames() {
    assert_eq!(Prompt::SalesAgent.filename(), "sales_agent.md");
    assert_eq!(Prompt::Calculator.filename(), "calculator.md");
    assert_eq!(Prompt::FriendlyAI.filename(), "friendly_ai.md");
    assert_eq!(Prompt::Moderator.filename(), "moderator.md");
    assert_eq!(Prompt::Digest.filename(), "digest.md");
    assert_eq!(Prompt::CrmParser.filename(), "crm_parser.md");
}

#[test]
fn test_prompt_filenames_are_md() {
    for prompt in list_prompts() {
        assert!(
            prompt.filename().ends_with(".md"),
            "Prompt filename should end with .md"
        );
    }
}

// ============================================================================
// Module Availability Tests
// ============================================================================

#[test]
fn test_modules_are_public() {
    // Test that main modules are accessible
    use telegram_reader::config;
    use telegram_reader::error;
    use telegram_reader::lightrag;
    use telegram_reader::prompts;

    // These should compile if modules are public
    let _ = config::SESSION_NAME;
    let _ = error::Error::SessionLocked;
    let _ = lightrag::chunker::Chunker::new(5, 1);
    let _ = prompts::Prompt::Calculator;
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[test]
fn test_chunker_is_clone() {
    let chunker = Chunker::new(5, 2);
    let cloned = chunker.clone();
    let chunks = cloned.chunk("test words here", "src");
    assert!(!chunks.is_empty());
}

#[test]
fn test_chunk_is_clone() {
    let chunk = Chunk::new("text".into(), 0, 1, "src");
    let cloned = chunk.clone();
    assert_eq!(cloned.text, "text");
    assert_eq!(cloned.source, "src");
}

#[test]
fn test_config_is_clone() {
    let config = Config::new();
    let cloned = config.clone();
    assert_eq!(config.session_name, cloned.session_name);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_chunker_unicode_text() {
    let chunker = Chunker::new(3, 1);
    let text = "Привет мир тест эмодзи 🎉 данные";
    let chunks = chunker.chunk(text, "unicode");

    assert!(!chunks.is_empty());
    // Unicode should be preserved
    let all_text: String = chunks.iter().map(|c| c.text.clone()).collect();
    assert!(all_text.contains("Привет") || chunks[0].text.contains("Привет"));
}

#[test]
fn test_error_debug_trait() {
    let err = Error::ChatNotFound("test".into());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("ChatNotFound"));
}

#[test]
fn test_chat_entity_debug_trait() {
    let entity = ChatEntity::channel(123);
    let debug_str = format!("{:?}", entity);
    assert!(debug_str.contains("Channel"));
    assert!(debug_str.contains("123"));
}
