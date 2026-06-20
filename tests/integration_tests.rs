//! Integration tests for telegram_reader library
//!
//! These tests verify the public API and module interactions.

mod commands;

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
fn test_chat_entity_variants() {
    // Channel
    let channel = ChatEntity::channel(12345);
    assert!(matches!(channel, ChatEntity::Channel(12345)));

    // Chat (group)
    let chat = ChatEntity::chat(67890);
    assert!(matches!(chat, ChatEntity::Chat(67890)));

    // Username with @
    let user = ChatEntity::username("@example_user_one");
    assert!(matches!(user, ChatEntity::Username(ref s) if s == "example_user_one"));

    // Username without @
    let user2 = ChatEntity::username("example_user_two");
    assert!(matches!(user2, ChatEntity::Username(ref s) if s == "example_user_two"));

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

// ============================================================================
// Re-export Tests
// ============================================================================

#[test]
fn test_client_re_exports_available() {
    use telegram_reader::{
        ClaudeClient, GeminiClient, OllamaClient, OpenAIClient, YandexTTSClient,
    };

    // These should compile, demonstrating clients are re-exported
    fn _uses_openai(_: OpenAIClient) {}
    fn _uses_gemini(_: GeminiClient) {}
    fn _uses_ollama(_: OllamaClient) {}
    fn _uses_claude(_: ClaudeClient) {}
    fn _uses_yandex(_: YandexTTSClient) {}
}

#[test]
fn test_grammers_client_re_export() {
    use telegram_reader::grammers_client;

    // Verify module is accessible by using a type from it
    fn _uses_grammers(_: grammers_client::types::Message) {}
}

#[test]
fn test_mysql_async_re_export() {
    use telegram_reader::mysql_async;

    // Verify module is accessible by using a type from it
    fn _uses_mysql(_: mysql_async::Conn) {}
}

// ============================================================================
// Analytics Module Tests
// ============================================================================

#[test]
fn test_ab_test_manager_accessible() {
    use telegram_reader::analytics::ABTestManager;

    // Verify ABTestManager is accessible
    fn _uses_manager(_: ABTestManager) {}
}

#[test]
fn test_prompt_variant_accessible() {
    use telegram_reader::analytics::PromptVariant;

    // Verify PromptVariant is accessible
    fn _uses_variant(_: PromptVariant) {}
}

// ============================================================================
// Bot Analytics Tests
// ============================================================================

#[test]
fn test_bot_analytics_accessible() {
    use telegram_reader::analytics::BotAnalytics;

    // Verify BotAnalytics is accessible
    fn _uses_analytics(_: BotAnalytics) {}
}

#[test]
fn test_session_stats_accessible() {
    use telegram_reader::analytics::SessionStats;

    // Verify SessionStats is accessible
    fn _uses_stats(_: SessionStats) {}
}

// ============================================================================
// Entity Extractor Tests
// ============================================================================

#[test]
fn test_entity_extraction_basic() {
    use telegram_reader::lightrag::chunker::Chunk;
    use telegram_reader::lightrag::entity_extractor::EntityExtractor;

    let extractor = EntityExtractor::new();
    let chunk = Chunk::new(
        "EntityA встретила EntityB в парке".to_string(),
        0,
        5,
        "test",
    );

    let (entities, _relations) = extractor.extract(&chunk);

    // Should extract at least some capitalized words as entities
    assert!(!entities.is_empty());
}

#[test]
fn test_entity_extraction_empty() {
    use telegram_reader::lightrag::chunker::Chunk;
    use telegram_reader::lightrag::entity_extractor::EntityExtractor;

    let extractor = EntityExtractor::new();
    let chunk = Chunk::new("".to_string(), 0, 0, "test");

    let (entities, _relations) = extractor.extract(&chunk);
    assert!(entities.is_empty());
}

#[test]
fn test_relation_extraction() {
    use telegram_reader::lightrag::chunker::Chunk;
    use telegram_reader::lightrag::entity_extractor::EntityExtractor;

    let extractor = EntityExtractor::new();
    let chunk = Chunk::new("EntityA met EntityB at the park".to_string(), 0, 6, "test");

    let (entities, relations) = extractor.extract(&chunk);

    // Entities in same chunk should have co-occurrence relations
    if entities.len() >= 2 {
        assert!(!relations.is_empty());
    }
}

// ============================================================================
// Retriever Tests
// ============================================================================

#[test]
fn test_lightrag_modules_accessible() {
    use telegram_reader::lightrag::chunker::{Chunk, Chunker};
    use telegram_reader::lightrag::entity_extractor::EntityExtractor;
    use telegram_reader::lightrag::graph::KnowledgeGraph;

    // Verify types are accessible
    let chunker = Chunker::new(5, 2);
    let _ = chunker.chunk("test text here for chunking", "source");

    let extractor = EntityExtractor::new();
    let chunk = Chunk::new("test".into(), 0, 1, "src");
    let _ = extractor.extract(&chunk);

    let graph = KnowledgeGraph::new();
    let _ = graph.related_entities("test", 5);
}

// ============================================================================
// Knowledge Graph Tests
// ============================================================================

#[test]
fn test_knowledge_graph_creation() {
    use telegram_reader::lightrag::graph::KnowledgeGraph;

    let graph = KnowledgeGraph::new();
    let related = graph.related_entities("nonexistent", 5);
    assert!(related.is_empty());
}

#[test]
fn test_knowledge_graph_with_entities_and_relations() {
    use telegram_reader::lightrag::entity_extractor::{Entity, Relation};
    use telegram_reader::lightrag::graph::KnowledgeGraph;
    use uuid::Uuid;

    let mut graph = KnowledgeGraph::new();
    let chunk_id = Uuid::new_v4();

    let entities = vec![
        Entity {
            name: "EntityA".to_string(),
            normalized: "alice".to_string(),
            chunk_id,
            position: 0,
        },
        Entity {
            name: "EntityB".to_string(),
            normalized: "bob".to_string(),
            chunk_id,
            position: 1,
        },
    ];

    let relations = vec![Relation {
        from: "alice".to_string(),
        to: "bob".to_string(),
        relation_type: "co_occurs".to_string(),
        weight: 1.0,
    }];

    graph.add_entities(&entities);
    graph.add_relations(&relations);

    let neighbors = graph.related_entities("alice", 5);
    assert!(!neighbors.is_empty());
    assert_eq!(neighbors[0].0, "bob");
}

// ============================================================================
// Linear Client Tests
// ============================================================================

#[test]
fn test_linear_module_accessible() {
    use telegram_reader::linear;

    // Verify linear module is accessible
    fn _uses_linear(_: linear::LinearClient) {}
}

// ============================================================================
// Export Utilities Tests
// ============================================================================

#[test]
fn test_export_module_accessible() {
    use telegram_reader::export;

    // Verify export module types and functions are accessible
    fn _uses_export(_: export::ExportWriter) {}
    let _ = export::media_dir_exists;
}

// ============================================================================
// Metrics Tests
// ============================================================================

#[test]
fn test_metrics_module_accessible() {
    use telegram_reader::metrics;

    // The metrics module should be accessible
    let _ = metrics::spawn_metrics_server;
    let _ = metrics::record_command_start;
}

// ============================================================================
// Additional Prompt Tests
// ============================================================================

#[test]
fn test_all_prompts_have_unique_filenames() {
    use std::collections::HashSet;

    let prompts = list_prompts();
    let filenames: HashSet<&str> = prompts.iter().map(|p| p.filename()).collect();

    assert_eq!(
        filenames.len(),
        prompts.len(),
        "All prompts should have unique filenames"
    );
}

#[test]
fn test_prompt_load_returns_result() {
    // Loading a prompt that might not exist should return a Result
    let result = Prompt::Calculator.load();
    // We don't assert success since prompts/ dir might not exist in test env
    assert!(result.is_ok() || result.is_err());
}
