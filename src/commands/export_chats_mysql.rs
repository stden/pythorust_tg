//! Export all Telegram dialogs into MySQL (`telegram_chats` table).
//!
//! Mirrors the behavior of the Python script `export_chats_to_mysql.py` but in Rust.

use chrono::NaiveDateTime;
use grammers_client::types::{peer::Peer, Dialog};
use grammers_tl_types as tl;
use mysql_async::{params, prelude::Queryable, OptsBuilder, Pool};
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

/// Chat metadata ready for database insertion.
#[derive(Debug)]
struct ChatRecord {
    id: i64,
    title: String,
    username: Option<String>,
    chat_type: String,
    members_count: Option<i32>,
    unread_count: i32,
    last_message: Option<NaiveDateTime>,
    is_verified: bool,
    is_restricted: bool,
    is_creator: bool,
    is_admin: bool,
}

/// Main entry point: export dialogs to MySQL.
pub async fn run(max_dialogs: usize) -> Result<()> {
    // Ensure environment is loaded for MySQL credentials.
    let _ = dotenvy::dotenv();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let pool = mysql_pool_from_env()?;
    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| Error::ConnectionError(e.to_string()))?;

    let mut dialogs = client.iter_dialogs();
    let mut processed = 0usize;
    let mut inserted = 0u64;
    let mut updated = 0u64;
    let mut skipped = 0usize;

    info!("Fetching dialogs from Telegram...");

    while let Some(dialog) = dialogs.next().await? {
        if max_dialogs > 0 && processed >= max_dialogs {
            break;
        }

        let record = match ChatRecord::from_dialog(&dialog) {
            Some(record) => record,
            None => {
                skipped += 1;
                continue;
            }
        };

        match insert_chat(&mut conn, &record).await {
            Ok(affected) => {
                processed += 1;

                match affected {
                    1 => inserted += 1,
                    2 => updated += 1,
                    _ => {}
                }

                println!(
                    "[{:<10}] {} (id: {})",
                    record.chat_type, record.title, record.id
                );
            }
            Err(err) => {
                skipped += 1;
                warn!(
                    "Failed to upsert chat {} (id {}): {}",
                    record.title, record.id, err
                );
            }
        }
    }

    info!(
        "Export finished: {} processed ({} inserted, {} updated, {} skipped)",
        processed, inserted, updated, skipped
    );

    conn.disconnect().await.ok();
    Ok(())
}

async fn insert_chat(conn: &mut mysql_async::Conn, chat: &ChatRecord) -> Result<u64> {
    const SQL: &str = r#"
        INSERT INTO telegram_chats
        (id, title, username, chat_type, members_count, unread_count,
         last_message_date, is_verified, is_restricted, is_creator, is_admin)
        VALUES (:id, :title, :username, :chat_type, :members_count, :unread_count,
                :last_message_date, :is_verified, :is_restricted, :is_creator, :is_admin)
        ON DUPLICATE KEY UPDATE
            title = VALUES(title),
            username = VALUES(username),
            members_count = VALUES(members_count),
            unread_count = VALUES(unread_count),
            last_message_date = VALUES(last_message_date),
            is_verified = VALUES(is_verified),
            is_restricted = VALUES(is_restricted),
            is_creator = VALUES(is_creator),
            is_admin = VALUES(is_admin),
            updated_at = CURRENT_TIMESTAMP
    "#;

    conn.exec_drop(
        SQL,
        params! {
            "id" => chat.id,
            "title" => &chat.title,
            "username" => &chat.username,
            "chat_type" => &chat.chat_type,
            "members_count" => chat.members_count,
            "unread_count" => chat.unread_count,
            "last_message_date" => chat.last_message,
            "is_verified" => chat.is_verified,
            "is_restricted" => chat.is_restricted,
            "is_creator" => chat.is_creator,
            "is_admin" => chat.is_admin,
        },
    )
    .await
    .map_err(|e| Error::ConnectionError(e.to_string()))?;

    Ok(conn.affected_rows())
}

impl ChatRecord {
    fn from_dialog(dialog: &Dialog) -> Option<Self> {
        let chat_type = chat_type(&dialog.peer)?;

        Some(Self {
            id: peer_id(&dialog.peer),
            title: chat_title(&dialog.peer),
            username: username(&dialog.peer),
            chat_type: chat_type.to_string(),
            members_count: members_count(&dialog.peer),
            unread_count: extract_unread_count(dialog),
            last_message: dialog
                .last_message
                .as_ref()
                .map(|msg| msg.date().naive_utc()),
            is_verified: is_verified(&dialog.peer),
            is_restricted: is_restricted(&dialog.peer),
            is_creator: is_creator(&dialog.peer),
            is_admin: is_admin(&dialog.peer),
        })
    }
}

fn chat_type(peer: &Peer) -> Option<&'static str> {
    match peer {
        Peer::Channel(channel) => {
            if channel.raw.megagroup || channel.raw.gigagroup {
                Some("supergroup")
            } else {
                Some("channel")
            }
        }
        Peer::Group(group) => {
            if group.is_megagroup() {
                Some("supergroup")
            } else {
                Some("group")
            }
        }
        Peer::User(user) => {
            let is_bot = matches!(&user.raw, tl::enums::User::User(u) if u.bot);
            if is_bot {
                Some("bot")
            } else {
                Some("user")
            }
        }
    }
}

fn chat_title(peer: &Peer) -> String {
    match peer {
        Peer::Channel(c) => c.title().to_string(),
        Peer::Group(g) => g.title().unwrap_or("Unknown").to_string(),
        Peer::User(u) => u.full_name(),
    }
}

fn username(peer: &Peer) -> Option<String> {
    match peer {
        Peer::Channel(c) => c.username().map(|u| u.to_string()),
        Peer::Group(g) => g.username().map(|u| u.to_string()),
        Peer::User(u) => u.username().map(|u| u.to_string()),
    }
}

fn members_count(peer: &Peer) -> Option<i32> {
    match peer {
        Peer::Channel(channel) => channel.raw.participants_count,
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Chat(chat) => Some(chat.participants_count),
            tl::enums::Chat::Channel(chat) => chat.participants_count,
            tl::enums::Chat::ChannelForbidden(_) => None,
            tl::enums::Chat::Forbidden(_) => None,
            tl::enums::Chat::Empty(_) => None,
        },
        Peer::User(_) => None,
    }
}

fn is_verified(peer: &Peer) -> bool {
    match peer {
        Peer::Channel(channel) => channel.raw.verified,
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Channel(chat) => chat.verified,
            _ => false,
        },
        Peer::User(user) => matches!(&user.raw, tl::enums::User::User(u) if u.verified),
    }
}

fn is_restricted(peer: &Peer) -> bool {
    match peer {
        Peer::Channel(channel) => channel.raw.restricted,
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Channel(chat) => chat.restricted,
            _ => false,
        },
        Peer::User(user) => matches!(&user.raw, tl::enums::User::User(u) if u.restricted),
    }
}

fn is_creator(peer: &Peer) -> bool {
    match peer {
        Peer::Channel(channel) => channel.raw.creator,
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Chat(chat) => chat.creator,
            tl::enums::Chat::Channel(chat) => chat.creator,
            _ => false,
        },
        Peer::User(_) => false,
    }
}

fn is_admin(peer: &Peer) -> bool {
    match peer {
        Peer::Channel(channel) => channel.raw.admin_rights.is_some() || channel.raw.creator,
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Chat(chat) => chat.admin_rights.is_some() || chat.creator,
            tl::enums::Chat::Channel(chat) => chat.admin_rights.is_some() || chat.creator,
            _ => false,
        },
        Peer::User(_) => false,
    }
}

fn peer_id(peer: &Peer) -> i64 {
    match peer {
        Peer::Channel(c) => c.raw.id,
        Peer::Group(g) => match &g.raw {
            tl::enums::Chat::Empty(c) => c.id,
            tl::enums::Chat::Chat(c) => c.id,
            tl::enums::Chat::Forbidden(c) => c.id,
            tl::enums::Chat::Channel(c) => c.id,
            tl::enums::Chat::ChannelForbidden(c) => c.id,
        },
        Peer::User(u) => u.raw.id(),
    }
}

fn extract_unread_count(dialog: &Dialog) -> i32 {
    match &dialog.raw {
        tl::enums::Dialog::Dialog(d) => d.unread_count,
        tl::enums::Dialog::Folder(folder) => {
            folder.unread_muted_messages_count + folder.unread_unmuted_messages_count
        }
    }
}

fn mysql_pool_from_env() -> Result<Pool> {
    let host = std::env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: u16 = std::env::var("MYSQL_PORT")
        .unwrap_or_else(|_| "3306".to_string())
        .parse()
        .map_err(|e| Error::ConnectionError(format!("Invalid MYSQL_PORT: {}", e)))?;
    let database = std::env::var("MYSQL_DATABASE").unwrap_or_else(|_| "pythorust_tg".to_string());
    let user = std::env::var("MYSQL_USER").unwrap_or_else(|_| "pythorust_tg".to_string());
    let password = std::env::var("MYSQL_PASSWORD").unwrap_or_default();

    let opts = OptsBuilder::default()
        .ip_or_hostname(host)
        .tcp_port(port)
        .db_name(Some(database))
        .user(Some(user))
        .pass(Some(password));

    Ok(Pool::new(opts))
}
