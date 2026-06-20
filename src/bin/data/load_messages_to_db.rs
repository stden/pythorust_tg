//! Load messages from Telegram chats into MySQL database.
//!
//! Usage: cargo run --bin load_messages_to_db -- [--limit 100] [--days 7] [--chat-id <id>]

use anyhow::Result;
use chrono::{Duration, Utc};
use clap::Parser;
use dotenvy::dotenv;
use std::env;
use telegram_reader::mysql_async::{prelude::*, Pool, Value};
use tracing::info;

use telegram_reader::chat::peer_name;
use telegram_reader::get_client;
use telegram_reader::grammers_client::types::{Media, Peer};
use telegram_reader::session::SessionLock;

#[derive(Parser)]
struct Args {
    /// Message limit per chat
    #[arg(long, default_value = "100")]
    limit: usize,

    /// Filter messages by last N days
    #[arg(long)]
    days: Option<i64>,

    /// Filter by specific chat ID
    #[arg(long)]
    chat_id: Option<i64>,

    /// Maximum number of chats to process
    #[arg(long, default_value = "50")]
    max_chats: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

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
        CREATE TABLE IF NOT EXISTS telegram_messages (
            id BIGINT,
            chat_id BIGINT,
            sender_id BIGINT,
            sender_name VARCHAR(255),
            message_text TEXT,
            date DATETIME,
            reply_to_msg_id BIGINT,
            forward_from_id BIGINT,
            views INT,
            forwards INT,
            reactions_count INT DEFAULT 0,
            reactions_json TEXT,
            media_type VARCHAR(50),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (id, chat_id),
            INDEX idx_chat_id (chat_id),
            INDEX idx_date (date)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#,
    )
    .await?;

    // Connect to Telegram
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    info!("Connected to Telegram");

    let min_date = args.days.map(|d| Utc::now() - Duration::days(d));

    // Get all dialogs
    let mut dialogs = client.iter_dialogs();
    let mut processed_chats = 0;
    let mut total_inserted = 0;

    while let Some(dialog) = dialogs.next().await? {
        if matches!(dialog.peer, Peer::User(_)) {
            continue;
        }

        let chat_id = match &dialog.peer {
            Peer::User(u) => u.raw.id() as i64,
            Peer::Channel(c) => c.raw.id,
            Peer::Group(g) => match &g.raw {
                grammers_tl_types::enums::Chat::Chat(c) => c.id,
                grammers_tl_types::enums::Chat::Channel(c) => c.id,
                _ => continue,
            },
        };

        if let Some(target_id) = args.chat_id {
            if chat_id != target_id {
                continue;
            }
        }

        if processed_chats >= args.max_chats {
            break;
        }

        let chat_title = peer_name(&dialog.peer);
        info!("📥 Processing chat: {} (id={})", chat_title, chat_id);

        let mut messages = client.iter_messages(&dialog.peer).limit(args.limit);
        let mut inserted = 0;

        while let Some(m) = messages.next().await? {
            if let Some(min) = min_date {
                if m.date() < min {
                    break;
                }
            }

            let sender_name = if let Some(sender) = m.sender() {
                peer_name(sender)
            } else {
                "Unknown".to_string()
            };

            let media_type = match m.media() {
                Some(Media::Photo(_)) => Some("photo"),
                Some(Media::Document(_)) => Some("document"),
                Some(_) => Some("other"),
                None => None,
            };

            // Simplified reactions for now
            let reactions_count = 0;
            let reactions_json: Option<String> = None;

            let insert_query = r#"
                INSERT INTO telegram_messages
                (id, chat_id, sender_id, sender_name, message_text, date,
                 reply_to_msg_id, forward_from_id, views, forwards,
                 reactions_count, reactions_json, media_type)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    message_text = VALUES(message_text),
                    views = VALUES(views),
                    forwards = VALUES(forwards),
                    reactions_count = VALUES(reactions_count),
                    reactions_json = VALUES(reactions_json)
            "#;

            let date_str = m.date().format("%Y-%m-%d %H:%M:%S").to_string();
            let sender_id = m
                .sender()
                .map(|s| match s {
                    Peer::User(u) => u.raw.id() as i64,
                    Peer::Channel(c) => c.raw.id,
                    Peer::Group(g) => match &g.raw {
                        grammers_tl_types::enums::Chat::Chat(c) => c.id,
                        _ => 0,
                    },
                })
                .unwrap_or(0);

            match conn
                .exec_drop(
                    insert_query,
                    // 13 positional params: mysql_async only impls Params: From for
                    // tuples up to 12, so build a Vec<Value> explicitly.
                    vec![
                        Value::from(m.id() as i64),
                        Value::from(chat_id),
                        Value::from(sender_id),
                        Value::from(sender_name.clone()),
                        Value::from(m.text().to_string()),
                        Value::from(date_str.clone()),
                        Value::from(m.reply_to_message_id().map(|id| id as i64)),
                        Value::from(None::<i64>), // forward_from_id simplified
                        Value::from(m.view_count().unwrap_or(0)),
                        Value::from(0i32), // forwards: no grammers accessor (simplified)
                        Value::from(reactions_count),
                        Value::from(reactions_json.clone()),
                        Value::from(media_type.map(|s| s.to_string())),
                    ],
                )
                .await
            {
                Ok(_) => inserted += 1,
                Err(e) => eprintln!("  ⚠️ Error inserting message {}: {}", m.id(), e),
            }
        }

        info!("  ✅ Loaded: {} messages", inserted);
        total_inserted += inserted;
        processed_chats += 1;
    }

    println!(
        "\n🎉 Done! Total loaded: {} messages from {} chats",
        total_inserted, processed_chats
    );

    drop(conn);
    pool.disconnect().await?;

    Ok(())
}
