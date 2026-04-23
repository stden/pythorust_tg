//! Утилиты для получения списка диалогов.
//!
//! Быстрый просмотр актуальных диалогов с метаданными и экспортом
//! в разные форматы (таблица/JSON/YAML).

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use grammers_client::types::peer::Peer;
use grammers_client::Client;
use serde::Serialize;

const DEFAULT_PARALLEL_FETCH: usize = 8;
const _: () = assert!(DEFAULT_PARALLEL_FETCH > 0);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DialogInfo {
    pub title: String,
    pub id: i64,
    pub chat_type: String,
    pub unread: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Table,
    Json,
    Yaml,
}

impl OutputFormat {
    fn parse(raw: &str) -> Result<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "table" | "pretty" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            other => Err(Error::InvalidArgument(format!(
                "Unsupported format '{}'. Use table|json|yaml",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone)]
struct PendingChat {
    title: String,
    id: i64,
    unread: i32,
    chat_type: String,
    peer: Peer,
}

/// Основная точка входа для CLI.
pub async fn run(limit: usize, format: &str, output: Option<PathBuf>) -> Result<()> {
    let fmt = OutputFormat::parse(format)?;

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let mut dialogs = fetch_dialogs(&client, limit).await?;
    sort_dialogs(&mut dialogs);

    match fmt {
        OutputFormat::Table => print_table(&dialogs),
        OutputFormat::Json => print_json(&dialogs)?,
        OutputFormat::Yaml => print_yaml(&dialogs)?,
    }

    if let Some(path) = output {
        persist(&path, fmt, &dialogs)?;
        println!("Сохранено: {}", path.display());
    }

    Ok(())
}

fn sort_dialogs(dialogs: &mut [DialogInfo]) {
    dialogs.sort_by(|a, b| match (&a.last_message, &b.last_message) {
        (Some(a_ts), Some(b_ts)) => b_ts.cmp(a_ts),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    });
}

fn print_table(dialogs: &[DialogInfo]) {
    println!("Диалоги: {}\n", dialogs.len());
    println!(
        "{:<4} {:<16} {:<9} {:<12} Последнее сообщение | Заголовок",
        "#", "ID", "Тип", "Непрочитано"
    );
    println!("{}", "-".repeat(80));

    for (idx, dialog) in dialogs.iter().enumerate() {
        let ts = dialog
            .last_message
            .map(|d| d.format("%d.%m.%Y %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<4} {:<16} {:<9} {:<12} {} | {}",
            idx + 1,
            dialog.id,
            dialog.chat_type,
            dialog.unread,
            ts,
            dialog.title
        );
    }
}

fn print_json(dialogs: &[DialogInfo]) -> Result<()> {
    let payload = serde_json::to_string_pretty(dialogs)
        .map_err(|e| Error::SerializationError(e.to_string()))?;
    println!("{payload}");
    Ok(())
}

fn print_yaml(dialogs: &[DialogInfo]) -> Result<()> {
    let payload =
        serde_yaml::to_string(dialogs).map_err(|e| Error::SerializationError(e.to_string()))?;
    println!("{payload}");
    Ok(())
}

fn persist(path: &Path, fmt: OutputFormat, dialogs: &[DialogInfo]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = match fmt {
        OutputFormat::Table => render_table(dialogs),
        OutputFormat::Json => serde_json::to_string_pretty(dialogs)
            .map_err(|e| Error::SerializationError(e.to_string()))?,
        OutputFormat::Yaml => {
            serde_yaml::to_string(dialogs).map_err(|e| Error::SerializationError(e.to_string()))?
        }
    };

    fs::write(path, payload)?;
    Ok(())
}

fn render_table(dialogs: &[DialogInfo]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{:<4} {:<16} {:<9} {:<12} {}\n",
        "#", "ID", "Тип", "Непрочитано", "Последнее сообщение | Заголовок"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for (idx, dialog) in dialogs.iter().enumerate() {
        let ts = dialog
            .last_message
            .map(|d| d.format("%d.%m.%Y %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());

        out.push_str(&format!(
            "{:<4} {:<16} {:<9} {:<12} {} | {}\n",
            idx + 1,
            dialog.id,
            dialog.chat_type,
            dialog.unread,
            ts,
            dialog.title
        ));
    }

    out
}

async fn fetch_dialogs(client: &Client, limit: usize) -> Result<Vec<DialogInfo>> {
    let mut chat_activity: Vec<DialogInfo> = Vec::new();
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
            chat_activity.push(DialogInfo {
                title,
                id,
                last_message: Some(last_message.date()),
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
        if count >= limit {
            break;
        }
    }

    if !pending.is_empty() {
        let mut fetched =
            fetch_missing_last_messages(client, pending, DEFAULT_PARALLEL_FETCH).await;
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

fn extract_unread_count(dialog: &grammers_client::types::Dialog) -> i32 {
    match &dialog.raw {
        grammers_tl_types::enums::Dialog::Dialog(d) => d.unread_count,
        grammers_tl_types::enums::Dialog::Folder(folder) => {
            folder.unread_muted_messages_count + folder.unread_unmuted_messages_count
        }
    }
}

async fn fetch_missing_last_messages(
    client: &Client,
    pending: Vec<PendingChat>,
    parallel_fetch: usize,
) -> Vec<DialogInfo> {
    let concurrency = parallel_fetch.max(1);

    stream::iter(pending.into_iter().map(|chat| {
        let client = client.clone();
        async move {
            let mut messages = client.iter_messages(&chat.peer);
            match messages.next().await.transpose() {
                Some(Ok(msg)) => Some(DialogInfo {
                    title: chat.title,
                    id: chat.id,
                    last_message: Some(msg.date()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_parse_table() {
        assert!(matches!(
            OutputFormat::parse("table"),
            Ok(OutputFormat::Table)
        ));
        assert!(matches!(
            OutputFormat::parse("pretty"),
            Ok(OutputFormat::Table)
        ));
        assert!(matches!(
            OutputFormat::parse("TABLE"),
            Ok(OutputFormat::Table)
        ));
    }

    #[test]
    fn output_format_parse_json() {
        assert!(matches!(
            OutputFormat::parse("json"),
            Ok(OutputFormat::Json)
        ));
        assert!(matches!(
            OutputFormat::parse("JSON"),
            Ok(OutputFormat::Json)
        ));
    }

    #[test]
    fn output_format_parse_yaml() {
        assert!(matches!(
            OutputFormat::parse("yaml"),
            Ok(OutputFormat::Yaml)
        ));
        assert!(matches!(OutputFormat::parse("yml"), Ok(OutputFormat::Yaml)));
        assert!(matches!(
            OutputFormat::parse("YAML"),
            Ok(OutputFormat::Yaml)
        ));
    }

    #[test]
    fn output_format_parse_invalid() {
        assert!(OutputFormat::parse("xml").is_err());
        assert!(OutputFormat::parse("csv").is_err());
    }

    #[test]
    fn sort_dialogs_by_last_message() {
        let t1 = Utc::now();
        let t2 = t1 - chrono::Duration::hours(1);

        let mut dialogs = vec![
            DialogInfo {
                title: "Older".to_string(),
                id: 1,
                chat_type: "user".to_string(),
                unread: 0,
                last_message: Some(t2),
            },
            DialogInfo {
                title: "Newer".to_string(),
                id: 2,
                chat_type: "user".to_string(),
                unread: 0,
                last_message: Some(t1),
            },
            DialogInfo {
                title: "NoMessage".to_string(),
                id: 3,
                chat_type: "user".to_string(),
                unread: 0,
                last_message: None,
            },
        ];

        sort_dialogs(&mut dialogs);

        assert_eq!(dialogs[0].title, "Newer");
        assert_eq!(dialogs[1].title, "Older");
        assert_eq!(dialogs[2].title, "NoMessage");
    }

    #[test]
    fn render_table_contains_headers() {
        let dialogs = vec![DialogInfo {
            title: "Test Chat".to_string(),
            id: 123,
            chat_type: "group".to_string(),
            unread: 5,
            last_message: Some(Utc::now()),
        }];

        let table = render_table(&dialogs);
        assert!(table.contains("#"));
        assert!(table.contains("ID"));
        assert!(table.contains("Test Chat"));
        assert!(table.contains("123"));
    }

    #[test]
    fn dialog_info_serialization() {
        let dialog = DialogInfo {
            title: "Test".to_string(),
            id: 1,
            chat_type: "channel".to_string(),
            unread: 10,
            last_message: None,
        };

        let json = serde_json::to_string(&dialog).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("channel"));
        assert!(!json.contains("last_message")); // skip_serializing_if = None
    }
}
