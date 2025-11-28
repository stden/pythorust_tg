//! LightRAG command helpers (Rust port of `analyze_with_lightrag.py`).
//!
//! - Загружает сообщения из MySQL (`telegram_messages`, `telegram_chats`)
//! - Формирует документы с реакциями/вовлечённостью
//! - Строит лёгкий RAG-индекс на базе внутреннего LightRAG

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use mysql_async::{from_row_opt, params, prelude::Queryable, OptsBuilder, Pool, Row};
use tracing::{debug, info, warn};

use crate::lightrag::{LightRAGConfig, LightRAGRetriever, RetrievalMode};

/// Default number of messages to index.
pub const DEFAULT_LIMIT: usize = 3000;
/// Default batch size for embedding calls.
pub const DEFAULT_BATCH_SIZE: usize = 48;

/// Message fetched from MySQL for RAG.
#[derive(Debug, Clone)]
pub struct RagMessage {
    pub message_id: i64,
    pub chat_title: String,
    pub sender_name: Option<String>,
    pub message_text: String,
    pub date: NaiveDateTime,
    pub reactions_count: Option<u32>,
    pub reactions_raw: Option<String>,
    pub views: Option<u32>,
    pub forwards: Option<u32>,
    pub reply_to_msg_id: Option<i64>,
}

/// Map CLI/ENV string to retrieval mode (keeps Python parity).
pub fn mode_from_str(value: &str) -> RetrievalMode {
    match value.to_lowercase().as_str() {
        "vector" | "naive" => RetrievalMode::VectorOnly,
        "graph" | "local" => RetrievalMode::GraphOnly,
        _ => RetrievalMode::Hybrid, // hybrid / global
    }
}

/// Load messages from MySQL with the same filters as the Python script.
pub async fn load_messages(limit: usize) -> Result<Vec<RagMessage>> {
    let pool = mysql_pool_from_env()?;
    let mut conn = pool
        .get_conn()
        .await
        .context("failed to connect to MySQL")?;

    let sql = r#"
        SELECT
            tm.id as message_id,
            tc.title as chat_title,
            tm.sender_name,
            tm.message_text,
            tm.date,
            tm.reactions_count,
            tm.reactions_json,
            tm.views,
            tm.forwards,
            tm.reply_to_msg_id
        FROM telegram_messages tm
        JOIN telegram_chats tc ON tm.chat_id = tc.id
        WHERE tm.message_text IS NOT NULL
          AND LENGTH(tm.message_text) > 20
          AND tm.message_text NOT LIKE 'http%%'
        ORDER BY tm.reactions_count DESC, tm.date DESC
        LIMIT :limit
    "#;

    let rows: Vec<Row> = conn
        .exec(sql, params! { "limit" => limit as u64 })
        .await
        .context("failed to fetch messages from MySQL")?;

    let mut messages = Vec::new();
    for row in rows {
        match row_to_message(row) {
            Some(mut msg) => {
                msg.message_text = msg.message_text.trim().to_string();
                if msg.message_text.is_empty() {
                    continue;
                }
                messages.push(msg);
            }
            None => warn!("Skipping malformed row from database"),
        }
    }

    conn.disconnect().await.ok();
    info!("Loaded {} messages from MySQL", messages.len());
    Ok(messages)
}

/// Build LightRAG index from messages (batched to reduce API calls).
pub async fn build_retriever(
    messages: &[RagMessage],
    batch_size: usize,
) -> Result<LightRAGRetriever> {
    let mut rag = LightRAGRetriever::new(LightRAGConfig::default());

    if messages.is_empty() {
        return Ok(rag);
    }

    let documents: Vec<(String, String)> = messages
        .iter()
        .map(|m| (format_source(m), format_document(m)))
        .collect();

    let batch_size = batch_size.max(1);
    for (idx, chunk) in documents.chunks(batch_size).enumerate() {
        let before = rag.len();
        let _ = rag.ingest_documents(chunk).await?;
        debug!(
            "Indexed batch {} ({} docs), total chunks: {}",
            idx + 1,
            chunk.len(),
            rag.len()
        );

        if rag.len() == before {
            debug!(
                "Batch {} added no new chunks (possibly empty texts)",
                idx + 1
            );
        }
    }

    info!(
        "LightRAG index ready: {} messages -> {} chunks",
        messages.len(),
        rag.len()
    );

    Ok(rag)
}

/// Format a message into a RAG-ready document.
pub fn format_document(msg: &RagMessage) -> String {
    let engagement = format_engagement(msg);
    let reactions = msg
        .reactions_raw
        .as_ref()
        .filter(|raw| !raw.trim().is_empty());

    let mut doc = format!(
        "Чат: {}\nОтправитель: {}\nДата: {}\nЭто ответ на другое сообщение: {}",
        msg.chat_title,
        msg.sender_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("Аноним"),
        msg.date.format("%Y-%m-%d %H:%M:%S"),
        if msg.reply_to_msg_id.is_some() {
            "Да"
        } else {
            "Нет"
        }
    );

    if let Some(eng) = engagement {
        doc.push_str(&format!("\nВовлечённость: {}", eng));
    }

    doc.push_str(&format!("\nСообщение: {}", msg.message_text));

    if let Some(reactions) = reactions {
        doc.push_str(&format!("\nРеакции: {}", reactions.trim()));
    }

    doc.push_str("\n---");
    doc
}

/// Build a concise source label for the chunk metadata.
pub fn format_source(msg: &RagMessage) -> String {
    format!(
        "{} | {} | {} | msg_id={}",
        msg.chat_title,
        msg.sender_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("Аноним"),
        msg.date.format("%Y-%m-%d %H:%M:%S"),
        msg.message_id
    )
}

fn row_to_message(row: Row) -> Option<RagMessage> {
    let (
        message_id,
        chat_title,
        sender_name,
        message_text,
        date,
        reactions_count,
        reactions_json,
        views,
        forwards,
        reply_to_msg_id,
    ): (
        i64,
        String,
        Option<String>,
        Option<String>,
        NaiveDateTime,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    ) = from_row_opt(row).ok()?;

    let message_text = message_text?;

    Some(RagMessage {
        message_id,
        chat_title,
        sender_name,
        message_text,
        date,
        reactions_count: to_u32(reactions_count),
        reactions_raw: reactions_json,
        views: to_u32(views),
        forwards: to_u32(forwards),
        reply_to_msg_id,
    })
}

fn to_u32(value: Option<i64>) -> Option<u32> {
    value.and_then(|v| u32::try_from(v).ok())
}

fn format_engagement(msg: &RagMessage) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(count) = msg.reactions_count {
        if count > 0 {
            parts.push(format!("Реакций: {}", count));
        }
    }

    if let Some(views) = msg.views {
        if views > 0 {
            parts.push(format!("Просмотров: {}", views));
        }
    }

    if let Some(forwards) = msg.forwards {
        if forwards > 0 {
            parts.push(format!("Репостов: {}", forwards));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn mysql_pool_from_env() -> Result<Pool> {
    let host = std::env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: u16 = std::env::var("MYSQL_PORT")
        .unwrap_or_else(|_| "3306".to_string())
        .parse()
        .context("MYSQL_PORT must be a number")?;
    let database = std::env::var("MYSQL_DATABASE").unwrap_or_else(|_| "pythorust_tg".to_string());
    let user = std::env::var("MYSQL_USER").context("MYSQL_USER not set")?;
    let password = std::env::var("MYSQL_PASSWORD").context("MYSQL_PASSWORD not set")?;

    let builder = OptsBuilder::default()
        .ip_or_hostname(host)
        .tcp_port(port)
        .db_name(Some(database))
        .user(Some(user))
        .pass(Some(password));

    Ok(Pool::new(builder))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_document_with_engagement_and_reactions() {
        let msg = RagMessage {
            message_id: 1,
            chat_title: "Test Chat".into(),
            sender_name: Some("Alice".into()),
            message_text: "Hello world!".into(),
            date: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            reactions_count: Some(5),
            reactions_raw: Some(r#"[{"emoji": "🔥", "count": 3}]"#.into()),
            views: Some(10),
            forwards: Some(2),
            reply_to_msg_id: None,
        };

        let doc = format_document(&msg);
        assert!(doc.contains("Чат: Test Chat"));
        assert!(doc.contains("Отправитель: Alice"));
        assert!(doc.contains("Вовлечённость: Реакций: 5 Просмотров: 10 Репостов: 2"));
        assert!(doc.contains("Реакции: [{\"emoji\": \"🔥\", \"count\": 3}]"));
        assert!(doc.ends_with("---"));
    }

    #[test]
    fn formats_document_without_optional_fields() {
        let msg = RagMessage {
            message_id: 99,
            chat_title: "No Extras".into(),
            sender_name: None,
            message_text: "Content only".into(),
            date: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            reactions_count: Some(0),
            reactions_raw: Some("   ".into()),
            views: None,
            forwards: None,
            reply_to_msg_id: Some(10),
        };

        let doc = format_document(&msg);
        assert!(doc.contains("Отправитель: Аноним"));
        assert!(doc.contains("Это ответ на другое сообщение: Да"));
        assert!(!doc.contains("Вовлечённость"));
        assert!(!doc.contains("Реакции:"));
    }

    #[test]
    fn parses_retrieval_mode_aliases() {
        assert_eq!(mode_from_str("vector"), RetrievalMode::VectorOnly);
        assert_eq!(mode_from_str("NAIVE"), RetrievalMode::VectorOnly);
        assert_eq!(mode_from_str("graph"), RetrievalMode::GraphOnly);
        assert_eq!(mode_from_str("local"), RetrievalMode::GraphOnly);
        assert_eq!(mode_from_str("hybrid"), RetrievalMode::Hybrid);
        assert_eq!(mode_from_str(""), RetrievalMode::Hybrid);
    }

    #[test]
    fn formats_source_with_fallback_sender() {
        let msg = RagMessage {
            message_id: 7,
            chat_title: "Test".into(),
            sender_name: Some("".into()),
            message_text: "hi".into(),
            date: NaiveDateTime::from_timestamp_opt(1_700_000_100, 0).unwrap(),
            reactions_count: None,
            reactions_raw: None,
            views: None,
            forwards: None,
            reply_to_msg_id: None,
        };

        let source = format_source(&msg);
        assert!(source.contains("Test | Аноним"));
        assert!(source.contains("msg_id=7"));
    }

    #[test]
    fn engagement_string_skips_zero_values() {
        let base = RagMessage {
            message_id: 0,
            chat_title: "".into(),
            sender_name: None,
            message_text: "".into(),
            date: NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
            reactions_count: Some(0),
            reactions_raw: None,
            views: Some(0),
            forwards: Some(0),
            reply_to_msg_id: None,
        };

        assert!(format_engagement(&base).is_none());

        let mut with_values = base.clone();
        with_values.reactions_count = Some(3);
        with_values.views = Some(2);
        with_values.forwards = Some(1);

        let engagement = format_engagement(&with_values).expect("engagement");
        assert!(engagement.contains("Реакций: 3"));
        assert!(engagement.contains("Просмотров: 2"));
        assert!(engagement.contains("Репостов: 1"));
    }

    #[test]
    fn converts_optional_i64_to_u32() {
        assert_eq!(to_u32(Some(5)), Some(5));
        assert_eq!(to_u32(Some(-1)), None);
        assert_eq!(to_u32(None), None);
    }
}
