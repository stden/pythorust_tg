//! DevOps AI Assistant Telegram Bot with monitoring and quick commands.
//!
//! Features:
//! - Service health monitoring (HTTP, TCP, systemd)
//! - Log viewing
//! - Service restart with confirmation
//! - AI-powered DevOps questions
//!
//! Usage:
//!   DEVOPS_BOT_TOKEN=... cargo run --bin devops_ai_bot

use anyhow::{Context, Result};
use dotenvy::dotenv;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{error, info, warn};

use telegram_reader::integrations::openai::{ChatMessage, OpenAIClient};

const DEFAULT_SYSTEM_PROMPT: &str = "Ты — DevOps/Backend ассистент. Отвечай коротко и по шагам. \
    Всегда предлагай безопасные команды, проверяй статус сервисов, помни про логи и порты. \
    Не придумывай данные, если их нет.";

/// Service configuration.
#[derive(Debug, Clone, Deserialize)]
struct ServiceConfig {
    #[serde(default)]
    name: String,
    #[serde(default = "default_kind")]
    kind: String,
    url: Option<String>,
    #[serde(default = "default_status")]
    expected_status: u16,
    #[serde(default = "default_timeout")]
    timeout: f64,
    restart_command: Option<String>,
    log_command: Option<String>,
    status_command: Option<String>,
    tcp_host: Option<String>,
    tcp_port: Option<u16>,
    service_name: Option<String>,
}

fn default_kind() -> String {
    "http".to_string()
}
fn default_status() -> u16 {
    200
}
fn default_timeout() -> f64 {
    10.0
}

/// Monitor configuration.
#[derive(Debug, Clone, Deserialize, Default)]
struct MonitorConfig {
    #[serde(default)]
    enabled: bool,
    interval_seconds: Option<u64>,
    cooldown_seconds: Option<u64>,
    alert_chat_id: Option<i64>,
}

/// Bot configuration.
#[derive(Debug, Clone, Deserialize, Default)]
struct BotConfig {
    allowed_users: Option<Vec<i64>>,
}

/// YAML config structure.
#[derive(Debug, Clone, Deserialize, Default)]
struct Config {
    #[serde(default)]
    bot: BotConfig,
    #[serde(default)]
    monitor: MonitorConfig,
    #[serde(default)]
    services: HashMap<String, ServiceConfig>,
}

/// Service check result.
#[derive(Debug, Clone)]
struct CheckResult {
    name: String,
    ok: bool,
    detail: String,
    latency_ms: Option<u64>,
}

/// Application state.
struct AppState {
    config: Config,
    ai: OpenAIClient,
    ai_model: String,
    allowed_users: Vec<i64>,
    last_status: Mutex<HashMap<String, bool>>,
    last_alert: Mutex<HashMap<String, Instant>>,
}

impl AppState {
    fn is_allowed(&self, user_id: i64) -> bool {
        self.allowed_users.is_empty() || self.allowed_users.contains(&user_id)
    }
}

/// Load config from YAML file.
fn load_config() -> Config {
    let config_path =
        env::var("DEVOPS_BOT_CONFIG").unwrap_or_else(|_| "devops_bot.yml".to_string());
    let path = Path::new(&config_path);

    if !path.exists() {
        warn!("Config file {} not found, using defaults", config_path);
        return Config::default();
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match serde_yaml::from_str(&content) {
            Ok(cfg) => {
                info!("Loaded config from {}", config_path);
                cfg
            }
            Err(e) => {
                error!("Failed to parse config: {}", e);
                Config::default()
            }
        },
        Err(e) => {
            error!("Failed to read config: {}", e);
            Config::default()
        }
    }
}

/// Check HTTP service.
async fn check_http(svc: &ServiceConfig) -> CheckResult {
    let url = match &svc.url {
        Some(u) => u,
        None => {
            return CheckResult {
                name: svc.name.clone(),
                ok: false,
                detail: "URL not set".to_string(),
                latency_ms: None,
            }
        }
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs_f64(svc.timeout))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let start = Instant::now();

    match client.get(url).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let status = resp.status().as_u16();
            let ok = status == svc.expected_status;

            CheckResult {
                name: svc.name.clone(),
                ok,
                detail: format!("HTTP {}", status),
                latency_ms: Some(latency_ms),
            }
        }
        Err(e) => CheckResult {
            name: svc.name.clone(),
            ok: false,
            detail: e.to_string(),
            latency_ms: None,
        },
    }
}

/// Check TCP service.
async fn check_tcp(svc: &ServiceConfig) -> CheckResult {
    let host = svc.tcp_host.as_deref().unwrap_or("localhost");
    let port = match svc.tcp_port {
        Some(p) => p,
        None => {
            return CheckResult {
                name: svc.name.clone(),
                ok: false,
                detail: "TCP port not configured".to_string(),
                latency_ms: None,
            }
        }
    };

    let start = Instant::now();
    let addr = format!("{}:{}", host, port);

    match timeout(
        Duration::from_secs_f64(svc.timeout),
        TcpStream::connect(&addr),
    )
    .await
    {
        Ok(Ok(_)) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            CheckResult {
                name: svc.name.clone(),
                ok: true,
                detail: format!("TCP {}:{}", host, port),
                latency_ms: Some(latency_ms),
            }
        }
        Ok(Err(e)) => CheckResult {
            name: svc.name.clone(),
            ok: false,
            detail: e.to_string(),
            latency_ms: None,
        },
        Err(_) => CheckResult {
            name: svc.name.clone(),
            ok: false,
            detail: format!("Timeout after {}s", svc.timeout),
            latency_ms: None,
        },
    }
}

/// Check service via status command.
async fn check_status_command(svc: &ServiceConfig) -> CheckResult {
    let cmd = match &svc.status_command {
        Some(c) => c,
        None => {
            return CheckResult {
                name: svc.name.clone(),
                ok: false,
                detail: "Status command not configured".to_string(),
                latency_ms: None,
            }
        }
    };

    match run_command(cmd, svc.timeout).await {
        Ok((code, stdout, stderr)) => {
            let output = if stdout.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            let ok = code == 0 && (output.contains("active") || output.contains("running"));

            CheckResult {
                name: svc.name.clone(),
                ok,
                detail: if output.is_empty() {
                    format!("exit {}", code)
                } else {
                    output.to_string()
                },
                latency_ms: None,
            }
        }
        Err(e) => CheckResult {
            name: svc.name.clone(),
            ok: false,
            detail: e.to_string(),
            latency_ms: None,
        },
    }
}

/// Check any service based on kind.
async fn check_service(svc: &ServiceConfig) -> CheckResult {
    match svc.kind.as_str() {
        "http" => check_http(svc).await,
        "tcp" => check_tcp(svc).await,
        "systemd" | "command" => check_status_command(svc).await,
        _ => CheckResult {
            name: svc.name.clone(),
            ok: false,
            detail: "Unknown service kind".to_string(),
            latency_ms: None,
        },
    }
}

/// Run a shell command.
async fn run_command(cmd: &str, timeout_secs: f64) -> Result<(i32, String, String)> {
    let output = timeout(
        Duration::from_secs_f64(timeout_secs),
        Command::new("sh").args(["-c", cmd]).output(),
    )
    .await??;

    Ok((
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

/// Handle /start command.
async fn handle_start(bot: Bot, msg: Message) -> Result<()> {
    let text = "🤖 DevOps AI бот готов.\n\
        /status [name] — статус сервисов\n\
        /logs <name> [filter] — последние логи\n\
        /restart <name> — перезапуск (с подтверждением)\n\
        /ask <вопрос> — вопрос по DevOps\n\
        /help — показать команды";

    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

/// Handle /status command.
async fn handle_status(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
    target: Option<String>,
) -> Result<()> {
    if let Some(ref name) = target {
        if !state.config.services.contains_key(name) {
            let available = state
                .config
                .services
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            bot.send_message(
                msg.chat.id,
                format!("❔ Сервис '{}' не найден. Доступно: {}", name, available),
            )
            .await?;
            return Ok(());
        }
    }

    let names: Vec<String> = if let Some(name) = target {
        vec![name]
    } else {
        state.config.services.keys().cloned().collect()
    };

    if names.is_empty() {
        bot.send_message(
            msg.chat.id,
            "Нет настроенных сервисов. Проверьте devops_bot.yml.",
        )
        .await?;
        return Ok(());
    }

    bot.send_message(msg.chat.id, "⏳ Проверяю...").await?;

    let mut lines = Vec::new();
    for name in names {
        if let Some(svc) = state.config.services.get(&name) {
            let result = check_service(svc).await;
            let emoji = if result.ok { "✅" } else { "❌" };
            let latency = result
                .latency_ms
                .map(|ms| format!(" ({}ms)", ms))
                .unwrap_or_default();
            lines.push(format!("{} {}: {}{}", emoji, name, result.detail, latency));
        }
    }

    bot.send_message(msg.chat.id, lines.join("\n")).await?;
    Ok(())
}

/// Handle /logs command.
async fn handle_logs(bot: Bot, msg: Message, state: Arc<AppState>, parts: Vec<&str>) -> Result<()> {
    if parts.len() < 2 {
        bot.send_message(msg.chat.id, "Использование: /logs <service> [filter]")
            .await?;
        return Ok(());
    }

    let svc_name = parts[1];
    let svc = match state.config.services.get(svc_name) {
        Some(s) => s,
        None => {
            bot.send_message(msg.chat.id, format!("Сервис '{}' не найден.", svc_name))
                .await?;
            return Ok(());
        }
    };

    let log_cmd = match &svc.log_command {
        Some(c) => c.clone(),
        None => {
            bot.send_message(msg.chat.id, "Для сервиса не настроена команда логов.")
                .await?;
            return Ok(());
        }
    };

    let grep_term = if parts.len() > 2 { parts[2] } else { "" };
    let cmd = if grep_term.is_empty() {
        log_cmd
    } else {
        format!("{} | grep -i '{}'", log_cmd, grep_term)
    };

    bot.send_message(msg.chat.id, format!("🪵 Читаю логи {}...", svc_name))
        .await?;

    match run_command(&cmd, svc.timeout).await {
        Ok((code, stdout, stderr)) => {
            let mut output = if stdout.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            };

            if output.is_empty() {
                output = format!("empty (exit {})", code);
            }

            // Truncate if too long
            if output.len() > 3500 {
                output = output[output.len() - 3500..].to_string();
            }

            bot.send_message(msg.chat.id, format!("```\n{}\n```", output))
                .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ Ошибка: {}", e))
                .await?;
        }
    }

    Ok(())
}

/// Handle /restart command - show confirmation.
async fn handle_restart(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
    parts: Vec<&str>,
) -> Result<()> {
    if parts.len() < 2 {
        bot.send_message(msg.chat.id, "Использование: /restart <service>")
            .await?;
        return Ok(());
    }

    let svc_name = parts[1];
    let svc = match state.config.services.get(svc_name) {
        Some(s) => s,
        None => {
            bot.send_message(msg.chat.id, format!("Сервис '{}' не найден.", svc_name))
                .await?;
            return Ok(());
        }
    };

    let restart_cmd = match &svc.restart_command {
        Some(c) => c,
        None => {
            bot.send_message(msg.chat.id, "Для сервиса не настроена команда перезапуска.")
                .await?;
            return Ok(());
        }
    };

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ Подтвердить", format!("restart:{}:yes", svc_name)),
        InlineKeyboardButton::callback("❌ Отмена", format!("restart:{}:no", svc_name)),
    ]]);

    bot.send_message(
        msg.chat.id,
        format!("Перезапустить {}? Команда:\n`{}`", svc_name, restart_cmd),
    )
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

/// Handle /ask command.
async fn handle_ask(bot: Bot, msg: Message, state: Arc<AppState>, question: String) -> Result<()> {
    if question.trim().is_empty() {
        bot.send_message(msg.chat.id, "Использование: /ask <вопрос>")
            .await?;
        return Ok(());
    }

    bot.send_message(msg.chat.id, "🤔 Думаю...").await?;

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: Some(DEFAULT_SYSTEM_PROMPT.to_string()),
        },
        ChatMessage {
            role: "user".to_string(),
            content: Some(question),
        },
    ];

    match state
        .ai
        .chat_completion(messages, &state.ai_model, 0.2, 500)
        .await
    {
        Ok(response) => {
            bot.send_message(msg.chat.id, response).await?;
        }
        Err(e) => {
            error!("AI error: {}", e);
            bot.send_message(msg.chat.id, format!("❌ Ошибка AI: {}", e))
                .await?;
        }
    }

    Ok(())
}

/// Handle callback queries (restart confirmation).
async fn handle_callback(bot: Bot, q: CallbackQuery, state: Arc<AppState>) -> Result<()> {
    let data = q.data.as_deref().unwrap_or("");
    let user_id = q.from.id.0 as i64;

    if !state.is_allowed(user_id) {
        bot.answer_callback_query(&q.id)
            .text("⛔ Нет доступа")
            .show_alert(true)
            .await?;
        return Ok(());
    }

    if !data.starts_with("restart:") {
        return Ok(());
    }

    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 3 {
        return Ok(());
    }

    let svc_name = parts[1];
    let action = parts[2];

    let svc = match state.config.services.get(svc_name) {
        Some(s) => s,
        None => {
            bot.answer_callback_query(&q.id)
                .text("Сервис недоступен")
                .await?;
            return Ok(());
        }
    };

    let restart_cmd = match &svc.restart_command {
        Some(c) => c,
        None => {
            bot.answer_callback_query(&q.id)
                .text("Нет команды перезапуска")
                .await?;
            return Ok(());
        }
    };

    if action == "no" {
        if let Some(msg) = &q.message {
            bot.edit_message_text(
                msg.chat.id,
                msg.id,
                format!("Перезапуск {} отменен.", svc_name),
            )
            .await?;
        }
        return Ok(());
    }

    if let Some(msg) = &q.message {
        bot.edit_message_text(
            msg.chat.id,
            msg.id,
            format!("🔄 Перезапускаю {}...", svc_name),
        )
        .await?;
    }

    let (code, stdout, stderr) = run_command(restart_cmd, f64::max(30.0, svc.timeout)).await?;
    let mut result = if stdout.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        stdout.trim().to_string()
    };

    if result.is_empty() {
        result = format!("exit {}", code);
    }

    // Check status after restart
    let status_line = {
        let check = check_service(svc).await;
        let emoji = if check.ok { "✅" } else { "❌" };
        format!("\n{} После перезапуска: {}", emoji, check.detail)
    };

    // Truncate
    if result.len() > 1500 {
        result = result[result.len() - 1500..].to_string();
    }

    if let Some(msg) = &q.message {
        bot.edit_message_text(
            msg.chat.id,
            msg.id,
            format!(
                "Результат перезапуска {}:\n```\n{}\n```{}",
                svc_name, result, status_line
            ),
        )
        .await?;
    }

    Ok(())
}

/// Background monitoring loop.
async fn monitor_loop(bot: Bot, state: Arc<AppState>) {
    let monitor_cfg = &state.config.monitor;
    if !monitor_cfg.enabled {
        info!("Monitor disabled in config");
        return;
    }

    let interval = monitor_cfg.interval_seconds.unwrap_or(300);
    let cooldown = monitor_cfg.cooldown_seconds.unwrap_or(300);
    let alert_chat_id = match monitor_cfg.alert_chat_id.or_else(|| {
        env::var("DEVOPS_ALERT_CHAT_ID")
            .ok()
            .and_then(|s| s.parse().ok())
    }) {
        Some(id) => ChatId(id),
        None => {
            warn!("No alert_chat_id configured, monitoring disabled");
            return;
        }
    };

    if state.config.services.is_empty() {
        info!("No services configured, monitor loop skipped");
        return;
    }

    info!(
        "Starting monitor loop for {} services",
        state.config.services.len()
    );

    loop {
        for (name, svc) in &state.config.services {
            let result = check_service(svc).await;
            let prev = {
                let lock = state.last_status.lock().await;
                lock.get(name).copied()
            };

            // Update status
            {
                let mut lock = state.last_status.lock().await;
                lock.insert(name.clone(), result.ok);
            }

            let now = Instant::now();

            if !result.ok {
                // Check cooldown
                let should_alert = {
                    let lock = state.last_alert.lock().await;
                    match lock.get(name) {
                        Some(last) => now.duration_since(*last).as_secs() >= cooldown,
                        None => true,
                    }
                };

                if prev != Some(false) || should_alert {
                    // Send alert
                    let _ = bot
                        .send_message(alert_chat_id, format!("❌ {}: {}", name, result.detail))
                        .await;

                    let mut lock = state.last_alert.lock().await;
                    lock.insert(name.clone(), now);
                }
            } else if prev == Some(false) {
                // Recovered
                let _ = bot
                    .send_message(
                        alert_chat_id,
                        format!("✅ {} восстановился ({})", name, result.detail),
                    )
                    .await;
            }
        }

        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = env::var("DEVOPS_BOT_TOKEN")
        .or_else(|_| env::var("TASK_ASSISTANT_BOT_TOKEN"))
        .or_else(|_| env::var("TELEGRAM_BOT_TOKEN"))
        .context("Set DEVOPS_BOT_TOKEN in environment")?;

    let ai = OpenAIClient::from_env()?;
    let ai_model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let mut config = load_config();

    // Ensure service names are set
    for (name, svc) in config.services.iter_mut() {
        if svc.name.is_empty() {
            svc.name = name.clone();
        }
        // Generate status command for systemd services
        if svc.kind == "systemd" && svc.status_command.is_none() {
            let service_name = svc.service_name.as_ref().unwrap_or(name);
            svc.status_command = Some(format!("systemctl is-active {}", service_name));
        }
    }

    // Get allowed users from config and env
    let mut allowed_users: Vec<i64> = config.bot.allowed_users.clone().unwrap_or_default();
    if let Ok(env_users) = env::var("DEVOPS_ALLOWED_USERS") {
        for part in env_users.split(',') {
            if let Ok(id) = part.trim().parse() {
                if !allowed_users.contains(&id) {
                    allowed_users.push(id);
                }
            }
        }
    }

    let state = Arc::new(AppState {
        config,
        ai,
        ai_model,
        allowed_users,
        last_status: Mutex::new(HashMap::new()),
        last_alert: Mutex::new(HashMap::new()),
    });

    info!("Starting DevOps AI Bot...");

    let bot = Bot::new(token);

    // Start monitor loop
    let bot_clone = bot.clone();
    let state_clone = state.clone();
    tokio::spawn(async move {
        monitor_loop(bot_clone, state_clone).await;
    });

    let handler = dptree::entry()
        // Callback queries
        .branch(Update::filter_callback_query().endpoint({
            let state = state.clone();
            move |bot: Bot, q: CallbackQuery| {
                let state = state.clone();
                async move { handle_callback(bot, q, state).await }
            }
        }))
        // Commands
        .branch(Update::filter_message().endpoint({
            let state = state.clone();
            move |bot: Bot, msg: Message| {
                let state = state.clone();
                async move {
                    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
                    if !state.is_allowed(user_id) {
                        bot.send_message(msg.chat.id, "⛔ Доступ запрещен.").await?;
                        return Ok::<_, anyhow::Error>(());
                    }

                    let text = msg.text().unwrap_or("").to_string();
                    let parts: Vec<String> =
                        text.split_whitespace().map(|s| s.to_string()).collect();
                    let cmd = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

                    match cmd.as_str() {
                        "/start" | "/help" => handle_start(bot, msg).await?,
                        "/status" => {
                            let target = parts.get(1).cloned();
                            handle_status(bot, msg, state.clone(), target).await?
                        }
                        "/logs" => {
                            let parts_refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
                            handle_logs(bot, msg, state.clone(), parts_refs).await?
                        }
                        "/restart" => {
                            let parts_refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
                            handle_restart(bot, msg, state.clone(), parts_refs).await?
                        }
                        "/ask" => {
                            let question =
                                text.strip_prefix("/ask").unwrap_or("").trim().to_string();
                            handle_ask(bot, msg, state.clone(), question).await?
                        }
                        _ => {}
                    }

                    Ok(())
                }
            }
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
