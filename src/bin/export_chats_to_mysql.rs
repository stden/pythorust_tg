//! Export Telegram chats list to MySQL database.
//!
//! Usage:
//!   cargo run --bin export_chats_to_mysql

use anyhow::Result;
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use grammers_client::types::peer::Peer;
use mysql_async::{prelude::*, Pool};
use std::env;
use tracing::info;

use telegram_reader::get_client;
use telegram_reader::session::SessionLock;

/// Get chat type from peer.
fn get_chat_type(peer: &Peer) -> &'static str {
    match peer {
        Peer::User(u) => {
            if let grammers_tl_types::enums::User::User(user) = &u.raw {
                if user.bot {
                    return "bot";
                }
            }
            "user"
        }
        Peer::Group(_) => "group",
        Peer::Channel(c) => {
            if c.raw.megagroup {
                "supergroup"
            } else {
                "channel"
            }
        }
    }
}

/// Chat data for export.
struct ChatData {
    id: i64,
    title: String,
    username: Option<String>,
    chat_type: String,
    members_count: Option<i32>,
    unread_count: i32,
    last_message_date: Option<DateTime<Utc>>,
    is_verified: bool,
    is_restricted: bool,
    is_creator: bool,
    is_admin: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // MySQL connection
    let mysql_url = env::var("DATABASE_URL").or_else(|_| {
        let host = env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".to_string());
        let db = env::var("MYSQL_DATABASE").unwrap_or_else(|_| "pythorust_tg".to_string());
        let user = env::var("MYSQL_USER").unwrap_or_else(|_| "pythorust_tg".to_string());
        let password = env::var("MYSQL_PASSWORD")?;
        Ok::<_, env::VarError>(format!(
            "mysql://{}:{}@{}:{}/{}",
            user, password, host, port, db
        ))
    })?;

    let pool = Pool::new(mysql_url.as_str());
    let mut conn = pool.get_conn().await?;

    // Create table if not exists
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS telegram_chats (
            id BIGINT PRIMARY KEY,
            title VARCHAR(255),
            username VARCHAR(255),
            chat_type VARCHAR(50),
            members_count INT,
            unread_count INT DEFAULT 0,
            last_message_date DATETIME,
            is_verified BOOLEAN DEFAULT FALSE,
            is_restricted BOOLEAN DEFAULT FALSE,
            is_creator BOOLEAN DEFAULT FALSE,
            is_admin BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            INDEX idx_chat_type (chat_type),
            INDEX idx_username (username)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#,
    )
    .await?;

    // Connect to Telegram
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    info!("Connected to Telegram");

    // Get all dialogs
    let mut dialogs = client.iter_dialogs();
    let mut chats: Vec<ChatData> = Vec::new();

    while let Some(dialog) = dialogs.next().await? {
        let peer = &dialog.peer;
        let chat_id = match peer {
            Peer::User(u) => u.raw.id() as i64,
            Peer::Channel(c) => c.raw.id,
            Peer::Group(g) => match &g.raw {
                grammers_tl_types::enums::Chat::Chat(c) => c.id,
                grammers_tl_types::enums::Chat::Channel(c) => c.id,
                _ => continue,
            },
        };

        let title = peer.name().unwrap_or("Unknown").to_string();

        let username = match peer {
            Peer::User(u) => {
                if let grammers_tl_types::enums::User::User(user) = &u.raw {
                    user.username.clone()
                } else {
                    None
                }
            }
            Peer::Channel(c) => c.raw.username.clone(),
            _ => None,
        };

        let chat_type = get_chat_type(peer).to_string();

        let members_count = match peer {
            Peer::Channel(c) => c.raw.participants_count,
            Peer::Group(g) => {
                if let grammers_tl_types::enums::Chat::Chat(c) = &g.raw {
                    Some(c.participants_count)
                } else {
                    None
                }
            }
            _ => None,
        };

        let (is_verified, is_restricted, is_creator) = match peer {
            Peer::Channel(c) => (c.raw.verified, c.raw.restricted, c.raw.creator),
            _ => (false, false, false),
        };

        let is_admin = match peer {
            Peer::Channel(c) => c.raw.admin_rights.is_some(),
            _ => false,
        };

        chats.push(ChatData {
            id: chat_id,
            title,
            username,
            chat_type,
            members_count: members_count.map(|c| c as i32),
            unread_count: 0,         // Would need dialog.unread_count if available
            last_message_date: None, // Would need last message from dialog
            is_verified,
            is_restricted,
            is_creator,
            is_admin,
        });

        print!(
            "  [{:10}] {:40} (ID: {})\n",
            chats.last().unwrap().chat_type,
            &chats.last().unwrap().title[..chats.last().unwrap().title.len().min(40)],
            chat_id
        );
    }

    println!("\nFound {} dialogs", chats.len());

    // Insert into MySQL
    let insert_query = r#"
        INSERT INTO telegram_chats
        (id, title, username, chat_type, members_count, unread_count,
         last_message_date, is_verified, is_restricted, is_creator, is_admin)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
        title = VALUES(title),
        username = VALUES(username),
        members_count = VALUES(members_count),
        unread_count = VALUES(unread_count),
        last_message_date = VALUES(last_message_date),
        is_verified = VALUES(is_verified),
        is_restricted = VALUES(is_restricted),
        is_creator = VALUES(is_creator),
        is_admin = VALUES(is_admin)
    "#;

    let mut inserted = 0;
    let mut updated = 0;
    let mut errors = 0;

    for chat in &chats {
        let last_msg_str = chat
            .last_message_date
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        match conn
            .exec_drop(
                insert_query,
                (
                    chat.id,
                    &chat.title,
                    &chat.username,
                    &chat.chat_type,
                    chat.members_count,
                    chat.unread_count,
                    &last_msg_str,
                    chat.is_verified,
                    chat.is_restricted,
                    chat.is_creator,
                    chat.is_admin,
                ),
            )
            .await
        {
            Ok(_) => {
                // MySQL returns 1 for insert, 2 for update
                inserted += 1;
            }
            Err(e) => {
                errors += 1;
                eprintln!("ERROR {}: {}", chat.title, e);
            }
        }
    }

    // Since we can't easily distinguish insert vs update with exec_drop,
    // we'll report total successful
    updated = chats.len() - inserted - errors;

    println!("\n{}", "=".repeat(50));
    println!("Export complete!");
    println!("  Processed: {}", chats.len());
    println!("  Errors:    {}", errors);

    drop(conn);
    pool.disconnect().await?;

    Ok(())
}
