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
        Some(other) => Err(D::Error::custom(format!("expected string or number, got {:?}", other))),
    }
}

#[derive(Debug, Deserialize)]
struct LimitsConfig {
    default: Option<usize>,
    ci: Option<usize>,
    media_reaction_threshold: Option<i32>,
    media_reaction_threshold_tg: Option<i32>,
}

#[derive(Debug, Deserialize)]
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
        let temp_file = std::env::temp_dir().join("test_config.yml");
        std::fs::write(&temp_file, yaml).unwrap();

        let config = Config::load_from_file(&temp_file).unwrap();
        assert_eq!(config.api_id, 12345);
        assert_eq!(config.api_hash, "test_hash");
        assert_eq!(config.my_user_id, 999);
        assert!(config.chats.contains_key("test_channel"));
        assert!(config.chats.contains_key("test_user"));

        std::fs::remove_file(temp_file).ok();
    }
}
