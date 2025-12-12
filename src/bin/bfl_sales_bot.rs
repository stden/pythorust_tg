//! BFL Sales Bot (Rust)
//!
//! Telegram бот-продавец массажных кресел (Relaxio) с логированием в MySQL
//! и A/B тестированием промптов. Переписан с Python-версии `bfl_sales_bot.py`.

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use dotenvy::dotenv;
use mysql_async::{prelude::*, Pool};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use regex::Regex;
use telegram_reader::integrations::openai::ChatMessage;
use telegram_reader::integrations::OpenAIClient;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::{Message, User};
use tracing::{error, info};

const BOT_NAME: &str = "BFL_sales_bot";

const SALES_SYSTEM_PROMPT: &str = r#"Ты - профессиональный консультант по массажным креслам компании Relaxio.

ТВОЯ ЦЕЛЬ: Помочь клиенту выбрать массажное кресло и довести до покупки.

СТИЛЬ ОБЩЕНИЯ:
- Дружелюбный, но профессиональный
- Задавай уточняющие вопросы (не более 3 за раз)
- Используй emoji умеренно
- Отвечай кратко, по делу

ЭТАПЫ ПРОДАЖИ:
1. Выявление потребностей (задачи, частота использования, бюджет)
2. Уточнение деталей (рост/вес, проблемы со здоровьем)
3. Презентация подходящей модели
4. Работа с возражениями
5. Закрытие сделки (город, способ оплаты, контакт)

ЛИНЕЙКА ПРОДУКТОВ:
- Relaxio Premium R5: до 120 тыс, базовый 3D-массаж
- Relaxio Premium R7: до 200 тыс, 4D-массаж, нулевая гравитация, Bluetooth
- Relaxio Premium R9: до 300 тыс, топовый 4D, растяжка, все функции

Всегда старайся продать модель в рамках бюджета клиента."#;

const FAST_CLOSE_PROMPT: &str = r#"Ты — sales closer по массажным креслам Relaxio.
Цель: за 3-5 сообщений вывести клиента на выбор модели и договориться о доставке/оплате.

Правила:
- Короткие ответы 1-3 предложения, без воды.
- Всегда заканчивай шагом: город + телефон для доставки, выбор цвета, подтверждение бюджета.
- Давай 2 модели: базовую в бюджете и +1 уровень с четкой выгодой.
- Возражения "дорого/подумаю" закрывай формулой: боль → выгода → гарантия/рассрочка → CTA.
- Не задавай больше 1 уточняющего вопроса за раз.

ЛИНЕЙКА ПРОДУКТОВ:
- R5: до 120 тыс, базовый 3D-массаж.
- R7: до 200 тыс, 4D + нулевая гравитация + Bluetooth.
- R9: до 300 тыс, топовый 4D, растяжка, прогрев, полный функционал."#;

const STORY_PROOF_PROMPT: &str = r#"Ты — консультант Relaxio, работаешь по SPIN + social proof.
Цель: понять задачи, дать мини-кейс и закрыть на оплату/доставку.

Правила:
- Отзеркаливай ответ клиента и добавляй микро-кейс (кто купил, что получил).
- Сообщения до 3 предложений, дружелюбные, emoji умеренно.
- В каждом ответе есть CTA: город/телефон для доставки или время для демонстрации.
- Если бюджет высокий, предлагай апсейл (прогрев, растяжка, массаж ног).
- Не выдумывай цены, опирайся на линейку R5/R7/R9 и реальную пользу."#;

#[derive(Clone)]
struct PromptVariant {
    name: String,
    prompt: String,
    description: String,
    weight: f32,
    temperature: f32,
    model: Option<String>,
}

#[derive(Clone)]
struct AppState {
    db: Arc<MySqlLogger>,
    ab: Arc<AbTestManager>,
    ai: OpenAIClient,
    default_model: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = std::env::var("BFL_SALES_BOT_TOKEN")
        .context("BFL_SALES_BOT_TOKEN not set in environment (.env)")?;
    let api_id: i32 = std::env::var("TELEGRAM_API_ID")
        .context("TELEGRAM_API_ID not set")?
        .parse()
        .context("Invalid TELEGRAM_API_ID")?;
    let api_hash = std::env::var("TELEGRAM_API_HASH").context("TELEGRAM_API_HASH not set")?;
    let experiment_name =
        std::env::var("BFL_PROMPT_EXPERIMENT").unwrap_or_else(|_| "bfl_prompt_ab".to_string());
    let default_model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    // Keep API credentials to align with other tooling (init_session, etc.)
    let _ = (api_id, api_hash);

    let db = Arc::new(MySqlLogger::new(BOT_NAME).await?);
    let ab = Arc::new(
        AbTestManager::new(
            db.clone(),
            BOT_NAME.to_string(),
            experiment_name,
            prompt_variants(),
        )
        .await?,
    );
    let ai = OpenAIClient::from_env()?;

    let state = Arc::new(AppState {
        db,
        ab,
        ai,
        default_model,
    });

    let bot = Bot::new(token);

    let handler = dptree::entry().branch(Update::filter_message().endpoint({
        move |bot: Bot, msg: Message, state: Arc<AppState>| async move {
            if let Err(err) = handle_message(bot, state, msg).await {
                error!("Handler error: {err:?}");
            }
            Ok::<_, teloxide::RequestError>(())
        }
    }));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_message(bot: Bot, state: Arc<AppState>, msg: Message) -> Result<()> {
    let text = match msg.text() {
        Some(t) => t.trim(),
        None => return Ok(()),
    };

    if text.starts_with('/') {
        if text == "/start" {
            handle_start(&bot, state, &msg).await?;
        }
        return Ok(());
    }

    let user = msg.from();
    let user_id = user.map(|u| u.id.0 as i64).unwrap_or_else(|| msg.chat.id.0);

    state.db.save_user(user).await?;

    state
        .db
        .log_message(
            msg.id.0 as i64,
            user_id,
            "incoming",
            text,
            msg.reply_to_message().map(|m| m.id.0 as i64),
        )
        .await?;

    let session_id = match state.db.get_active_session(user_id).await? {
        Some(id) => id,
        None => state.db.create_session(user_id).await?,
    };

    let variant = match state.ab.get_or_assign_variant(user_id, session_id).await {
        Ok(v) => v,
        Err(err) => {
            error!("AB assignment failed: {err}");
            state.ab.fallback_variant()
        }
    };

    state
        .ab
        .detect_and_mark_conversion(session_id, text)
        .await?;

    let history = state.db.conversation_history(user_id, 20).await?;

    let mut messages = Vec::with_capacity(history.len() + 2);
    messages.push(ChatMessage {
        role: "system".to_string(),
        content: Some(variant.prompt.clone()),
    });

    for h in history {
        let role = if h.direction == "outgoing" {
            "assistant"
        } else {
            "user"
        };
        messages.push(ChatMessage {
            role: role.to_string(),
            content: Some(h.message_text),
        });
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: Some(text.to_string()),
    });

    let model = variant
        .model
        .as_deref()
        .unwrap_or_else(|| state.default_model.as_str());

    let ai_reply = match state
        .ai
        .chat_completion(messages, model, variant.temperature, 800)
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            error!("OpenAI error: {err}");
            "Извините, произошла ошибка. Попробуйте ещё раз.".to_string()
        }
    };

    let sent = bot
        .send_message(msg.chat.id, ai_reply.clone())
        .reply_to_message_id(msg.id)
        .await?;

    state
        .db
        .log_message(
            sent.id.0 as i64,
            user_id,
            "outgoing",
            &ai_reply,
            Some(msg.id.0 as i64),
        )
        .await?;

    Ok(())
}

async fn handle_start(bot: &Bot, state: Arc<AppState>, msg: &Message) -> Result<()> {
    let user = msg.from();
    let user_id = user.map(|u| u.id.0 as i64).unwrap_or_else(|| msg.chat.id.0);

    state.db.save_user(user).await?;

    state
        .db
        .log_message(msg.id.0 as i64, user_id, "incoming", "/start", None)
        .await?;

    let session_id = match state.db.create_session(user_id).await {
        Ok(id) => id,
        Err(err) => {
            error!("Failed to create session: {err}");
            0
        }
    };
    let _ = state
        .ab
        .get_or_assign_variant(user_id, session_id)
        .await
        .map_err(|err| {
            error!("AB assignment failed: {err}");
            err
        });

    let first_name = user
        .map(|u| u.first_name.clone())
        .unwrap_or_else(|| "друг".to_string());

    let greeting = format!(
        "Привет, {first_name}!\nЯ помогу выбрать массажное кресло под ваши задачи за пару минут."
    );
    let questions = "Чтобы подобрать точный вариант, подскажите:\n- для каких задач ищете кресло (расслабление, здоровье, подарок)?\n- как часто планируете использовать?\n- есть ли ориентир по бюджету?";
    let recommendation = "Пока вы отвечаете, предварительно рекомендую линейку Relaxio Premium: компактные модели с 4D-массажем и прогревом, подходят для ежедневного использования. Подберу точную модель, когда узнаю ваши вводные.";

    let m1 = send_and_log(bot, &state, msg, user_id, &greeting).await?;
    let m2 = send_and_log(bot, &state, msg, user_id, questions).await?;
    let m3 = send_and_log(bot, &state, msg, user_id, recommendation).await?;

    info!(
        "Sent onboarding messages {:?} {:?} {:?} to user {}",
        m1.id, m2.id, m3.id, user_id
    );

    Ok(())
}

async fn send_and_log(
    bot: &Bot,
    state: &AppState,
    msg: &Message,
    user_id: i64,
    text: &str,
) -> Result<Message> {
    let sent = bot
        .send_message(msg.chat.id, text.to_string())
        .reply_to_message_id(msg.id)
        .await?;

    state
        .db
        .log_message(
            sent.id.0 as i64,
            user_id,
            "outgoing",
            text,
            Some(msg.id.0 as i64),
        )
        .await?;
    Ok(sent)
}

#[derive(Clone)]
struct MySqlLogger {
    pool: Pool,
    bot_name: String,
}

impl MySqlLogger {
    async fn new(bot_name: &str) -> Result<Self> {
        let host = std::env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = std::env::var("MYSQL_PORT")
            .unwrap_or_else(|_| "3306".to_string())
            .parse()
            .unwrap_or(3306);
        let database =
            std::env::var("MYSQL_DATABASE").unwrap_or_else(|_| "pythorust_tg".to_string());
        let user = std::env::var("MYSQL_USER").unwrap_or_else(|_| "pythorust_tg".to_string());
        let password = std::env::var("MYSQL_PASSWORD").unwrap_or_default();

        let opts = mysql_async::OptsBuilder::default()
            .ip_or_hostname(host)
            .tcp_port(port)
            .db_name(Some(database))
            .user(Some(user))
            .pass(Some(password));

        let pool = Pool::new(opts);
        let logger = Self {
            pool,
            bot_name: bot_name.to_string(),
        };
        logger.ensure_tables().await?;
        Ok(logger)
    }

    async fn ensure_tables(&self) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            r#"
            CREATE TABLE IF NOT EXISTS bot_sessions (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                user_id BIGINT NOT NULL,
                bot_name VARCHAR(64) NOT NULL,
                state VARCHAR(32) DEFAULT 'greeting',
                is_active TINYINT(1) DEFAULT TRUE,
                session_start TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                session_end TIMESTAMP NULL,
                KEY idx_session_user (user_id),
                KEY idx_session_bot (bot_name)
            )
        "#,
            (),
        )
        .await?;
        conn.exec_drop(
            r#"
            CREATE TABLE IF NOT EXISTS bot_messages (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                telegram_message_id BIGINT NOT NULL,
                user_id BIGINT NOT NULL,
                bot_name VARCHAR(64) NOT NULL,
                direction VARCHAR(16) NOT NULL,
                message_text TEXT,
                reply_to_message_id BIGINT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                KEY idx_msg_user (user_id),
                KEY idx_msg_bot (bot_name)
            )
        "#,
            (),
        )
        .await?;
        Ok(())
    }

    async fn save_user(&self, user: Option<&User>) -> Result<()> {
        let Some(u) = user else {
            return Ok(());
        };
        let query = r#"
            INSERT INTO bot_users (id, username, first_name, last_name, language_code, is_premium, is_bot)
            VALUES (:id, :username, :first_name, :last_name, :language_code, :is_premium, :is_bot)
            ON DUPLICATE KEY UPDATE
                username = VALUES(username),
                first_name = VALUES(first_name),
                last_name = VALUES(last_name),
                language_code = VALUES(language_code),
                is_premium = VALUES(is_premium),
                last_seen_at = CURRENT_TIMESTAMP
        "#;

        let params = params! {
            "id" => u.id.0 as i64,
            "username" => u.username.clone().unwrap_or_default(),
            "first_name" => u.first_name.clone(),
            "last_name" => u.last_name.clone().unwrap_or_default(),
            "language_code" => u
                .language_code
                .clone()
                .map(|c| c.to_string())
                .unwrap_or_default(),
            "is_premium" => u.is_premium,
            "is_bot" => u.is_bot,
        };

        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(query, params).await?;
        Ok(())
    }

    async fn log_message(
        &self,
        message_id: i64,
        user_id: i64,
        direction: &str,
        text: &str,
        reply_to: Option<i64>,
    ) -> Result<()> {
        let query = r#"
            INSERT INTO bot_messages
            (telegram_message_id, user_id, bot_name, direction, message_text, reply_to_message_id)
            VALUES (:message_id, :user_id, :bot_name, :direction, :message_text, :reply_to)
        "#;

        let params = params! {
            "message_id" => message_id,
            "user_id" => user_id,
            "bot_name" => self.bot_name.clone(),
            "direction" => direction.to_string(),
            "message_text" => text.to_string(),
            "reply_to" => reply_to,
        };

        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(query, params).await?;
        Ok(())
    }

    async fn get_active_session(&self, user_id: i64) -> Result<Option<i64>> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<(i64,)> = conn
            .exec_first(
                r#"
                SELECT id FROM bot_sessions
                WHERE user_id = :user_id AND bot_name = :bot_name AND is_active = TRUE
                ORDER BY session_start DESC
                LIMIT 1
            "#,
                params! {
                    "user_id" => user_id,
                    "bot_name" => self.bot_name.clone(),
                },
            )
            .await?;
        Ok(row.map(|t| t.0))
    }

    async fn create_session(&self, user_id: i64) -> Result<i64> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            r#"
            UPDATE bot_sessions
            SET is_active = FALSE, session_end = CURRENT_TIMESTAMP
            WHERE user_id = :user_id AND bot_name = :bot_name AND is_active = TRUE
        "#,
            params! {
                "user_id" => user_id,
                "bot_name" => self.bot_name.clone(),
            },
        )
        .await?;

        conn.exec_drop(
            r#"
            INSERT INTO bot_sessions (user_id, bot_name, state)
            VALUES (:user_id, :bot_name, 'greeting')
        "#,
            params! {
                "user_id" => user_id,
                "bot_name" => self.bot_name.clone(),
            },
        )
        .await?;

        let session_id: Option<(i64,)> = conn.exec_first("SELECT LAST_INSERT_ID()", ()).await?;
        session_id
            .map(|t| t.0)
            .ok_or_else(|| anyhow!("Failed to create session"))
    }

    async fn conversation_history(&self, user_id: i64, limit: usize) -> Result<Vec<HistoryRow>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<(String, String)> = conn
            .exec(
                r#"
                SELECT direction, message_text
                FROM bot_messages
                WHERE user_id = :user_id AND bot_name = :bot_name
                ORDER BY created_at DESC
                LIMIT :limit
            "#,
                params! {
                    "user_id" => user_id,
                    "bot_name" => self.bot_name.clone(),
                    "limit" => limit as u32,
                },
            )
            .await?;

        Ok(rows
            .into_iter()
            .rev()
            .map(|(direction, message_text)| HistoryRow {
                direction,
                message_text,
            })
            .collect())
    }
}

struct HistoryRow {
    direction: String,
    message_text: String,
}

struct AbTestManager {
    pool: Pool,
    bot_name: String,
    experiment_name: String,
    variants: Vec<PromptVariant>,
}

impl AbTestManager {
    async fn new(
        db: Arc<MySqlLogger>,
        bot_name: String,
        experiment_name: String,
        variants: Vec<PromptVariant>,
    ) -> Result<Self> {
        if variants.is_empty() {
            return Err(anyhow!("At least one prompt variant is required"));
        }
        let manager = Self {
            pool: db.pool.clone(),
            bot_name,
            experiment_name,
            variants,
        };
        manager.ensure_table().await?;
        Ok(manager)
    }

    async fn ensure_table(&self) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            r#"
            CREATE TABLE IF NOT EXISTS bot_experiments (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                bot_name VARCHAR(64) NOT NULL,
                experiment_name VARCHAR(128) NOT NULL,
                session_id BIGINT NULL,
                user_id BIGINT NOT NULL,
                variant VARCHAR(64) NOT NULL,
                conversion TINYINT(1) DEFAULT 0,
                conversion_reason VARCHAR(255) NULL,
                conversion_value INT NULL,
                assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                closed_at TIMESTAMP NULL,
                KEY idx_experiment (bot_name, experiment_name, variant),
                KEY idx_session (session_id),
                KEY idx_user (user_id)
            )
        "#,
            (),
        )
        .await?;
        Ok(())
    }

    async fn get_or_assign_variant(&self, user_id: i64, session_id: i64) -> Result<PromptVariant> {
        if let Some(name) = self.fetch_assigned_variant(user_id, session_id).await? {
            let variant = self
                .variants
                .iter()
                .find(|v| v.name == name)
                .cloned()
                .ok_or_else(|| anyhow!("Unknown variant {name}"))?;

            info!(
                "Using previously assigned variant '{}' ({}) for user {} session {}",
                variant.name, variant.description, user_id, session_id
            );
            return Ok(variant);
        }

        let chosen = self.choose_variant();
        info!(
            "Assigning variant '{}' ({}) to user {} session {}",
            chosen.name, chosen.description, user_id, session_id
        );
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            r#"
            INSERT INTO bot_experiments
            (bot_name, experiment_name, session_id, user_id, variant)
            VALUES (:bot_name, :experiment_name, :session_id, :user_id, :variant)
        "#,
            params! {
                "bot_name" => self.bot_name.clone(),
                "experiment_name" => self.experiment_name.clone(),
                "session_id" => session_id,
                "user_id" => user_id,
                "variant" => chosen.name.clone(),
            },
        )
        .await?;
        Ok(chosen)
    }

    async fn fetch_assigned_variant(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<Option<String>> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<(String,)> = conn
            .exec_first(
                r#"
                SELECT variant
                FROM bot_experiments
                WHERE bot_name = :bot_name
                  AND experiment_name = :experiment_name
                  AND user_id = :user_id
                  AND session_id = :session_id
                ORDER BY assigned_at DESC
                LIMIT 1
            "#,
                params! {
                    "bot_name" => self.bot_name.clone(),
                    "experiment_name" => self.experiment_name.clone(),
                    "user_id" => user_id,
                    "session_id" => session_id,
                },
            )
            .await?;
        Ok(row.map(|t| t.0))
    }

    async fn detect_and_mark_conversion(&self, session_id: i64, text: &str) -> Result<()> {
        if let Some(reason) = detect_conversion_reason(text) {
            let mut conn = self.pool.get_conn().await?;
            conn.exec_drop(
                r#"
                UPDATE bot_experiments
                SET conversion = 1,
                    conversion_reason = COALESCE(:reason, conversion_reason),
                    closed_at = COALESCE(closed_at, CURRENT_TIMESTAMP)
                WHERE session_id = :session_id
                  AND bot_name = :bot_name
                  AND experiment_name = :experiment_name
            "#,
                params! {
                    "reason" => reason,
                    "session_id" => session_id,
                    "bot_name" => self.bot_name.clone(),
                    "experiment_name" => self.experiment_name.clone(),
                },
            )
            .await?;
        }
        Ok(())
    }

    fn choose_variant(&self) -> PromptVariant {
        let weights: Vec<f32> = self.variants.iter().map(|v| v.weight).collect();
        if let Ok(dist) = WeightedIndex::new(weights) {
            let mut rng = thread_rng();
            let idx = dist.sample(&mut rng);
            return self.variants[idx].clone();
        }
        self.variants
            .first()
            .cloned()
            .unwrap_or_else(|| PromptVariant {
                name: "fallback".to_string(),
                prompt: SALES_SYSTEM_PROMPT.to_string(),
                description: "fallback".to_string(),
                weight: 1.0,
                temperature: 0.7,
                model: None,
            })
    }

    fn fallback_variant(&self) -> PromptVariant {
        self.variants
            .first()
            .cloned()
            .unwrap_or_else(|| PromptVariant {
                name: "fallback".to_string(),
                prompt: SALES_SYSTEM_PROMPT.to_string(),
                description: "fallback".to_string(),
                weight: 1.0,
                temperature: 0.7,
                model: None,
            })
    }
}

fn detect_conversion_reason(text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }
    let phone_re = Regex::new(r"(?i)(\+?\d[\d\s\-\(\)]{8,}\d)").unwrap();
    if phone_re.is_match(text) {
        return Some("phone_shared".to_string());
    }

    let normalized = text.to_lowercase();
    let intent_keywords = [
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
    if intent_keywords.iter().any(|kw| normalized.contains(kw)) {
        return Some("purchase_intent".to_string());
    }

    let delivery_keywords = ["доставка", "оплата", "адрес", "курьер", "самовывоз"];
    if delivery_keywords.iter().any(|kw| normalized.contains(kw))
        && (normalized.contains("давай") || normalized.contains("офор"))
    {
        return Some("checkout_details".to_string());
    }

    None
}

fn prompt_variants() -> Vec<PromptVariant> {
    vec![
        PromptVariant {
            name: "control_consultative".to_string(),
            prompt: SALES_SYSTEM_PROMPT.to_string(),
            description: "Базовый скрипт: выявление потребностей → подбор → закрытие".to_string(),
            weight: 1.0,
            temperature: 0.7,
            model: None,
        },
        PromptVariant {
            name: "fast_close_cta".to_string(),
            prompt: FAST_CLOSE_PROMPT.to_string(),
            description: "Короткие ответы, ранний CTA на оплату/доставку".to_string(),
            weight: 1.0,
            temperature: 0.6,
            model: None,
        },
        PromptVariant {
            name: "story_social_proof".to_string(),
            prompt: STORY_PROOF_PROMPT.to_string(),
            description: "SPIN + микро-кейсы и апсейл".to_string(),
            weight: 1.0,
            temperature: 0.7,
            model: None,
        },
    ]
}
