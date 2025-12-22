//! Credit Expert Bot (Rust)
//!
//! Telegram бот-консультант по банкротству с логированием в MySQL.
//! Переписан с Python-версии `credit_expert_bot.py`.

use std::sync::Arc;

use anyhow::{Context, Result};
use dotenvy::dotenv;
use mysql_async::{prelude::*, Pool};
use regex::Regex;
use telegram_reader::integrations::openai::ChatMessage;
use telegram_reader::integrations::OpenAIClient;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::{Message, User};
use tracing::{error, info};

const BOT_NAME: &str = "Credit_Expert_Bot";

const CREDIT_EXPERT_SYSTEM_PROMPT: &str = r#"Ты — кредитный эксперт Дарья из ФЦБ (Федеральный Центр Банкротства).
ТВОЯ ЦЕЛЬ: Получить номер телефона клиента для консультации.

СТИЛЬ ОБЩЕНИЯ:
- Профессиональный и деловой тон
- Без эмодзи и восклицательных знаков
- Краткие, чёткие ответы (2-4 предложения)
- Уверенный эксперт, а не подруга

ИНСТРУКЦИЯ ПО ДИАЛОГУ:

ШАГ 1: ЗНАКОМСТВО
"Добрый день. Я Дарья, кредитный эксперт ФЦБ. Как могу к вам обращаться?"

ШАГ 2: КВАЛИФИКАЦИЯ
После получения имени спроси: "[Имя], расскажите коротко вашу ситуацию — какой примерно долг и есть ли просрочки?"

Уточняющие вопросы (по одному за раз):
- Коллекторы уже звонят?
- Долги только в МФО или банки тоже?
- Есть ли имущество, которое хотите сохранить?

ШАГ 3: ПЕРЕХОД К ЗВОНКУ
"[Имя], чтобы дать конкретные рекомендации по вашей ситуации, предлагаю созвониться на 10 минут. Консультация бесплатная. Какой номер для связи?"

ОТРАБОТКА ВОЗРАЖЕНИЙ:
- "Расскажите сначала": "Каждый случай индивидуален. На звонке за 10 минут разберём ваш — это эффективнее переписки."
- "Сколько стоит?": "Зависит от суммы долга и ситуации. Консультация бесплатная — расскажу варианты и стоимость."
- "Подумаю": "Хорошо. Учтите, что пени и проценты продолжают расти. Готова ответить, когда решите."
- "Можно в переписке?": "В переписке сложно учесть все нюансы. 10 минут разговора заменят час переписки. Давайте попробуем?"

СТРОГИЕ ПРАВИЛА:
- Никаких эмодзи
- Никаких "подружеских" выражений
- Цель — телефон для консультации
- Один вопрос за раз
- Не затягивай переписку — веди к звонку
"#;

/// Remove emoji from text.
fn strip_emoji(text: &str) -> String {
    let emoji_regex = Regex::new(
        r"[\x{1f600}-\x{1f64f}\x{1f300}-\x{1f5ff}\x{1f680}-\x{1f6ff}\x{1f1e0}-\x{1f1ff}\x{2702}-\x{27b0}\x{24c2}-\x{1f251}\x{1f900}-\x{1f9ff}\x{1fa00}-\x{1fa6f}\x{1fa70}-\x{1faff}\x{2600}-\x{26ff}\x{2700}-\x{27bf}]+"
    ).unwrap();
    emoji_regex.replace_all(text, "").to_string()
}

/// Check if text looks like a valid name.
fn is_valid_name(name: &str) -> bool {
    let name = name.trim();
    if name.len() < 2 || name.len() > 30 {
        return false;
    }

    let name_regex = Regex::new(r"^[А-Яа-яЁёA-Za-z][А-Яа-яЁёA-Za-z\-\s]{1,30}$").unwrap();
    if !name_regex.is_match(name) {
        return false;
    }

    let non_names = [
        "привет",
        "здравствуй",
        "добрый",
        "вечер",
        "день",
        "утро",
        "да",
        "нет",
        "ок",
        "окей",
        "хорошо",
        "ладно",
        "понял",
        "спасибо",
        "пожалуйста",
        "извини",
        "простите",
    ];

    !non_names.contains(&name.to_lowercase().as_str())
}

/// MySQL logger for bot data.
#[derive(Clone)]
struct MySqlLogger {
    pool: Pool,
}

impl MySqlLogger {
    fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;

        conn.exec_drop(
            r#"
            INSERT INTO bot_users (id, username, first_name, last_name, language_code, is_premium, is_bot)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                username = VALUES(username),
                first_name = VALUES(first_name),
                last_name = VALUES(last_name),
                language_code = VALUES(language_code),
                is_premium = VALUES(is_premium),
                last_seen_at = CURRENT_TIMESTAMP
            "#,
            (
                user.id.0 as i64,
                &user.username,
                &user.first_name,
                &user.last_name,
                user.language_code.as_deref(),
                user.is_premium,
                false,
            ),
        )
        .await?;

        info!(user_id = user.id.0, name = %user.first_name, "Saved user");
        Ok(())
    }

    async fn save_message(
        &self,
        user_id: i64,
        message_id: i32,
        text: &str,
        direction: &str,
        reply_to: Option<i32>,
    ) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;

        conn.exec_drop(
            r#"
            INSERT INTO bot_messages
            (telegram_message_id, user_id, bot_name, direction, message_text, reply_to_message_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            (message_id, user_id, BOT_NAME, direction, text, reply_to),
        )
        .await?;

        info!(user_id = user_id, direction = direction, "Saved message");
        Ok(())
    }

    async fn get_or_create_session(&self, user_id: i64) -> Result<i64> {
        let mut conn = self.pool.get_conn().await?;

        // Check for existing active session
        let existing: Option<i64> = conn
            .exec_first(
                r#"
                SELECT id FROM bot_sessions
                WHERE user_id = ? AND bot_name = ? AND is_active = TRUE
                ORDER BY session_start DESC LIMIT 1
                "#,
                (user_id, BOT_NAME),
            )
            .await?;

        if let Some(session_id) = existing {
            return Ok(session_id);
        }

        // End any existing sessions and create new one
        conn.exec_drop(
            r#"
            UPDATE bot_sessions SET is_active = FALSE, session_end = CURRENT_TIMESTAMP
            WHERE user_id = ? AND bot_name = ? AND is_active = TRUE
            "#,
            (user_id, BOT_NAME),
        )
        .await?;

        conn.exec_drop(
            r#"
            INSERT INTO bot_sessions (user_id, bot_name, state)
            VALUES (?, ?, 'greeting')
            "#,
            (user_id, BOT_NAME),
        )
        .await?;

        let session_id: i64 = conn
            .exec_first("SELECT LAST_INSERT_ID()", ())
            .await?
            .unwrap_or(0);

        Ok(session_id)
    }

    async fn get_conversation_history(
        &self,
        user_id: i64,
        limit: u32,
    ) -> Result<Vec<(String, String)>> {
        let mut conn = self.pool.get_conn().await?;

        let rows: Vec<(String, String)> = conn
            .exec(
                r#"
                SELECT direction, message_text
                FROM bot_messages
                WHERE user_id = ? AND bot_name = ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
                (user_id, BOT_NAME, limit),
            )
            .await?;

        // Reverse to get chronological order
        Ok(rows.into_iter().rev().collect())
    }
}

/// Application state shared across handlers.
#[derive(Clone)]
struct AppState {
    db: Arc<MySqlLogger>,
    ai: OpenAIClient,
}

async fn handle_start(bot: Bot, msg: Message, state: AppState) -> Result<()> {
    let user = msg.from().context("No user in message")?;
    let user_id = user.id.0 as i64;

    // Save user
    state.db.save_user(user).await?;

    // Save incoming message
    state
        .db
        .save_message(user_id, msg.id.0, "/start", "incoming", None)
        .await?;

    // Create new session
    state.db.get_or_create_session(user_id).await?;

    // Send greeting
    let greeting = "Добрый день. Я Дарья, кредитный эксперт ФЦБ. Как могу к вам обращаться?";
    let sent = bot.send_message(msg.chat.id, greeting).await?;

    // Save outgoing message
    state
        .db
        .save_message(user_id, sent.id.0, greeting, "outgoing", None)
        .await?;

    Ok(())
}

async fn handle_message(bot: Bot, msg: Message, state: AppState) -> Result<()> {
    let text = match msg.text() {
        Some(t) if !t.starts_with('/') => t,
        _ => return Ok(()),
    };

    let user = msg.from().context("No user in message")?;
    let user_id = user.id.0 as i64;

    // Save user
    state.db.save_user(user).await?;

    // Save incoming message
    state
        .db
        .save_message(user_id, msg.id.0, text, "incoming", None)
        .await?;

    // Ensure session exists
    state.db.get_or_create_session(user_id).await?;

    // Get conversation history
    let history = state.db.get_conversation_history(user_id, 20).await?;

    // Check if name is expected (early in conversation)
    let is_name_expected = history.len() <= 2;

    // Build messages for AI
    let mut messages = vec![ChatMessage {
        role: "system".to_string(),
        content: Some(CREDIT_EXPERT_SYSTEM_PROMPT.to_string()),
    }];

    for (direction, msg_text) in &history {
        let role = if direction == "outgoing" {
            "assistant"
        } else {
            "user"
        };
        messages.push(ChatMessage {
            role: role.to_string(),
            content: Some(msg_text.clone()),
        });
    }

    // Add current message with name validation hint if needed
    let current_msg = if is_name_expected && !is_valid_name(text) {
        format!(
            "{}\n[СИСТЕМА: Это не похоже на имя. Переспроси имя клиента вежливо.]",
            text
        )
    } else {
        text.to_string()
    };

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: Some(current_msg),
    });

    // Get AI response
    let response_text = match state
        .ai
        .chat_completion(messages, "gpt-4o-mini", 0.7, 500)
        .await
    {
        Ok(text) => {
            // Strip emoji and clean up whitespace
            let cleaned = strip_emoji(&text);
            let cleaned = Regex::new(r"\s+")
                .unwrap()
                .replace_all(&cleaned, " ")
                .trim()
                .to_string();
            Regex::new(r" ([.,!?])")
                .unwrap()
                .replace_all(&cleaned, "$1")
                .to_string()
        }
        Err(e) => {
            error!(error = %e, "AI error");
            "Извините, произошла техническая ошибка. Напишите ещё раз.".to_string()
        }
    };

    // Send response
    let sent = bot.send_message(msg.chat.id, &response_text).await?;

    // Save outgoing message
    state
        .db
        .save_message(user_id, sent.id.0, &response_text, "outgoing", None)
        .await?;

    Ok(())
}

fn build_mysql_url() -> String {
    let host = std::env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".to_string());
    let database = std::env::var("MYSQL_DATABASE").unwrap_or_else(|_| "pythorust_tg".to_string());
    let user = std::env::var("MYSQL_USER").unwrap_or_else(|_| "pythorust_tg".to_string());
    let password = std::env::var("MYSQL_PASSWORD").unwrap_or_default();

    format!(
        "mysql://{}:{}@{}:{}/{}",
        user, password, host, port, database
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = std::env::var("CREDIT_EXPERT_BOT_TOKEN")
        .context("CREDIT_EXPERT_BOT_TOKEN not set in environment (.env)")?;

    // Initialize MySQL pool
    let pool = Pool::new(build_mysql_url().as_str());

    // Initialize OpenAI client
    let ai = OpenAIClient::from_env()?;

    let state = AppState {
        db: Arc::new(MySqlLogger::new(pool)),
        ai,
    };

    info!("Starting Credit Expert Bot...");

    let bot = Bot::new(token);

    // Simple dispatcher that handles both /start and regular messages
    Dispatcher::builder(
        bot,
        Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
            let state = state.clone();
            async move {
                if msg.text() == Some("/start") {
                    handle_start(bot, msg, state).await
                } else {
                    handle_message(bot, msg, state).await
                }
            }
        }),
    )
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    Ok(())
}
