//! Configuration for Telegram API and chat entities
//!
//! Loads configuration from config.yml file

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

/// Default constants (fallback if config.yml not found)
pub const SESSION_NAME: &str = "telegram_session";
pub const LOCK_FILE: &str = "telegram_session.lock";
pub const DEFAULT_LIMIT: usize = 3000;
pub const CI_LIMIT: usize = 1000;
pub const MEDIA_REACTION_THRESHOLD: i32 = 100_000;
pub const MEDIA_REACTION_THRESHOLD_TG: i32 = 1000;

/// Default API_ID for session.rs (must be set via config.yml or env)
pub const API_ID: i32 = 0;

/// Chat entity types
#[derive(Debug, Clone)]
pub enum ChatEntity {
    /// Channel by ID
    Channel(i64),
    /// Group chat by ID
    Chat(i64),
    /// User by username (without @)
    Username(String),
    /// User by ID
    UserId(i64),
}

impl ChatEntity {
    pub fn channel(id: i64) -> Self {
        ChatEntity::Channel(id)
    }

    pub fn chat(id: i64) -> Self {
        ChatEntity::Chat(id)
    }

    pub fn username(name: &str) -> Self {
        let name = name.strip_prefix('@').unwrap_or(name);
        ChatEntity::Username(name.to_string())
    }

    pub fn user_id(id: i64) -> Self {
        ChatEntity::UserId(id)
    }
}

/// YAML config structures
#[derive(Debug, Deserialize)]
struct YamlConfig {
    telegram: Option<TelegramConfig>,
    user: Option<UserConfig>,
    limits: Option<LimitsConfig>,
    chats: Option<HashMap<String, ChatConfig>>,
    openai: Option<OpenAIConfig>,
}

#[derive(Debug, Deserialize)]
struct TelegramConfig {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    api_id: Option<String>,
    api_hash: Option<String>,
    phone: Option<String>,
    session_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UserConfig {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    id: Option<String>,
    name: Option<String>,
}

/// Deserialize a value that can be either a string or a number
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value: Option<serde_yaml::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_yaml::Value::String(s)) => Ok(Some(s)),
        Some(serde_yaml::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {:?}",
            other
        ))),
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LimitsConfig {
    default: Option<usize>,
    ci: Option<usize>,
    media_reaction_threshold: Option<i32>,
    media_reaction_threshold_tg: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatConfig {
    #[serde(rename = "type")]
    chat_type: String,
    id: Option<i64>,
    username: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIConfig {
    model: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

/// Known sender names cache (loaded from config)
pub static KNOWN_SENDERS: LazyLock<HashMap<i64, &'static str>> = LazyLock::new(|| {
    // Пустой по умолчанию - заполняется из config.yml
    HashMap::new()
});

/// Main configuration struct
#[derive(Debug, Clone)]
pub struct Config {
    pub phone: String,
    pub api_id: i32,
    pub api_hash: String,
    pub session_name: String,
    pub lock_file: String,
    pub my_user_id: i64,
    pub default_limit: usize,
    pub ci_limit: usize,
    pub chats: HashMap<String, ChatEntity>,
    pub openai_model: String,
    pub openai_max_tokens: u32,
    pub openai_temperature: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Load configuration from config.yml or use defaults
    /// Environment variables take precedence over config.yml values
    pub fn new() -> Self {
        Self::load_from_file("config.yml")
            .or_else(|_| Self::load_from_file("../config.yml"))
            .unwrap_or_else(|_| Self::defaults())
    }

    /// Resolve a value: prefer env var if config value looks like ${VAR}
    fn resolve_env_string(value: Option<String>, env_key: &str) -> String {
        // If value from YAML looks like ${...}, try env var
        if let Some(ref v) = value {
            if v.starts_with("${") && v.ends_with('}') {
                // Extract var name from ${VAR_NAME}
                let var_name = &v[2..v.len() - 1];
                if let Ok(env_val) = std::env::var(var_name) {
                    return env_val;
                }
            }
        }
        // Also check explicit env_key as fallback
        if let Ok(env_val) = std::env::var(env_key) {
            return env_val;
        }
        value.unwrap_or_default()
    }

    /// Resolve an integer value from string config or env var
    fn resolve_env_i32(value: Option<String>, env_key: &str) -> i32 {
        // If value from YAML looks like ${...}, try env var
        if let Some(ref v) = value {
            if v.starts_with("${") && v.ends_with('}') {
                let var_name = &v[2..v.len() - 1];
                if let Ok(env_val) = std::env::var(var_name) {
                    if let Ok(parsed) = env_val.parse::<i32>() {
                        return parsed;
                    }
                }
            }
            // Try parsing directly if it's a number
            if let Ok(parsed) = v.parse::<i32>() {
                return parsed;
            }
        }
        // Fallback: check explicit env_key
        if let Ok(env_val) = std::env::var(env_key) {
            if let Ok(parsed) = env_val.parse::<i32>() {
                return parsed;
            }
        }
        0
    }

    /// Resolve an i64 value from string config or env var
    fn resolve_env_i64(value: Option<String>, env_key: &str) -> i64 {
        // If value from YAML looks like ${...}, try env var
        if let Some(ref v) = value {
            if v.starts_with("${") && v.ends_with('}') {
                let var_name = &v[2..v.len() - 1];
                if let Ok(env_val) = std::env::var(var_name) {
                    if let Ok(parsed) = env_val.parse::<i64>() {
                        return parsed;
                    }
                }
            }
            // Try parsing directly if it's a number
            if let Ok(parsed) = v.parse::<i64>() {
                return parsed;
            }
        }
        // Fallback: check explicit env_key
        if let Ok(env_val) = std::env::var(env_key) {
            if let Ok(parsed) = env_val.parse::<i64>() {
                return parsed;
            }
        }
        0
    }

    /// Load .env file into environment variables using dotenvy
    fn load_dotenv() {
        // Try to load from current directory first, then parent
        if dotenvy::dotenv().is_err() {
            let _ = dotenvy::from_filename("../.env");
        }
    }

    /// Load configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        // Load .env file first
        Self::load_dotenv();

        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let yaml: YamlConfig = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        let telegram = yaml.telegram.unwrap_or(TelegramConfig {
            api_id: None,
            api_hash: None,
            phone: None,
            session_name: None,
        });

        let user = yaml.user.unwrap_or(UserConfig {
            id: None,
            name: None,
        });

        let limits = yaml.limits.unwrap_or(LimitsConfig {
            default: None,
            ci: None,
            media_reaction_threshold: None,
            media_reaction_threshold_tg: None,
        });

        let openai = yaml.openai.unwrap_or(OpenAIConfig {
            model: None,
            max_tokens: None,
            temperature: None,
        });

        // Parse chats
        let mut chats = HashMap::new();
        if let Some(yaml_chats) = yaml.chats {
            for (name, chat_config) in yaml_chats {
                let entity = match chat_config.chat_type.as_str() {
                    "channel" => {
                        if let Some(id) = chat_config.id {
                            ChatEntity::Channel(id)
                        } else {
                            continue;
                        }
                    }
                    "group" => {
                        if let Some(id) = chat_config.id {
                            ChatEntity::Chat(id)
                        } else {
                            continue;
                        }
                    }
                    "user" => {
                        if let Some(id) = chat_config.id {
                            ChatEntity::UserId(id)
                        } else {
                            continue;
                        }
                    }
                    "username" => {
                        if let Some(username) = chat_config.username {
                            ChatEntity::Username(username)
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };
                chats.insert(name, entity);
            }
        }

        // Resolve values with env var precedence
        let api_id = Self::resolve_env_i32(telegram.api_id, "TELEGRAM_API_ID");
        let api_hash = Self::resolve_env_string(telegram.api_hash, "TELEGRAM_API_HASH");
        let phone = Self::resolve_env_string(telegram.phone, "TELEGRAM_PHONE");
        let my_user_id = Self::resolve_env_i64(user.id, "USER_ID");

        Ok(Self {
            phone,
            api_id,
            api_hash,
            session_name: telegram
                .session_name
                .unwrap_or_else(|| SESSION_NAME.to_string()),
            lock_file: LOCK_FILE.to_string(),
            my_user_id,
            default_limit: limits.default.unwrap_or(DEFAULT_LIMIT),
            ci_limit: limits.ci.unwrap_or(CI_LIMIT),
            chats,
            openai_model: openai.model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
            openai_max_tokens: openai.max_tokens.unwrap_or(150),
            openai_temperature: openai.temperature.unwrap_or(0.7),
        })
    }

    /// Create config with empty defaults (fallback)
    /// User MUST provide config.yml with actual credentials
    fn defaults() -> Self {
        Self {
            phone: String::new(),
            api_id: 0,
            api_hash: String::new(),
            session_name: SESSION_NAME.to_string(),
            lock_file: LOCK_FILE.to_string(),
            my_user_id: 0,
            default_limit: DEFAULT_LIMIT,
            ci_limit: CI_LIMIT,
            chats: HashMap::new(),
            openai_model: "gpt-4o-mini".to_string(),
            openai_max_tokens: 150,
            openai_temperature: 0.7,
        }
    }

    /// Get chat entity by name
    pub fn get_chat(&self, name: &str) -> Option<&ChatEntity> {
        self.chats.get(name)
    }

    /// Check if running in GitHub Actions
    pub fn is_github_actions() -> bool {
        std::env::var("GITHUB_ACTIONS")
            .map(|v| v == "true")
            .unwrap_or(false)
    }

    /// Get message limit based on environment
    pub fn get_limit(&self) -> usize {
        if Self::is_github_actions() {
            self.ci_limit
        } else {
            self.default_limit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct EnvGuard {
        key: String,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(&self.key, value),
                None => std::env::remove_var(&self.key),
            }
        }
    }

    fn set_envs(vars: &[(&str, &str)]) -> Vec<EnvGuard> {
        vars.iter().map(|(k, v)| EnvGuard::set(k, v)).collect()
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.session_name, SESSION_NAME);
        // Config loads from yml or uses defaults
    }

    #[test]
    fn test_chat_entity() {
        let channel = ChatEntity::channel(123);
        assert!(matches!(channel, ChatEntity::Channel(123)));

        let username = ChatEntity::username("@test");
        assert!(matches!(username, ChatEntity::Username(ref s) if s == "test"));
    }

    #[test]
    fn test_get_limit_respects_github_actions() {
        let _lock = ENV_LOCK.lock().unwrap();
        let config = Config::defaults();
        let original = std::env::var("GITHUB_ACTIONS").ok();

        std::env::set_var("GITHUB_ACTIONS", "true");
        assert_eq!(config.get_limit(), CI_LIMIT);

        std::env::set_var("GITHUB_ACTIONS", "false");
        assert_eq!(config.get_limit(), DEFAULT_LIMIT);

        match original {
            Some(value) => std::env::set_var("GITHUB_ACTIONS", value),
            None => std::env::remove_var("GITHUB_ACTIONS"),
        }
    }

    #[test]
    fn test_get_chat_unknown_returns_none() {
        let config = Config::default();
        assert!(config.get_chat("does_not_exist").is_none());
    }

    #[test]
    fn test_load_from_yaml() {
        // This test verifies YAML parsing works correctly.
        // Note: env vars may override YAML values (by design).
        // We test that chat entities are parsed correctly.
        let yaml = r#"
telegram:
  api_id: 12345
  api_hash: "test_hash"
  phone: "+1234567890"

user:
  id: 999
  name: "Test"

chats:
  test_channel:
    type: channel
    id: 123456

  test_user:
    type: username
    username: "testuser"
"#;
        let temp_file = std::env::temp_dir().join("test_config_yaml.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        let config = Config::load_from_file(&temp_file).unwrap();

        // Chats should always be parsed from YAML
        assert!(config.chats.contains_key("test_channel"));
        assert!(config.chats.contains_key("test_user"));

        // Verify chat entity types
        if let Some(entity) = config.chats.get("test_channel") {
            assert!(matches!(entity, ChatEntity::Channel(123456)));
        }
        if let Some(entity) = config.chats.get("test_user") {
            assert!(matches!(entity, ChatEntity::Username(ref s) if s == "testuser"));
        }

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_yaml_api_id_parsing() {
        // Test that api_id can be parsed from YAML when no env var is set
        let yaml = r#"
telegram:
  api_id: 54321
"#;
        let temp_file = std::env::temp_dir().join("test_api_id.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        // If TELEGRAM_API_ID env var is NOT set, YAML value should be used
        // This test just verifies the parsing doesn't fail
        let result = Config::load_from_file(&temp_file);
        assert!(result.is_ok());

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn env_placeholders_are_resolved_from_environment() {
        let _lock = ENV_LOCK.lock().unwrap();
        let yaml = r#"
telegram:
  api_id: "${TELEGRAM_API_ID}"
  api_hash: "${TELEGRAM_API_HASH}"
  phone: "+should_be_overridden"
user:
  id: "${USER_ID}"
  name: "Ignored"
"#;
        let temp_file = std::env::temp_dir().join("config_env_override.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        let _guards = set_envs(&[
            ("TELEGRAM_API_ID", "4242"),
            ("TELEGRAM_API_HASH", "hash_from_env"),
            ("TELEGRAM_PHONE", "+1999"),
            ("USER_ID", "777"),
        ]);

        let config = Config::load_from_file(&temp_file).unwrap();

        assert_eq!(config.api_id, 4242);
        assert_eq!(config.api_hash, "hash_from_env");
        assert_eq!(config.phone, "+1999");
        assert_eq!(config.my_user_id, 777);

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn env_does_not_override_numeric_yaml_values() {
        let _lock = ENV_LOCK.lock().unwrap();
        let yaml = r#"
telegram:
  api_id: 321
  phone: "from_yaml"
"#;
        let temp_file = std::env::temp_dir().join("config_numeric_priority.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        let _guards = set_envs(&[("TELEGRAM_API_ID", "9999"), ("TELEGRAM_PHONE", "+8888")]);

        let config = Config::load_from_file(&temp_file).unwrap();

        // Explicit numeric values from YAML take precedence over env vars,
        // while string values still get overridden by the environment.
        assert_eq!(config.api_id, 321);
        assert_eq!(config.phone, "+8888");

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn skips_invalid_chat_definitions() {
        let yaml = r#"
telegram:
  api_id: 0
  api_hash: "hash"
chats:
  valid_channel:
    type: channel
    id: 123
  missing_id:
    type: channel
  missing_username:
    type: username
  unknown_type:
    type: random
    id: 999
"#;
        let temp_file = std::env::temp_dir().join("config_invalid_chats.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        let config = Config::load_from_file(&temp_file).unwrap();

        assert!(config.chats.contains_key("valid_channel"));
        assert_eq!(config.chats.len(), 1);

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn chat_entity_channel_debug() {
        let channel = ChatEntity::Channel(123456);
        let debug_str = format!("{:?}", channel);
        
        assert!(debug_str.contains("Channel"));
        assert!(debug_str.contains("123456"));
    }

    #[test]
    fn chat_entity_chat_debug() {
        let chat = ChatEntity::Chat(-999);
        let debug_str = format!("{:?}", chat);
        
        assert!(debug_str.contains("Chat"));
        assert!(debug_str.contains("-999"));
    }

    #[test]
    fn chat_entity_username_debug() {
        let username = ChatEntity::Username("testuser".into());
        let debug_str = format!("{:?}", username);
        
        assert!(debug_str.contains("Username"));
        assert!(debug_str.contains("testuser"));
    }

    #[test]
    fn chat_entity_user_id_debug() {
        let user_id = ChatEntity::UserId(777);
        let debug_str = format!("{:?}", user_id);
        
        assert!(debug_str.contains("UserId"));
        assert!(debug_str.contains("777"));
    }

    #[test]
    fn chat_entity_clone() {
        let original = ChatEntity::Channel(111);
        let cloned = original.clone();
        
        match cloned {
            ChatEntity::Channel(id) => assert_eq!(id, 111),
            _ => panic!("Expected Channel variant"),
        }
    }

    #[test]
    fn chat_entity_username_clone() {
        let original = ChatEntity::Username("clone_test".into());
        let cloned = original.clone();
        
        match cloned {
            ChatEntity::Username(s) => assert_eq!(s, "clone_test"),
            _ => panic!("Expected Username variant"),
        }
    }

    #[test]
    fn config_constants_values() {
        assert_eq!(SESSION_NAME, "telegram_session");
        assert_eq!(LOCK_FILE, "telegram_session.lock");
        assert_eq!(DEFAULT_LIMIT, 3000);
        assert_eq!(CI_LIMIT, 1000);
        assert_eq!(API_ID, 0);
    }

    #[test]
    fn config_defaults_has_correct_values() {
        let config = Config::defaults();
        
        assert_eq!(config.session_name, SESSION_NAME);
        assert_eq!(config.lock_file, LOCK_FILE);
        assert_eq!(config.default_limit, DEFAULT_LIMIT);
        assert_eq!(config.ci_limit, CI_LIMIT);
        assert!(config.chats.is_empty());
    }

    #[test]
    fn chat_entity_user_id_constructor() {
        let entity = ChatEntity::user_id(999);
        
        match entity {
            ChatEntity::UserId(id) => assert_eq!(id, 999),
            _ => panic!("Expected UserId variant"),
        }
    }

    #[test]
    fn chat_entity_chat_constructor() {
        let entity = ChatEntity::chat(-12345);
        
        match entity {
            ChatEntity::Chat(id) => assert_eq!(id, -12345),
            _ => panic!("Expected Chat variant"),
        }
    }

    #[test]
    fn config_debug_trait() {
        let config = Config::defaults();
        let debug_str = format!("{:?}", config);
        
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("session_name"));
    }

    #[test]
    fn config_clone() {
        let config = Config::defaults();
        let cloned = config.clone();
        
        assert_eq!(cloned.session_name, config.session_name);
        assert_eq!(cloned.default_limit, config.default_limit);
    }

    #[test]
    fn parses_group_chat_type() {
        let yaml = r#"
telegram:
  api_id: 111
  api_hash: "hash"
chats:
  my_group:
    type: group
    id: -1001234567890
"#;
        let temp_file = std::env::temp_dir().join("config_group_chat.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        let config = Config::load_from_file(&temp_file).unwrap();

        assert!(config.chats.contains_key("my_group"));
        match config.chats.get("my_group") {
            Some(ChatEntity::Chat(id)) => assert_eq!(*id, -1001234567890),
            _ => panic!("Expected Chat variant"),
        }

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn parses_user_type() {
        let yaml = r#"
telegram:
  api_id: 111
  api_hash: "hash"
chats:
  my_user:
    type: user
    id: 5555
"#;
        let temp_file = std::env::temp_dir().join("config_user_type.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        let config = Config::load_from_file(&temp_file).unwrap();

        assert!(config.chats.contains_key("my_user"));
        match config.chats.get("my_user") {
            Some(ChatEntity::UserId(id)) => assert_eq!(*id, 5555),
            _ => panic!("Expected UserId variant"),
        }

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn load_from_file_fails_on_missing_file() {
        let result = Config::load_from_file("/nonexistent/path/config.yml");
        assert!(result.is_err());
    }

    #[test]
    fn load_from_file_fails_on_invalid_yaml() {
        let temp_file = std::env::temp_dir().join("config_invalid_yaml.yml");
        std::fs::write(&temp_file, "{ invalid yaml [").unwrap();

        let result = Config::load_from_file(&temp_file);
        assert!(result.is_err());

        std::fs::remove_file(temp_file).ok();
    }
}

