//! List chats command
//!
//! Equivalent to Python's list_chats.py

use std::fs::{self, File};
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use futures::stream::{self, StreamExt};
use grammers_client::types::peer::Peer;
use grammers_client::types::Dialog;
use serde::{Deserialize, Serialize};

const DEFAULT_CACHE_TTL_SECS: u64 = 300; // 5 minutes
const DEFAULT_PARALLEL_FETCH: usize = 8;
const DEFAULT_MAX_DIALOGS: usize = 200;
const DEFAULT_CACHE_PATH: &str = ".cache/list_chats_cache.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatInfo {
    title: String,
    id: i64,
    last_message: DateTime<Utc>,
    unread: i32,
    chat_type: String,
}

/// Filter for chat types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChatFilter {
    All,
    Users,
    Groups,
    Channels,
}

impl ChatFilter {
    fn matches(&self, chat_type: &str) -> bool {
        match self {
            ChatFilter::All => true,
            ChatFilter::Users => chat_type == "user",
            ChatFilter::Groups => chat_type == "group",
            ChatFilter::Channels => chat_type == "channel",
        }
    }
}

#[derive(Debug)]
struct ListChatsSettings {
    cache_enabled: bool,
    cache_path: PathBuf,
    cache_ttl: Duration,
    parallel_fetch: usize,
    max_dialogs: usize,
}

impl ListChatsSettings {
    fn from_env() -> Self {
        Self {
            cache_enabled: !env_flag("LIST_CHATS_DISABLE_CACHE"),
            cache_path: std::env::var("LIST_CHATS_CACHE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(DEFAULT_CACHE_PATH)),
            cache_ttl: parse_env_duration("LIST_CHATS_CACHE_TTL_SECS", DEFAULT_CACHE_TTL_SECS),
            parallel_fetch: parse_env_usize("LIST_CHATS_PARALLEL", DEFAULT_PARALLEL_FETCH, 1),
            max_dialogs: parse_env_usize("LIST_CHATS_MAX_DIALOGS", DEFAULT_MAX_DIALOGS, 1),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DialogCacheFile {
    generated_at: DateTime<Utc>,
    chats: Vec<ChatInfo>,
}

#[derive(Debug)]
struct CachedDialogs {
    generated_at: DateTime<Utc>,
    chats: Vec<ChatInfo>,
}

#[derive(Debug, Clone)]
struct PendingChat {
    title: String,
    id: i64,
    unread: i32,
    chat_type: String,
    peer: Peer,
}

pub async fn run(limit: usize) -> Result<()> {
    run_with_filter(limit, ChatFilter::All).await
}

/// Run with specific chat type filter
pub async fn run_with_filter(limit: usize, filter: ChatFilter) -> Result<()> {
    let settings = ListChatsSettings::from_env();

    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    let maybe_cached = if settings.cache_enabled {
        match load_cache(&settings.cache_path, settings.cache_ttl) {
            Ok(Some(cache)) => {
                let age = Utc::now() - cache.generated_at;
                println!(
                    "Использую кэш диалогов ({} сек назад, {} чатов)",
                    age.num_seconds(),
                    cache.chats.len()
                );
                Some(cache)
            }
            Ok(None) => None,
            Err(err) => {
                eprintln!("Не удалось прочитать кэш диалогов: {}", err);
                None
            }
        }
    } else {
        None
    };

    let mut chat_activity = if let Some(cache) = maybe_cached {
        cache.chats
    } else {
        let fresh_dialogs = fetch_dialogs(&client, &settings).await?;
        if settings.cache_enabled {
            if let Err(err) = save_cache(&settings.cache_path, &fresh_dialogs) {
                eprintln!("Не удалось сохранить кэш диалогов: {}", err);
            }
        }
        fresh_dialogs
    };

    chat_activity = filter_chats(chat_activity, filter);

    // Sort by last message date (newest first)
    chat_activity.sort_by(|a, b| b.last_message.cmp(&a.last_message));

    println!("Наиболее активные чаты:\n");

    for (i, chat) in chat_activity.iter().take(limit).enumerate() {
        println!("{}. {}", i + 1, chat.title);
        println!(
            "   ID: {} | Тип: {} | Непрочитано: {}",
            chat.id, chat.chat_type, chat.unread
        );
        println!(
            "   Последнее сообщение: {}",
            chat.last_message.format("%d.%m.%Y %H:%M")
        );
        println!();
    }

    write_yaml(&chat_activity)?;

    println!(
        "\nИнформация сохранена в chats.yml ({} чатов)",
        chat_activity.len()
    );

    Ok(())
}

fn filter_chats(chats: Vec<ChatInfo>, filter: ChatFilter) -> Vec<ChatInfo> {
    chats
        .into_iter()
        .filter(|chat| filter.matches(&chat.chat_type))
        .collect()
}

fn load_cache(path: &Path, ttl: Duration) -> Result<Option<CachedDialogs>> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(Error::IoError(err)),
    };

    let parsed: DialogCacheFile =
        serde_json::from_str(&data).map_err(|e| Error::SerializationError(e.to_string()))?;

    let ttl = ChronoDuration::from_std(ttl).unwrap_or_else(|_| ChronoDuration::seconds(0));
    if Utc::now() - parsed.generated_at > ttl {
        return Ok(None);
    }

    Ok(Some(CachedDialogs {
        generated_at: parsed.generated_at,
        chats: parsed.chats,
    }))
}

fn save_cache(path: &Path, chats: &[ChatInfo]) -> Result<()> {
    if chats.is_empty() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = DialogCacheFile {
        generated_at: Utc::now(),
        chats: chats.to_vec(),
    };

    let serialized = serde_json::to_string_pretty(&payload)
        .map_err(|e| Error::SerializationError(e.to_string()))?;

    fs::write(path, serialized)?;
    Ok(())
}

fn parse_env_usize(key: &str, default: usize, min: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= min)
        .unwrap_or(default)
}

fn parse_env_duration(key: &str, default_secs: u64) -> Duration {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_secs))
}

fn env_flag(key: &str) -> bool {
    matches!(std::env::var(key), Ok(v) if v == "1" || v.eq_ignore_ascii_case("true"))
}

async fn fetch_dialogs(
    client: &grammers_client::Client,
    settings: &ListChatsSettings,
) -> Result<Vec<ChatInfo>> {
    let mut chat_activity: Vec<ChatInfo> = Vec::new();
    let mut pending: Vec<PendingChat> = Vec::new();
    let mut dialogs = client.iter_dialogs();

    let mut count = 0;
    while let Some(dialog) = dialogs.next().await? {
        let peer = dialog.peer.clone();
        let Some(chat_type) = classify_peer(&peer) else {
            continue;
        };

        let title = chat_title(&peer);
        let id = peer_id(&peer);
        let unread = extract_unread_count(&dialog);

        if let Some(last_message) = dialog.last_message.as_ref() {
            chat_activity.push(ChatInfo {
                title,
                id,
                last_message: last_message.date(),
                unread,
                chat_type: chat_type.to_string(),
            });
        } else {
            pending.push(PendingChat {
                title,
                id,
                unread,
                chat_type: chat_type.to_string(),
                peer,
            });
        }

        count += 1;
        if count >= settings.max_dialogs {
            break;
        }
    }

    if !pending.is_empty() {
        let mut fetched =
            fetch_missing_last_messages(client, pending, settings.parallel_fetch).await;
        chat_activity.append(&mut fetched);
    }

    Ok(chat_activity)
}

fn classify_peer(peer: &Peer) -> Option<&'static str> {
    match peer {
        Peer::Channel(_) => Some("channel"),
        Peer::Group(_) => Some("group"),
        Peer::User(user) => {
            let is_bot = match &user.raw {
                grammers_tl_types::enums::User::User(u) => u.bot,
                grammers_tl_types::enums::User::Empty(_) => false,
            };
            if is_bot {
                None
            } else {
                Some("user")
            }
        }
    }
}

fn chat_title(chat: &Peer) -> String {
    match chat {
        Peer::Channel(c) => c.title().to_string(),
        Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
        Peer::User(u) => u.full_name(),
    }
}

fn peer_id(chat: &Peer) -> i64 {
    match chat {
        Peer::Channel(c) => c.raw.id,
        Peer::Group(g) => match &g.raw {
            grammers_tl_types::enums::Chat::Empty(c) => c.id,
            grammers_tl_types::enums::Chat::Chat(c) => c.id,
            grammers_tl_types::enums::Chat::Forbidden(c) => c.id,
            grammers_tl_types::enums::Chat::Channel(c) => c.id,
            grammers_tl_types::enums::Chat::ChannelForbidden(c) => c.id,
        },
        Peer::User(u) => u.raw.id(),
    }
}

fn extract_unread_count(dialog: &Dialog) -> i32 {
    match &dialog.raw {
        grammers_tl_types::enums::Dialog::Dialog(d) => d.unread_count,
        grammers_tl_types::enums::Dialog::Folder(folder) => {
            folder.unread_muted_messages_count + folder.unread_unmuted_messages_count
        }
    }
}

async fn fetch_missing_last_messages(
    client: &grammers_client::Client,
    pending: Vec<PendingChat>,
    parallel_fetch: usize,
) -> Vec<ChatInfo> {
    let concurrency = parallel_fetch.max(1);

    stream::iter(pending.into_iter().map(|chat| {
        let client = client.clone();
        async move {
            let mut messages = client.iter_messages(&chat.peer);
            match messages.next().await.transpose() {
                Some(Ok(msg)) => Some(ChatInfo {
                    title: chat.title,
                    id: chat.id,
                    last_message: msg.date(),
                    unread: chat.unread,
                    chat_type: chat.chat_type,
                }),
                Some(Err(err)) => {
                    eprintln!(
                        "Не удалось загрузить последнее сообщение для {}: {}",
                        chat.title, err
                    );
                    None
                }
                None => None,
            }
        }
    }))
    .buffer_unordered(concurrency)
    .filter_map(|res| async move { res })
    .collect()
    .await
}

fn write_yaml(chats: &[ChatInfo]) -> Result<()> {
    let mut file = File::create("chats.yml")?;
    writeln!(file, "# Активные чаты Telegram")?;
    if let Some(first) = chats.first() {
        writeln!(
            file,
            "# Обновлено: {}\n",
            first.last_message.format("%d.%m.%Y %H:%M")
        )?;
    }
    writeln!(file, "chats:")?;

    for chat in chats {
        writeln!(file, "  - title: \"{}\"", chat.title)?;
        writeln!(file, "    id: {}", chat.id)?;
        writeln!(file, "    type: {}", chat.chat_type)?;
        writeln!(file, "    unread: {}", chat.unread)?;
        writeln!(
            file,
            "    last_message: \"{}\"",
            chat.last_message.format("%d.%m.%Y %H:%M:%S")
        )?;
        writeln!(file)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn test_chat_filter_all() {
        let filter = ChatFilter::All;
        assert!(filter.matches("user"));
        assert!(filter.matches("group"));
        assert!(filter.matches("channel"));
        assert!(filter.matches("unknown"));
    }

    #[test]
    fn test_chat_filter_users() {
        let filter = ChatFilter::Users;
        assert!(filter.matches("user"));
        assert!(!filter.matches("group"));
        assert!(!filter.matches("channel"));
    }

    #[test]
    fn test_chat_filter_groups() {
        let filter = ChatFilter::Groups;
        assert!(!filter.matches("user"));
        assert!(filter.matches("group"));
        assert!(!filter.matches("channel"));
    }

    #[test]
    fn test_chat_filter_channels() {
        let filter = ChatFilter::Channels;
        assert!(!filter.matches("user"));
        assert!(!filter.matches("group"));
        assert!(filter.matches("channel"));
    }

    #[test]
    fn test_filter_chats() {
        let chats = vec![
            ChatInfo {
                title: "User1".to_string(),
                id: 1,
                last_message: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                unread: 0,
                chat_type: "user".to_string(),
            },
            ChatInfo {
                title: "Group1".to_string(),
                id: 2,
                last_message: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                unread: 5,
                chat_type: "group".to_string(),
            },
            ChatInfo {
                title: "Channel1".to_string(),
                id: 3,
                last_message: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                unread: 10,
                chat_type: "channel".to_string(),
            },
        ];

        let users = filter_chats(chats.clone(), ChatFilter::Users);
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].chat_type, "user");

        let groups = filter_chats(chats.clone(), ChatFilter::Groups);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].chat_type, "group");

        let channels = filter_chats(chats.clone(), ChatFilter::Channels);
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].chat_type, "channel");

        let all = filter_chats(chats, ChatFilter::All);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_parse_env_usize_default() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("TEST_USIZE_VAR");
        assert_eq!(parse_env_usize("TEST_USIZE_VAR", 42, 1), 42);
    }

    #[test]
    fn test_parse_env_usize_valid() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("TEST_USIZE_VAR", "100");
        assert_eq!(parse_env_usize("TEST_USIZE_VAR", 42, 1), 100);
        std::env::remove_var("TEST_USIZE_VAR");
    }

    #[test]
    fn test_parse_env_usize_below_min() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("TEST_USIZE_VAR", "0");
        assert_eq!(parse_env_usize("TEST_USIZE_VAR", 42, 1), 42);
        std::env::remove_var("TEST_USIZE_VAR");
    }

    #[test]
    fn test_parse_env_usize_invalid() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("TEST_USIZE_VAR", "invalid");
        assert_eq!(parse_env_usize("TEST_USIZE_VAR", 42, 1), 42);
        std::env::remove_var("TEST_USIZE_VAR");
    }

    #[test]
    fn test_parse_env_duration_default() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("TEST_DURATION_VAR");
        let duration = parse_env_duration("TEST_DURATION_VAR", 300);
        assert_eq!(duration, Duration::from_secs(300));
    }

    #[test]
    fn test_parse_env_duration_valid() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("TEST_DURATION_VAR", "600");
        let duration = parse_env_duration("TEST_DURATION_VAR", 300);
        assert_eq!(duration, Duration::from_secs(600));
        std::env::remove_var("TEST_DURATION_VAR");
    }

    #[test]
    fn test_env_flag_true() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("TEST_FLAG", "1");
        assert!(env_flag("TEST_FLAG"));
        std::env::remove_var("TEST_FLAG");

        std::env::set_var("TEST_FLAG", "true");
        assert!(env_flag("TEST_FLAG"));
        std::env::remove_var("TEST_FLAG");

        std::env::set_var("TEST_FLAG", "TRUE");
        assert!(env_flag("TEST_FLAG"));
        std::env::remove_var("TEST_FLAG");
    }

    #[test]
    fn test_env_flag_false() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("TEST_FLAG");
        assert!(!env_flag("TEST_FLAG"));

        std::env::set_var("TEST_FLAG", "0");
        assert!(!env_flag("TEST_FLAG"));
        std::env::remove_var("TEST_FLAG");

        std::env::set_var("TEST_FLAG", "false");
        assert!(!env_flag("TEST_FLAG"));
        std::env::remove_var("TEST_FLAG");
    }

    #[test]
    fn test_cache_serialization() {
        use tempfile::NamedTempFile;

        let chats = vec![ChatInfo {
            title: "Test Chat".to_string(),
            id: 123,
            last_message: Utc.with_ymd_and_hms(2024, 12, 15, 10, 30, 0).unwrap(),
            unread: 5,
            chat_type: "group".to_string(),
        }];

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Save cache
        save_cache(path, &chats).unwrap();

        // Load cache with long TTL
        let loaded = load_cache(path, Duration::from_secs(3600)).unwrap();
        assert!(loaded.is_some());

        let cached = loaded.unwrap();
        assert_eq!(cached.chats.len(), 1);
        assert_eq!(cached.chats[0].title, "Test Chat");
        assert_eq!(cached.chats[0].id, 123);
        assert_eq!(cached.chats[0].unread, 5);
        assert_eq!(cached.chats[0].chat_type, "group");
    }

    #[test]
    fn test_cache_expiration() {
        use tempfile::NamedTempFile;

        let chats = vec![ChatInfo {
            title: "Test".to_string(),
            id: 1,
            last_message: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            unread: 0,
            chat_type: "user".to_string(),
        }];

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        save_cache(path, &chats).unwrap();

        // Load with 0 TTL (expired)
        let loaded = load_cache(path, Duration::from_secs(0)).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_save_empty_cache() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Should succeed but not write anything
        assert!(save_cache(path, &[]).is_ok());

        // File should not exist or be empty
        let metadata = std::fs::metadata(path);
        assert!(metadata.is_err() || metadata.unwrap().len() == 0);
    }

    #[test]
    fn test_load_cache_missing_file_returns_none() {
        let temp_dir = tempfile::tempdir().unwrap();
        let missing_path = temp_dir.path().join("missing_cache.json");

        let loaded = load_cache(&missing_path, Duration::from_secs(3600)).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_load_cache_invalid_json_returns_error() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), "not-json").unwrap();

        let err = load_cache(temp_file.path(), Duration::from_secs(3600)).unwrap_err();
        assert!(matches!(err, Error::SerializationError(_)));
    }

    #[test]
    fn test_save_cache_creates_parent_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache_path = temp_dir.path().join("nested").join("cache.json");

        let chats = vec![ChatInfo {
            title: "Test".to_string(),
            id: 1,
            last_message: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            unread: 0,
            chat_type: "group".to_string(),
        }];

        save_cache(&cache_path, &chats).unwrap();
        assert!(cache_path.exists());
    }
}
