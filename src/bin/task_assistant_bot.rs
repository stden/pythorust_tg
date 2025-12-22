//! Task Assistant Telegram Bot - помощник для автоматизации задач.
//!
//! Функции:
//! - Мониторинг и управление N8N
//! - Создание и просмотр бэкапов
//! - AI консультации
//! - Мониторинг серверов
//!
//! Usage:
//!   TASK_ASSISTANT_BOT_TOKEN=... cargo run --bin task_assistant_bot

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use dotenvy::dotenv;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use tokio::process::Command;
use tracing::{error, info};

use telegram_reader::integrations::openai::OpenAIClient;

/// Allowed user IDs from environment.
fn get_allowed_users() -> Vec<i64> {
    env::var("ALLOWED_USERS")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_default()
}

/// Check if user is allowed.
fn check_access(user_id: i64) -> bool {
    let allowed = get_allowed_users();
    allowed.is_empty() || allowed.contains(&user_id)
}

/// Check N8N health.
async fn check_n8n_health() -> (String, Option<u16>, Option<String>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    match client
        .get("https://n8n.vier-pfoten.club/healthz")
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let status_text = if status == 200 {
                "✅ Работает".to_string()
            } else {
                "❌ Ошибка".to_string()
            };
            (status_text, Some(status), None)
        }
        Err(e) => ("❌ Недоступен".to_string(), None, Some(e.to_string())),
    }
}

/// Restart N8N service.
async fn restart_n8n_service() -> (bool, Option<String>) {
    match Command::new("systemctl")
        .args(["restart", "n8n"])
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                // Wait and check health
                tokio::time::sleep(Duration::from_secs(5)).await;
                let (status, _, _) = check_n8n_health().await;
                (true, Some(status))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                (false, Some(stderr))
            }
        }
        Err(e) => (false, Some(e.to_string())),
    }
}

/// Create N8N backup using Rust binary.
async fn create_n8n_backup() -> (bool, Option<String>) {
    let project_root = env::var("PROJECT_ROOT").unwrap_or_else(|_| ".".to_string());
    let backup_bin = format!("{}/target/release/n8n_backup", project_root);

    // Try release binary first, then debug
    let bin_path = if Path::new(&backup_bin).exists() {
        backup_bin
    } else {
        format!("{}/target/debug/n8n_backup", project_root)
    };

    match Command::new(&bin_path)
        .arg("backup")
        .current_dir(&project_root)
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                (true, Some(stdout))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                (false, Some(stderr))
            }
        }
        Err(e) => (false, Some(format!("Не удалось запустить бэкап: {}", e))),
    }
}

/// Backup info.
struct BackupInfo {
    name: String,
    size_mb: f64,
    date: String,
}

/// List N8N backups.
async fn list_n8n_backups() -> Vec<BackupInfo> {
    let backup_dir = Path::new("/srv/backups/n8n");
    if !backup_dir.exists() {
        return Vec::new();
    }

    let mut backups: Vec<BackupInfo> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with("n8n_backup_"))
                    .unwrap_or(false)
            })
            .collect();

        // Sort by modification time (newest first)
        files.sort_by(|a, b| {
            let time_a = a.metadata().and_then(|m| m.modified()).ok();
            let time_b = b.metadata().and_then(|m| m.modified()).ok();
            time_b.cmp(&time_a)
        });

        for entry in files.into_iter().take(10) {
            if let Ok(meta) = entry.metadata() {
                let size_mb = meta.len() as f64 / (1024.0 * 1024.0);
                let mtime = meta.modified().ok().map(|t| {
                    let dt: DateTime<Local> = t.into();
                    dt.format("%d.%m.%Y %H:%M").to_string()
                });

                backups.push(BackupInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    size_mb,
                    date: mtime.unwrap_or_else(|| "N/A".to_string()),
                });
            }
        }
    }

    backups
}

/// Server status metrics.
struct ServerStatus {
    cpu: f64,
    memory: f64,
    disk: f64,
}

/// Get server status.
async fn get_server_status() -> Result<ServerStatus> {
    // CPU usage
    let cpu_cmd = r#"top -bn1 | grep 'Cpu(s)' | sed 's/.*, *\([0-9.]*\)%* id.*/\1/' | awk '{print 100 - $1}'"#;
    let cpu_output = Command::new("sh").args(["-c", cpu_cmd]).output().await?;
    let cpu: f64 = String::from_utf8_lossy(&cpu_output.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);

    // Memory usage
    let mem_cmd = "free | grep Mem | awk '{print ($3/$2) * 100.0}'";
    let mem_output = Command::new("sh").args(["-c", mem_cmd]).output().await?;
    let memory: f64 = String::from_utf8_lossy(&mem_output.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);

    // Disk usage
    let disk_cmd = "df -h / | tail -1 | awk '{print $5}' | sed 's/%//'";
    let disk_output = Command::new("sh").args(["-c", disk_cmd]).output().await?;
    let disk: f64 = String::from_utf8_lossy(&disk_output.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);

    Ok(ServerStatus { cpu, memory, disk })
}

/// Build main keyboard.
fn main_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "🔍 Проверить N8N",
            "check_n8n",
        )],
        vec![InlineKeyboardButton::callback(
            "🔄 Перезапустить N8N",
            "restart_n8n",
        )],
        vec![InlineKeyboardButton::callback(
            "💾 Создать бэкап",
            "create_backup",
        )],
        vec![InlineKeyboardButton::callback(
            "📋 Список бэкапов",
            "list_backups",
        )],
        vec![InlineKeyboardButton::callback(
            "🤖 ИИ-консультант",
            "ai_consultant",
        )],
        vec![InlineKeyboardButton::callback(
            "📊 Статус серверов",
            "server_status",
        )],
    ])
}

/// Application state.
#[derive(Clone)]
struct AppState {
    ai: Arc<OpenAIClient>,
}

/// Handle /start command.
async fn handle_start(bot: Bot, msg: Message) -> Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    if !check_access(user_id) {
        bot.send_message(msg.chat.id, "❌ Доступ запрещён").await?;
        return Ok(());
    }

    let text = "👋 *Привет! Я твой помощник по автоматизации.*\n\n\
                Могу помочь с:\n\
                • Мониторинг и управление N8N\n\
                • Бэкапы конфигураций\n\
                • ИИ-консультации по проектам\n\
                • Проверка статуса серверов\n\n\
                Выбери действие:";

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(main_keyboard())
        .await?;

    Ok(())
}

/// Handle callback queries.
async fn handle_callback(bot: Bot, q: CallbackQuery, _state: AppState) -> Result<()> {
    let data = q.data.as_deref().unwrap_or("");
    let user_id = q.from.id.0 as i64;

    if !check_access(user_id) {
        bot.answer_callback_query(&q.id)
            .text("❌ Доступ запрещён")
            .await?;
        return Ok(());
    }

    let chat_id = q.message.as_ref().map(|m| m.chat.id);

    match data {
        "check_n8n" => {
            bot.answer_callback_query(&q.id)
                .text("Проверяю N8N...")
                .await?;

            let (status, code, error) = check_n8n_health().await;
            let text = format!(
                "*N8N Health Check*\n\n\
                 Статус: {}\n\
                 HTTP Code: {}\n\
                 Ошибка: {}",
                status,
                code.map(|c| c.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
                error.unwrap_or_else(|| "Нет".to_string())
            );

            if let Some(id) = chat_id {
                bot.send_message(id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
        }

        "restart_n8n" => {
            bot.answer_callback_query(&q.id)
                .text("Перезапускаю N8N...")
                .await?;

            let (success, info) = restart_n8n_service().await;
            let text = if success {
                format!(
                    "✅ *N8N перезапущен*\n\nСтатус: {}",
                    info.unwrap_or_default()
                )
            } else {
                format!(
                    "❌ *Ошибка перезапуска*\n\nОшибка: {}",
                    info.unwrap_or_default()
                )
            };

            if let Some(id) = chat_id {
                bot.send_message(id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
        }

        "create_backup" => {
            bot.answer_callback_query(&q.id)
                .text("Создаю бэкап...")
                .await?;

            let (success, info) = create_n8n_backup().await;
            let text = if success {
                "✅ Бэкап создан успешно".to_string()
            } else {
                format!("❌ Ошибка: {}", info.unwrap_or_default())
            };

            if let Some(id) = chat_id {
                bot.send_message(id, text).await?;
            }
        }

        "list_backups" => {
            bot.answer_callback_query(&q.id)
                .text("Получаю список бэкапов...")
                .await?;

            let backups = list_n8n_backups().await;
            let text = if backups.is_empty() {
                "📋 Бэкапы не найдены".to_string()
            } else {
                let mut text = "📋 *Последние бэкапы N8N:*\n\n".to_string();
                for backup in backups {
                    text.push_str(&format!(
                        "• {}\n  {} ({:.1} MB)\n\n",
                        backup.name, backup.date, backup.size_mb
                    ));
                }
                text
            };

            if let Some(id) = chat_id {
                bot.send_message(id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
        }

        "ai_consultant" => {
            bot.answer_callback_query(&q.id).await?;

            let text = "🤖 *ИИ-консультант*\n\n\
                        Просто напиши свой вопрос, и я помогу!\n\n\
                        Примеры:\n\
                        • Как настроить Caddy для N8N?\n\
                        • Почему приложение недоступно извне?\n\
                        • Напиши скрипт для мониторинга";

            if let Some(id) = chat_id {
                bot.send_message(id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
        }

        "server_status" => {
            bot.answer_callback_query(&q.id)
                .text("Получаю статус сервера...")
                .await?;

            let text = match get_server_status().await {
                Ok(status) => {
                    let cpu_emoji = if status.cpu < 70.0 {
                        "🟢"
                    } else if status.cpu < 90.0 {
                        "🟡"
                    } else {
                        "🔴"
                    };
                    let mem_emoji = if status.memory < 70.0 {
                        "🟢"
                    } else if status.memory < 90.0 {
                        "🟡"
                    } else {
                        "🔴"
                    };
                    let disk_emoji = if status.disk < 70.0 {
                        "🟢"
                    } else if status.disk < 90.0 {
                        "🟡"
                    } else {
                        "🔴"
                    };

                    format!(
                        "📊 *Статус сервера*\n\n\
                         {} CPU: {:.1}%\n\
                         {} RAM: {:.1}%\n\
                         {} Disk: {:.1}%",
                        cpu_emoji, status.cpu, mem_emoji, status.memory, disk_emoji, status.disk
                    )
                }
                Err(e) => format!("❌ Ошибка: {}", e),
            };

            if let Some(id) = chat_id {
                bot.send_message(id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
        }

        _ => {
            bot.answer_callback_query(&q.id)
                .text("Неизвестная команда")
                .await?;
        }
    }

    Ok(())
}

/// Handle regular messages (AI consultant).
async fn handle_message(bot: Bot, msg: Message, state: AppState) -> Result<()> {
    let text = match msg.text() {
        Some(t) if !t.starts_with('/') => t,
        _ => return Ok(()),
    };

    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    if !check_access(user_id) {
        return Ok(());
    }

    bot.send_message(msg.chat.id, "🤔 Думаю...").await?;

    let system_prompt = "Ты - технический помощник. Помогаешь с вопросами по N8N, \
                         серверам, автоматизации. Отвечай кратко и по делу, с примерами если нужно.";

    let messages = vec![
        telegram_reader::integrations::openai::ChatMessage {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()),
        },
        telegram_reader::integrations::openai::ChatMessage {
            role: "user".to_string(),
            content: Some(text.to_string()),
        },
    ];

    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    match state.ai.chat_completion(messages, &model, 0.3, 2048).await {
        Ok(response) => {
            bot.send_message(msg.chat.id, response).await?;
        }
        Err(e) => {
            error!("AI error: {}", e);
            bot.send_message(msg.chat.id, format!("❌ Ошибка: {}", e))
                .await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = env::var("TASK_ASSISTANT_BOT_TOKEN").context("TASK_ASSISTANT_BOT_TOKEN not set")?;

    let ai = OpenAIClient::from_env()?;
    let state = AppState { ai: Arc::new(ai) };

    info!("Starting Task Assistant Bot...");

    let bot = Bot::new(token);

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text() == Some("/start"))
                .endpoint(handle_start),
        )
        .branch(
            Update::filter_callback_query().endpoint(move |bot: Bot, q: CallbackQuery| {
                let state = state.clone();
                async move { handle_callback(bot, q, state).await }
            }),
        )
        .branch(
            Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
                let state_clone = AppState {
                    ai: Arc::new(OpenAIClient::from_env().unwrap()),
                };
                async move { handle_message(bot, msg, state_clone).await }
            }),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
