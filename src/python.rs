//! PyO3 Python bindings for telegram_reader
//!
//! Exposes Telegram chat functionality to Python for use with MCP server.
//!
//! Usage from Python:
//! ```python
//! from telegram_reader import list_configured_chats, fetch_messages, send_message
//!
//! # List chats from config.yml
//! chats = list_configured_chats()
//!
//! # Fetch messages (async)
//! messages = await fetch_messages("chat_name", limit=50)
//!
//! # Send message (async)
//! result = await send_message("chat_name", "Hello!", reply_to=None, silent=False)
//! ```

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::config::{ChatEntity, Config};

// ============================================================================
// Text Processing Module (Phase 1)
// ============================================================================

/// Maximum filename length for sanitized names.
const MAX_FILENAME_LEN: usize = 50;

/// Compiled regex patterns for performance.
static PHONE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\+?\d[\d\s\-\(\)]{8,}\d)").unwrap());

static FILENAME_INVALID_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\w\s\-]").unwrap());

static FILENAME_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Russian keywords indicating purchase intent.
const INTENT_KEYWORDS: &[&str] = &[
    "беру",
    "покупаю",
    "оформляем",
    "оплачиваю",
    "готов купить",
    "давай оформим",
    "давайте оформим",
    "давай заказ",
    "берем",
    "хочу купить",
];

/// Russian keywords indicating delivery/checkout details.
const DELIVERY_KEYWORDS: &[&str] = &["доставка", "оплата", "адрес", "курьер", "самовывоз"];

/// Sanitize a string to be used as a filename.
///
/// Removes invalid characters, replaces whitespace with underscores,
/// and truncates to MAX_FILENAME_LEN.
#[pyfunction]
#[pyo3(signature = (name, fallback = "unknown_chat", max_length = None))]
pub fn sanitize_filename(name: Option<&str>, fallback: &str, max_length: Option<usize>) -> String {
    let max_len = max_length.unwrap_or(MAX_FILENAME_LEN);
    let base = name.unwrap_or("");

    // Remove invalid characters
    let cleaned = FILENAME_INVALID_CHARS.replace_all(base, "");
    // Replace whitespace with underscores
    let normalized = FILENAME_WHITESPACE.replace_all(cleaned.trim(), "_");

    let result = if normalized.is_empty() {
        fallback.to_string()
    } else {
        normalized.to_string()
    };

    // Truncate to max length
    if result.len() > max_len {
        result[..max_len].to_string()
    } else {
        result
    }
}

/// Check if text contains a phone number pattern.
///
/// Uses optimized regex matching for phone number detection.
#[pyfunction]
pub fn contains_phone(text: &str) -> bool {
    PHONE_REGEX.is_match(text)
}

/// Detect conversion intent in user message.
///
/// Returns:
/// - "phone_shared" if a phone number is detected
/// - "purchase_intent" if purchase keywords are found
/// - "checkout_details" if delivery keywords with action words are found
/// - None otherwise
#[pyfunction]
pub fn detect_conversion(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    // Check for phone number first
    if PHONE_REGEX.is_match(text) {
        return Some("phone_shared".to_string());
    }

    let normalized = text.to_lowercase();

    // Check for purchase intent keywords
    for keyword in INTENT_KEYWORDS {
        if normalized.contains(keyword) {
            return Some("purchase_intent".to_string());
        }
    }

    // Check for checkout details (delivery keywords + action words)
    let has_delivery = DELIVERY_KEYWORDS.iter().any(|kw| normalized.contains(kw));
    let has_action = normalized.contains("давай") || normalized.contains("офор");

    if has_delivery && has_action {
        return Some("checkout_details".to_string());
    }

    None
}

/// Check if a message is meaningful (not a command or empty).
#[pyfunction]
pub fn is_meaningful_message(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && !trimmed.starts_with('/')
}

/// Parse ISO 8601 datetime string to components.
///
/// Returns a dict with year, month, day, hour, minute, second, or None if parsing fails.
#[pyfunction]
pub fn parse_iso_datetime(py: Python<'_>, s: &str) -> PyResult<Option<PyObject>> {
    // Handle Z suffix and parse
    let normalized = s.replace('Z', "+00:00");

    // Try parsing with chrono
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&normalized) {
        let dict = PyDict::new(py);
        dict.set_item("year", dt.format("%Y").to_string())?;
        dict.set_item("month", dt.format("%m").to_string())?;
        dict.set_item("day", dt.format("%d").to_string())?;
        dict.set_item("hour", dt.format("%H").to_string())?;
        dict.set_item("minute", dt.format("%M").to_string())?;
        dict.set_item("second", dt.format("%S").to_string())?;
        dict.set_item("iso", dt.to_rfc3339())?;
        return Ok(Some(dict.into()));
    }

    // Try parsing datetime without timezone
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        let dict = PyDict::new(py);
        dict.set_item("year", dt.format("%Y").to_string())?;
        dict.set_item("month", dt.format("%m").to_string())?;
        dict.set_item("day", dt.format("%d").to_string())?;
        dict.set_item("hour", dt.format("%H").to_string())?;
        dict.set_item("minute", dt.format("%M").to_string())?;
        dict.set_item("second", dt.format("%S").to_string())?;
        dict.set_item("iso", dt.format("%Y-%m-%dT%H:%M:%S").to_string())?;
        return Ok(Some(dict.into()));
    }

    Ok(None)
}

/// Parse reactions from a list of dicts with emoji and count.
///
/// Input: [{"emoji": "👍", "count": 5}, ...]
/// Output: (total_count, emoji_string, breakdown_list)
#[pyfunction]
pub fn parse_reactions(
    py: Python<'_>,
    reactions: Vec<HashMap<String, PyObject>>,
) -> PyResult<PyObject> {
    let mut total = 0i32;
    let mut emojis = Vec::new();
    let breakdown = PyList::empty(py);

    for reaction in reactions {
        let emoji = reaction
            .get("emoji")
            .and_then(|e| e.extract::<String>(py).ok())
            .unwrap_or_default();

        let count = reaction
            .get("count")
            .and_then(|c| c.extract::<i32>(py).ok())
            .unwrap_or(0);

        total += count;
        emojis.push(emoji.clone());

        let item = PyDict::new(py);
        item.set_item("emoji", emoji)?;
        item.set_item("count", count)?;
        breakdown.append(item)?;
    }

    let result = PyDict::new(py);
    result.set_item("total", total)?;
    result.set_item("emojis", emojis.join(""))?;
    result.set_item("breakdown", breakdown)?;

    Ok(result.into())
}

/// Build message text with media marker if needed.
#[pyfunction]
#[pyo3(signature = (text, has_media, media_marker = "[Media]"))]
pub fn build_message_text(text: Option<&str>, has_media: bool, media_marker: &str) -> String {
    let base = text.unwrap_or("");
    if has_media {
        if base.is_empty() {
            media_marker.to_string()
        } else {
            format!("{} {}", base, media_marker)
        }
    } else {
        base.to_string()
    }
}

/// Collect reactions summary string from emoji list.
#[pyfunction]
pub fn collect_reactions_summary(emojis: Vec<String>) -> String {
    emojis.join("")
}

/// Chat target information exposed to Python.
#[pyclass]
#[derive(Clone)]
pub struct ChatTarget {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub id: Option<i64>,
    #[pyo3(get)]
    pub username: Option<String>,
    #[pyo3(get)]
    pub title: Option<String>,
}

#[pymethods]
impl ChatTarget {
    fn __repr__(&self) -> String {
        format!(
            "ChatTarget(name='{}', kind='{}', id={:?}, username={:?})",
            self.name, self.kind, self.id, self.username
        )
    }

    /// Convert to Python dict for JSON serialization.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("type", &self.kind)?;
        if let Some(id) = self.id {
            dict.set_item("id", id)?;
        }
        if let Some(ref username) = self.username {
            dict.set_item("username", username)?;
        }
        if let Some(ref title) = self.title {
            dict.set_item("title", title)?;
        }
        Ok(dict.into())
    }
}

impl From<(&str, &ChatEntity)> for ChatTarget {
    fn from((name, entity): (&str, &ChatEntity)) -> Self {
        match entity {
            ChatEntity::Channel(id) => ChatTarget {
                name: name.to_string(),
                kind: "channel".to_string(),
                id: Some(*id),
                username: None,
                title: None,
            },
            ChatEntity::Chat(id) => ChatTarget {
                name: name.to_string(),
                kind: "group".to_string(),
                id: Some(*id),
                username: None,
                title: None,
            },
            ChatEntity::Username(username) => ChatTarget {
                name: name.to_string(),
                kind: "username".to_string(),
                id: None,
                username: Some(username.clone()),
                title: None,
            },
            ChatEntity::UserId(id) => ChatTarget {
                name: name.to_string(),
                kind: "user".to_string(),
                id: Some(*id),
                username: None,
                title: None,
            },
        }
    }
}

/// Message data exposed to Python.
#[pyclass]
#[derive(Clone)]
pub struct Message {
    #[pyo3(get)]
    pub id: i32,
    #[pyo3(get)]
    pub sender_id: Option<i64>,
    #[pyo3(get)]
    pub sender: String,
    #[pyo3(get)]
    pub date: String,
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub reply_to: Option<i32>,
    #[pyo3(get)]
    pub views: Option<i32>,
    #[pyo3(get)]
    pub forwards: Option<i32>,
    #[pyo3(get)]
    pub reactions: HashMap<String, i32>,
    #[pyo3(get)]
    pub has_media: bool,
}

#[pymethods]
impl Message {
    fn __repr__(&self) -> String {
        let text_preview = if self.text.len() > 50 {
            format!("{}...", &self.text[..50])
        } else {
            self.text.clone()
        };
        format!(
            "Message(id={}, sender='{}', text='{}')",
            self.id, self.sender, text_preview
        )
    }

    /// Convert to Python dict for JSON serialization.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("id", self.id)?;
        dict.set_item("sender_id", self.sender_id)?;
        dict.set_item("sender", &self.sender)?;
        dict.set_item("date", &self.date)?;
        dict.set_item("text", &self.text)?;
        dict.set_item("reply_to", self.reply_to)?;
        dict.set_item("views", self.views)?;
        dict.set_item("forwards", self.forwards)?;

        let reactions_dict = PyDict::new(py);
        for (emoji, count) in &self.reactions {
            reactions_dict.set_item(emoji, count)?;
        }
        dict.set_item("reactions", reactions_dict)?;
        dict.set_item("has_media", self.has_media)?;

        Ok(dict.into())
    }
}

/// Send result exposed to Python.
#[pyclass]
#[derive(Clone)]
pub struct SendResult {
    #[pyo3(get)]
    pub message_id: i32,
    #[pyo3(get)]
    pub date: String,
    #[pyo3(get)]
    pub chat_name: String,
}

#[pymethods]
impl SendResult {
    fn __repr__(&self) -> String {
        format!(
            "SendResult(message_id={}, chat='{}', date='{}')",
            self.message_id, self.chat_name, self.date
        )
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("message_id", self.message_id)?;
        dict.set_item("date", &self.date)?;
        dict.set_item("chat", &self.chat_name)?;
        Ok(dict.into())
    }
}

// ============================================================================
// Analytics Module (Phase 2.1)
// ============================================================================

/// Session metrics for bot analytics.
#[pyclass]
#[derive(Clone)]
pub struct SessionMetrics {
    #[pyo3(get, set)]
    pub session_id: i64,
    #[pyo3(get, set)]
    pub user_id: i64,
    #[pyo3(get, set)]
    pub bot_name: String,
    #[pyo3(get, set)]
    pub session_start: String,
    #[pyo3(get, set)]
    pub session_end: Option<String>,
    #[pyo3(get, set)]
    pub messages_in: i32,
    #[pyo3(get, set)]
    pub messages_out: i32,
    #[pyo3(get, set)]
    pub non_command_in: i32,
    #[pyo3(get, set)]
    pub phone_shared: bool,
}

impl SessionMetrics {
    pub fn new(session_id: i64, user_id: i64, bot_name: String, session_start: String) -> Self {
        Self {
            session_id,
            user_id,
            bot_name,
            session_start,
            session_end: None,
            messages_in: 0,
            messages_out: 0,
            non_command_in: 0,
            phone_shared: false,
        }
    }

    pub fn total_messages(&self) -> i32 {
        self.messages_in + self.messages_out
    }

    pub fn is_engaged(&self) -> bool {
        self.non_command_in >= 1
    }

    pub fn is_multi_turn(&self) -> bool {
        self.non_command_in >= 2
    }

    pub fn attach_message(&mut self, text: &str, direction: &str) {
        if direction == "incoming" {
            self.messages_in += 1;
            if is_meaningful_message(text) {
                self.non_command_in += 1;
                if contains_phone(text) {
                    self.phone_shared = true;
                }
            }
        } else {
            self.messages_out += 1;
        }
    }
}

#[pymethods]
impl SessionMetrics {
    #[new]
    #[pyo3(signature = (session_id, user_id, bot_name, session_start))]
    fn py_new(session_id: i64, user_id: i64, bot_name: String, session_start: String) -> Self {
        Self::new(session_id, user_id, bot_name, session_start)
    }

    #[pyo3(name = "total_messages")]
    fn py_total_messages(&self) -> i32 {
        self.total_messages()
    }

    #[pyo3(name = "is_engaged")]
    fn py_is_engaged(&self) -> bool {
        self.is_engaged()
    }

    #[pyo3(name = "is_multi_turn")]
    fn py_is_multi_turn(&self) -> bool {
        self.is_multi_turn()
    }

    #[pyo3(name = "attach_message")]
    fn py_attach_message(&mut self, text: &str, direction: &str) {
        self.attach_message(text, direction);
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionMetrics(session_id={}, user_id={}, bot='{}', in={}, out={})",
            self.session_id, self.user_id, self.bot_name, self.messages_in, self.messages_out
        )
    }
}

/// Retention metrics for bot analytics.
#[pyclass]
#[derive(Clone)]
pub struct RetentionMetrics {
    #[pyo3(get, set)]
    pub total_users: i32,
    #[pyo3(get, set)]
    pub d1_base: i32,
    #[pyo3(get, set)]
    pub d1_returned: i32,
    #[pyo3(get, set)]
    pub d7_base: i32,
    #[pyo3(get, set)]
    pub d7_returned: i32,
}

impl RetentionMetrics {
    pub fn new(total_users: i32, d1_base: i32) -> Self {
        Self {
            total_users,
            d1_base,
            d1_returned: 0,
            d7_base: 0,
            d7_returned: 0,
        }
    }

    pub fn d1_rate(&self) -> f64 {
        if self.d1_base == 0 {
            0.0
        } else {
            (self.d1_returned as f64 / self.d1_base as f64) * 100.0
        }
    }

    pub fn d7_rate(&self) -> f64 {
        if self.d7_base == 0 {
            0.0
        } else {
            (self.d7_returned as f64 / self.d7_base as f64) * 100.0
        }
    }
}

#[pymethods]
impl RetentionMetrics {
    #[new]
    #[pyo3(signature = (total_users, d1_base))]
    fn py_new(total_users: i32, d1_base: i32) -> Self {
        Self::new(total_users, d1_base)
    }

    #[pyo3(name = "d1_rate")]
    fn py_d1_rate(&self) -> f64 {
        self.d1_rate()
    }

    #[pyo3(name = "d7_rate")]
    fn py_d7_rate(&self) -> f64 {
        self.d7_rate()
    }

    fn __repr__(&self) -> String {
        format!(
            "RetentionMetrics(total={}, d1={:.1}%, d7={:.1}%)",
            self.total_users,
            self.d1_rate(),
            self.d7_rate()
        )
    }
}

// ============================================================================
// Formatting Module (Phase 2.2)
// ============================================================================

/// Message data for LLM formatting.
#[derive(Clone)]
pub struct MessageData {
    pub sender: String,
    pub text: String,
    pub date: String,
    pub reactions: Option<String>,
}

/// Chat metadata aggregated from messages.
#[derive(Clone)]
pub struct ChatMetadata {
    pub message_count: usize,
    pub unique_senders: usize,
}

/// Format messages for LLM consumption.
pub fn format_messages_for_llm(messages: &[MessageData], include_reactions: bool) -> String {
    let mut result = String::new();

    for msg in messages {
        result.push_str(&format!("**{}** ({}):\n{}", msg.sender, msg.date, msg.text));
        if include_reactions {
            if let Some(ref reactions) = msg.reactions {
                result.push_str(&format!(" {}", reactions));
            }
        }
        result.push_str("\n\n");
    }

    result
}

/// Get chat metadata from messages.
pub fn get_chat_metadata(messages: &[MessageData]) -> ChatMetadata {
    let message_count = messages.len();
    let unique_senders = messages
        .iter()
        .map(|m| &m.sender)
        .collect::<std::collections::HashSet<_>>()
        .len();

    ChatMetadata {
        message_count,
        unique_senders,
    }
}

/// List configured chats from config.yml
#[pyfunction]
pub fn list_configured_chats(py: Python<'_>) -> PyResult<PyObject> {
    let config = Config::new();
    let list = PyList::empty(py);

    for (name, entity) in &config.chats {
        let target = ChatTarget::from((name.as_str(), entity));
        list.append(target.into_pyobject(py)?)?;
    }

    Ok(list.into())
}

/// Resolve a chat name to ChatTarget.
#[pyfunction]
pub fn resolve_chat(chat: &str) -> PyResult<ChatTarget> {
    let chat = chat.trim();
    if chat.is_empty() {
        return Err(PyRuntimeError::new_err(
            "Chat name or identifier is required.",
        ));
    }

    let config = Config::new();

    // Check if it's a configured chat
    if let Some(entity) = config.chats.get(chat) {
        return Ok(ChatTarget::from((chat, entity)));
    }

    // Check if it's a @username
    if chat.starts_with('@') {
        let username = chat.strip_prefix('@').unwrap_or(chat);
        return Ok(ChatTarget {
            name: chat.to_string(),
            kind: "username".to_string(),
            id: None,
            username: Some(username.to_string()),
            title: None,
        });
    }

    // Check if it's a numeric ID
    if let Ok(id) = chat.parse::<i64>() {
        return Ok(ChatTarget {
            name: chat.to_string(),
            kind: "user".to_string(),
            id: Some(id),
            username: None,
            title: None,
        });
    }

    let available: Vec<String> = config.chats.keys().cloned().collect();
    let available_str = if available.is_empty() {
        "no chats configured".to_string()
    } else {
        available.join(", ")
    };

    Err(PyRuntimeError::new_err(format!(
        "Chat '{}' not found in config.yml. Available: {}",
        chat, available_str
    )))
}

/// Check if Telegram session exists.
#[pyfunction]
pub fn check_session() -> PyResult<bool> {
    match crate::session::check_session_exists() {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Get session lock file path.
#[pyfunction]
pub fn get_lock_file_path() -> String {
    crate::config::LOCK_FILE.to_string()
}

/// Python module definition.
#[pymodule]
pub fn telegram_reader(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<ChatTarget>()?;
    m.add_class::<Message>()?;
    m.add_class::<SendResult>()?;

    // Analytics classes (Phase 2.1)
    m.add_class::<SessionMetrics>()?;
    m.add_class::<RetentionMetrics>()?;

    // Chat functions
    m.add_function(wrap_pyfunction!(list_configured_chats, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_chat, m)?)?;
    m.add_function(wrap_pyfunction!(check_session, m)?)?;
    m.add_function(wrap_pyfunction!(get_lock_file_path, m)?)?;

    // Text processing functions (Phase 1)
    m.add_function(wrap_pyfunction!(sanitize_filename, m)?)?;
    m.add_function(wrap_pyfunction!(contains_phone, m)?)?;
    m.add_function(wrap_pyfunction!(detect_conversion, m)?)?;
    m.add_function(wrap_pyfunction!(is_meaningful_message, m)?)?;
    m.add_function(wrap_pyfunction!(parse_iso_datetime, m)?)?;
    m.add_function(wrap_pyfunction!(parse_reactions, m)?)?;
    m.add_function(wrap_pyfunction!(build_message_text, m)?)?;
    m.add_function(wrap_pyfunction!(collect_reactions_summary, m)?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_target_from_channel() {
        let entity = ChatEntity::Channel(12345);
        let target = ChatTarget::from(("test", &entity));

        assert_eq!(target.name, "test");
        assert_eq!(target.kind, "channel");
        assert_eq!(target.id, Some(12345));
        assert!(target.username.is_none());
    }

    #[test]
    fn test_chat_target_from_username() {
        let entity = ChatEntity::Username("testuser".to_string());
        let target = ChatTarget::from(("test", &entity));

        assert_eq!(target.name, "test");
        assert_eq!(target.kind, "username");
        assert!(target.id.is_none());
        assert_eq!(target.username, Some("testuser".to_string()));
    }

    #[test]
    fn test_chat_target_repr() {
        let target = ChatTarget {
            name: "test".to_string(),
            kind: "channel".to_string(),
            id: Some(123),
            username: None,
            title: None,
        };

        let repr = target.__repr__();
        assert!(repr.contains("ChatTarget"));
        assert!(repr.contains("test"));
        assert!(repr.contains("channel"));
    }

    #[test]
    fn test_message_repr() {
        let msg = Message {
            id: 1,
            sender_id: Some(123),
            sender: "Test User".to_string(),
            date: "2024-01-01T00:00:00Z".to_string(),
            text: "Hello world".to_string(),
            reply_to: None,
            views: None,
            forwards: None,
            reactions: HashMap::new(),
            has_media: false,
        };

        let repr = msg.__repr__();
        assert!(repr.contains("Message"));
        assert!(repr.contains("Test User"));
    }

    #[test]
    fn test_message_repr_long_text() {
        let msg = Message {
            id: 1,
            sender_id: Some(123),
            sender: "Test".to_string(),
            date: "2024-01-01".to_string(),
            text: "A".repeat(100),
            reply_to: None,
            views: None,
            forwards: None,
            reactions: HashMap::new(),
            has_media: false,
        };

        let repr = msg.__repr__();
        assert!(repr.contains("..."));
    }

    #[test]
    fn test_get_lock_file_path() {
        let path = get_lock_file_path();
        assert!(path.contains("lock"));
    }

    // Text processing tests (Phase 1)

    #[test]
    fn test_sanitize_filename_basic() {
        assert_eq!(
            sanitize_filename(Some("Hello World"), "fallback", None),
            "Hello_World"
        );
    }

    #[test]
    fn test_sanitize_filename_special_chars() {
        assert_eq!(
            sanitize_filename(Some("Test@#$%Name!"), "fallback", None),
            "TestName"
        );
    }

    #[test]
    fn test_sanitize_filename_empty() {
        assert_eq!(sanitize_filename(None, "fallback", None), "fallback");
        assert_eq!(sanitize_filename(Some(""), "fallback", None), "fallback");
    }

    #[test]
    fn test_sanitize_filename_truncate() {
        let long_name = "A".repeat(100);
        let result = sanitize_filename(Some(&long_name), "fallback", None);
        assert_eq!(result.len(), MAX_FILENAME_LEN);
    }

    #[test]
    fn test_sanitize_filename_custom_max_length() {
        let result = sanitize_filename(Some("LongNameHere"), "fallback", Some(5));
        assert_eq!(result, "LongN");
    }

    #[test]
    fn test_contains_phone_valid() {
        assert!(contains_phone("+0 000 000-00-00"));
        assert!(contains_phone("Мой телефон: 00000000000"));
        assert!(contains_phone("Call me at (000) 000-0000"));
    }

    #[test]
    fn test_contains_phone_invalid() {
        assert!(!contains_phone("Hello world"));
        assert!(!contains_phone("123")); // Too short
        assert!(!contains_phone(""));
    }

    #[test]
    fn test_detect_conversion_phone() {
        assert_eq!(
            detect_conversion("+0 000 000-00-00"),
            Some("phone_shared".to_string())
        );
    }

    #[test]
    fn test_detect_conversion_purchase_intent() {
        assert_eq!(
            detect_conversion("Да, беру это кресло!"),
            Some("purchase_intent".to_string())
        );
        assert_eq!(
            detect_conversion("Хочу купить"),
            Some("purchase_intent".to_string())
        );
    }

    #[test]
    fn test_detect_conversion_checkout() {
        // "Let's" plus a delivery keyword without purchase intent keywords.
        // Uses the Russian address keyword as delivery text and "let's" as action.
        assert_eq!(
            detect_conversion("Давай уточним адрес"),
            Some("checkout_details".to_string())
        );
    }

    #[test]
    fn test_detect_conversion_none() {
        assert_eq!(detect_conversion("Привет, как дела?"), None);
        assert_eq!(detect_conversion(""), None);
    }

    #[test]
    fn test_is_meaningful_message() {
        assert!(is_meaningful_message("Hello world"));
        assert!(!is_meaningful_message("/start"));
        assert!(!is_meaningful_message("  /help  "));
        assert!(!is_meaningful_message(""));
        assert!(!is_meaningful_message("   "));
    }

    #[test]
    fn test_build_message_text_no_media() {
        assert_eq!(build_message_text(Some("Hello"), false, "[Media]"), "Hello");
    }

    #[test]
    fn test_build_message_text_with_media() {
        assert_eq!(
            build_message_text(Some("Caption"), true, "[Media]"),
            "Caption [Media]"
        );
    }

    #[test]
    fn test_build_message_text_media_only() {
        assert_eq!(build_message_text(None, true, "[Media]"), "[Media]");
        assert_eq!(build_message_text(Some(""), true, "[Photo]"), "[Photo]");
    }

    #[test]
    fn test_collect_reactions_summary() {
        let emojis = vec!["👍".to_string(), "❤️".to_string(), "🔥".to_string()];
        assert_eq!(collect_reactions_summary(emojis), "👍❤️🔥");
    }

    #[test]
    fn test_collect_reactions_summary_empty() {
        assert_eq!(collect_reactions_summary(vec![]), "");
    }

    // =========================================================================
    // Phase 2.1: Analytics tests (TDD - RED phase)
    // =========================================================================

    #[test]
    fn test_session_metrics_new_with_defaults() {
        let metrics = SessionMetrics::new(
            1,   // session_id
            100, // user_id
            "test_bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        assert_eq!(metrics.session_id, 1);
        assert_eq!(metrics.user_id, 100);
        assert_eq!(metrics.bot_name, "test_bot");
        assert_eq!(metrics.messages_in, 0);
        assert_eq!(metrics.messages_out, 0);
        assert_eq!(metrics.non_command_in, 0);
        assert!(!metrics.phone_shared);
    }

    #[test]
    fn test_session_metrics_total_messages() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.messages_in = 5;
        metrics.messages_out = 3;

        assert_eq!(metrics.total_messages(), 8);
    }

    #[test]
    fn test_session_metrics_engaged_when_has_meaningful_message() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.non_command_in = 1;

        assert!(metrics.is_engaged());
    }

    #[test]
    fn test_session_metrics_not_engaged_when_no_meaningful_messages() {
        let metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        assert!(!metrics.is_engaged());
    }

    #[test]
    fn test_session_metrics_multi_turn_when_two_or_more_messages() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.non_command_in = 2;

        assert!(metrics.is_multi_turn());
    }

    #[test]
    fn test_retention_metrics_new() {
        let metrics = RetentionMetrics::new(100, 50);

        assert_eq!(metrics.total_users, 100);
        assert_eq!(metrics.d1_base, 50);
        assert_eq!(metrics.d1_returned, 0);
        assert_eq!(metrics.d7_base, 0);
        assert_eq!(metrics.d7_returned, 0);
    }

    #[test]
    fn test_retention_metrics_d1_rate_calculation() {
        let mut metrics = RetentionMetrics::new(100, 50);
        metrics.d1_returned = 25;

        assert!((metrics.d1_rate() - 50.0).abs() < 0.01); // 25/50 = 50%
    }

    #[test]
    fn test_retention_metrics_d1_rate_zero_base() {
        let metrics = RetentionMetrics::new(100, 0);

        assert!((metrics.d1_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_retention_metrics_d7_rate_calculation() {
        let mut metrics = RetentionMetrics::new(100, 50);
        metrics.d7_base = 40;
        metrics.d7_returned = 10;

        assert!((metrics.d7_rate() - 25.0).abs() < 0.01); // 10/40 = 25%
    }

    #[test]
    fn test_attach_message_to_session_incoming_meaningful() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        metrics.attach_message("Hello, I need help!", "incoming");

        assert_eq!(metrics.messages_in, 1);
        assert_eq!(metrics.non_command_in, 1);
        assert_eq!(metrics.messages_out, 0);
    }

    #[test]
    fn test_attach_message_to_session_incoming_command() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        metrics.attach_message("/start", "incoming");

        assert_eq!(metrics.messages_in, 1);
        assert_eq!(metrics.non_command_in, 0); // Command doesn't count as meaningful
    }

    #[test]
    fn test_attach_message_to_session_outgoing() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        metrics.attach_message("Bot response", "outgoing");

        assert_eq!(metrics.messages_in, 0);
        assert_eq!(metrics.messages_out, 1);
    }

    #[test]
    fn test_attach_message_detects_phone() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        metrics.attach_message("Мой телефон +0 000 000-00-00", "incoming");

        assert!(metrics.phone_shared);
    }

    // =========================================================================
    // Phase 2.2: Formatting tests (TDD - RED phase)
    // =========================================================================

    #[test]
    fn test_format_messages_for_llm_single_message() {
        let messages = vec![MessageData {
            sender: "User1".to_string(),
            text: "Hello world".to_string(),
            date: "2024-01-01 10:00".to_string(),
            reactions: None,
        }];

        let result = format_messages_for_llm(&messages, false);

        assert!(result.contains("User1"));
        assert!(result.contains("Hello world"));
        assert!(result.contains("2024-01-01 10:00"));
    }

    #[test]
    fn test_format_messages_for_llm_with_reactions() {
        let messages = vec![MessageData {
            sender: "User1".to_string(),
            text: "Great post!".to_string(),
            date: "2024-01-01 10:00".to_string(),
            reactions: Some("👍❤️".to_string()),
        }];

        let result = format_messages_for_llm(&messages, true);

        assert!(result.contains("👍❤️"));
    }

    #[test]
    fn test_format_messages_for_llm_without_reactions() {
        let messages = vec![MessageData {
            sender: "User1".to_string(),
            text: "Great post!".to_string(),
            date: "2024-01-01 10:00".to_string(),
            reactions: Some("👍❤️".to_string()),
        }];

        let result = format_messages_for_llm(&messages, false);

        assert!(!result.contains("👍"));
    }

    #[test]
    fn test_format_messages_for_llm_multiple_messages() {
        let messages = vec![
            MessageData {
                sender: "User1".to_string(),
                text: "First message".to_string(),
                date: "2024-01-01 10:00".to_string(),
                reactions: None,
            },
            MessageData {
                sender: "User2".to_string(),
                text: "Second message".to_string(),
                date: "2024-01-01 10:05".to_string(),
                reactions: None,
            },
        ];

        let result = format_messages_for_llm(&messages, false);

        assert!(result.contains("First message"));
        assert!(result.contains("Second message"));
    }

    #[test]
    fn test_get_chat_metadata_basic() {
        let messages = vec![
            MessageData {
                sender: "User1".to_string(),
                text: "Hello".to_string(),
                date: "2024-01-01 10:00".to_string(),
                reactions: None,
            },
            MessageData {
                sender: "User2".to_string(),
                text: "Hi there".to_string(),
                date: "2024-01-01 10:05".to_string(),
                reactions: None,
            },
            MessageData {
                sender: "User1".to_string(),
                text: "How are you?".to_string(),
                date: "2024-01-01 10:10".to_string(),
                reactions: None,
            },
        ];

        let metadata = get_chat_metadata(&messages);

        assert_eq!(metadata.message_count, 3);
        assert_eq!(metadata.unique_senders, 2);
    }

    #[test]
    fn test_get_chat_metadata_empty() {
        let messages: Vec<MessageData> = vec![];

        let metadata = get_chat_metadata(&messages);

        assert_eq!(metadata.message_count, 0);
        assert_eq!(metadata.unique_senders, 0);
    }

    // =========================================================================
    // Auto-generated tests (from tests/generate_pyo3_tests.py)
    // =========================================================================

    #[test]
    fn test_format_messages_for_llm_empty() {
        let messages: Vec<MessageData> = vec![];

        assert_eq!(format_messages_for_llm(&messages, false), "");
    }

    #[test]
    fn test_format_messages_for_llm_preserves_order() {
        let messages = vec![
            MessageData {
                sender: "A".to_string(),
                text: "First".to_string(),
                date: "10:00".to_string(),
                reactions: None,
            },
            MessageData {
                sender: "B".to_string(),
                text: "Second".to_string(),
                date: "10:01".to_string(),
                reactions: None,
            },
        ];
        let result = format_messages_for_llm(&messages, false);

        assert!(result.find("First").unwrap() < result.find("Second").unwrap());
    }

    #[test]
    fn test_format_messages_for_llm_reactions_flag_respected() {
        let messages = vec![MessageData {
            sender: "User".to_string(),
            text: "Test".to_string(),
            date: "10:00".to_string(),
            reactions: Some("🔥".to_string()),
        }];

        let with_reactions = format_messages_for_llm(&messages, true);
        let without_reactions = format_messages_for_llm(&messages, false);
        assert!(with_reactions.contains("🔥"));
        assert!(!without_reactions.contains("🔥"));
    }

    #[test]
    fn test_get_chat_metadata_counts_unique_senders() {
        let messages = vec![
            MessageData {
                sender: "A".to_string(),
                text: "1".to_string(),
                date: "".to_string(),
                reactions: None,
            },
            MessageData {
                sender: "A".to_string(),
                text: "2".to_string(),
                date: "".to_string(),
                reactions: None,
            },
            MessageData {
                sender: "B".to_string(),
                text: "3".to_string(),
                date: "".to_string(),
                reactions: None,
            },
        ];
        let metadata = get_chat_metadata(&messages);

        assert_eq!(metadata.unique_senders, 2);
    }

    // =========================================================================
    // Mutation testing: Additional tests to kill surviving mutants
    // =========================================================================

    // Kill mutant: sanitize_filename:83 - replace > with >=
    // Test exact boundary: filename length == max_length should NOT be truncated
    #[test]
    fn test_sanitize_filename_exact_max_length() {
        // Use unique chars to detect if truncation happens incorrectly
        let name = "ABCDEFGHIJ"; // 10 chars
        let result = sanitize_filename(Some(name), "fallback", Some(10));
        // If mutant changes > to >=, this would truncate at 10 chars and return only 10 chars
        // but since we're exactly at boundary, we expect full string
        assert_eq!(result.len(), 10);
        assert_eq!(result, "ABCDEFGHIJ", "exact boundary should not truncate");
    }

    #[test]
    fn test_sanitize_filename_one_over_max_length() {
        let name = "ABCDEFGHIJK"; // 11 chars
        let result = sanitize_filename(Some(name), "fallback", Some(10));
        assert_eq!(result.len(), 10);
        assert_eq!(result, "ABCDEFGHIJ", "one over should truncate");
    }

    #[test]
    fn test_sanitize_filename_boundary_distinguishes_gt_from_gte() {
        // This test specifically kills mutant: > to >=
        // With >: 10 > 10 is false, so no truncation, returns full 10-char string
        // With >=: 10 >= 10 is true, so truncates to 10 chars (same result for same-char strings)
        // We need different chars to detect the difference
        let name = "0000000000"; // exactly 10 chars
        let result = sanitize_filename(Some(name), "fallback", Some(10));

        // The key assertion: with >= mutation, this would still return the input.
        // because [..10] of a 10-char string is the full string.
        // So we need to test behavior difference more carefully.

        // Actually, the mutation doesn't change behavior when len == max_len
        // because result[..max_len] == result when len == max_len.
        // This is a semantically equivalent mutant - we can't kill it.
        assert_eq!(result, "0000000000");
    }

    // Kill mutant: detect_conversion:129 - replace && with ||
    // Test where has_delivery is true but has_action is false
    #[test]
    fn test_detect_conversion_delivery_without_action() {
        // The Russian address keyword is a delivery keyword, but there is no action word.
        assert_eq!(detect_conversion("мой адрес для справки"), None);
    }

    #[test]
    fn test_detect_conversion_action_without_delivery() {
        // The Russian "let's" word is an action word, but there is no delivery keyword.
        assert_eq!(detect_conversion("давай посмотрим фильм"), None);
    }

    // Kill mutant: Message::__repr__:350 - replace > with >=
    // Test exact boundary: text length == 50 should NOT be truncated
    #[test]
    fn test_message_repr_exactly_50_chars() {
        let msg = Message {
            id: 1,
            sender_id: Some(123),
            sender: "Test".to_string(),
            date: "2024-01-01".to_string(),
            text: "A".repeat(50),
            reply_to: None,
            views: None,
            forwards: None,
            reactions: HashMap::new(),
            has_media: false,
        };

        let repr = msg.__repr__();
        assert!(!repr.contains("..."), "50 chars should not be truncated");
        assert!(repr.contains(&"A".repeat(50)));
    }

    #[test]
    fn test_message_repr_51_chars() {
        let msg = Message {
            id: 1,
            sender_id: Some(123),
            sender: "Test".to_string(),
            date: "2024-01-01".to_string(),
            text: "A".repeat(51),
            reply_to: None,
            views: None,
            forwards: None,
            reactions: HashMap::new(),
            has_media: false,
        };

        let repr = msg.__repr__();
        assert!(repr.contains("..."), "51 chars should be truncated");
    }

    // Kill mutant: SendResult::__repr__ - replace with String::new() or "xyzzy"
    #[test]
    fn test_send_result_repr_contains_values() {
        let result = SendResult {
            message_id: 12345,
            date: "2024-01-15".to_string(),
            chat_name: "TestChat".to_string(),
        };

        let repr = result.__repr__();
        assert!(repr.contains("12345"), "repr must contain message_id");
        assert!(repr.contains("TestChat"), "repr must contain chat_name");
        assert!(repr.contains("2024-01-15"), "repr must contain date");
        assert!(repr.contains("SendResult"), "repr must contain type name");
    }

    // Kill mutant: SessionMetrics::is_multi_turn:466 - replace with true
    // Test that exactly 1 non-command message is NOT multi-turn
    #[test]
    fn test_session_metrics_one_message_not_multi_turn() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.non_command_in = 1;

        assert!(
            !metrics.is_multi_turn(),
            "1 message should not be multi-turn"
        );
    }

    // Kill mutant: SessionMetrics::py_total_messages:494 - replace with 0/1/-1
    #[test]
    fn test_session_metrics_py_total_messages_returns_sum() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.messages_in = 3;
        metrics.messages_out = 5;

        let total = metrics.py_total_messages();
        assert_eq!(total, 8, "py_total_messages must return sum of in + out");
    }

    // Kill mutant: SessionMetrics::py_is_engaged:499 - replace with true/false
    #[test]
    fn test_session_metrics_py_is_engaged_false() {
        let metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        assert!(
            !metrics.py_is_engaged(),
            "should not be engaged with 0 non-command messages"
        );
    }

    #[test]
    fn test_session_metrics_py_is_engaged_true() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.non_command_in = 1;

        assert!(
            metrics.py_is_engaged(),
            "should be engaged with 1+ non-command message"
        );
    }

    // Kill mutant: SessionMetrics::py_is_multi_turn:504 - replace with true/false
    #[test]
    fn test_session_metrics_py_is_multi_turn_false() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.non_command_in = 1;

        assert!(
            !metrics.py_is_multi_turn(),
            "should not be multi-turn with 1 message"
        );
    }

    #[test]
    fn test_session_metrics_py_is_multi_turn_true() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.non_command_in = 2;

        assert!(
            metrics.py_is_multi_turn(),
            "should be multi-turn with 2+ messages"
        );
    }

    // Kill mutant: SessionMetrics::py_attach_message:509 - replace with ()
    #[test]
    fn test_session_metrics_py_attach_message_modifies_state() {
        let mut metrics = SessionMetrics::new(
            1,
            100,
            "bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );

        assert_eq!(metrics.messages_in, 0);
        metrics.py_attach_message("Hello!", "incoming");
        assert_eq!(
            metrics.messages_in, 1,
            "py_attach_message must increment messages_in"
        );
        assert_eq!(
            metrics.non_command_in, 1,
            "py_attach_message must increment non_command_in"
        );
    }

    // Kill mutant: SessionMetrics::__repr__:513 - replace with String::new() or "xyzzy"
    #[test]
    fn test_session_metrics_repr_contains_values() {
        let mut metrics = SessionMetrics::new(
            42,
            999,
            "my_bot".to_string(),
            "2024-01-01T10:00:00Z".to_string(),
        );
        metrics.messages_in = 7;
        metrics.messages_out = 3;

        let repr = metrics.__repr__();
        assert!(repr.contains("42"), "repr must contain session_id");
        assert!(repr.contains("999"), "repr must contain user_id");
        assert!(repr.contains("my_bot"), "repr must contain bot_name");
        assert!(
            repr.contains("SessionMetrics"),
            "repr must contain type name"
        );
    }

    // Kill mutant: RetentionMetrics::py_d1_rate:574 - replace with 0.0/1.0/-1.0
    #[test]
    fn test_retention_metrics_py_d1_rate_calculated() {
        let mut metrics = RetentionMetrics::new(100, 80);
        metrics.d1_returned = 40;

        let rate = metrics.py_d1_rate();
        assert!(
            (rate - 50.0).abs() < 0.01,
            "d1_rate should be 50% (40/80*100)"
        );
    }

    // Kill mutant: RetentionMetrics::py_d7_rate:579 - replace with 0.0/1.0/-1.0
    #[test]
    fn test_retention_metrics_py_d7_rate_calculated() {
        let mut metrics = RetentionMetrics::new(100, 80);
        metrics.d7_base = 60;
        metrics.d7_returned = 30;

        let rate = metrics.py_d7_rate();
        assert!(
            (rate - 50.0).abs() < 0.01,
            "d7_rate should be 50% (30/60*100)"
        );
    }

    // Kill mutant: RetentionMetrics::__repr__:583 - replace with String::new() or "xyzzy"
    #[test]
    fn test_retention_metrics_repr_contains_values() {
        let mut metrics = RetentionMetrics::new(500, 400);
        metrics.d1_returned = 200;
        metrics.d7_base = 300;
        metrics.d7_returned = 90;

        let repr = metrics.__repr__();
        assert!(repr.contains("500"), "repr must contain total_users");
        assert!(
            repr.contains("RetentionMetrics"),
            "repr must contain type name"
        );
        // d1_rate = 200/400*100 = 50.0
        assert!(
            repr.contains("50.0"),
            "repr must contain d1_rate percentage"
        );
    }
}
