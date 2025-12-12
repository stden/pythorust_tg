//! VibeCoderz Bot (Rust)
//!
//! Бот для вайбкодинг-пати: запуск лобби, сбор ответов, голосование и логирование
//! пользователей/сообщений в MySQL.

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use mysql_async::{prelude::*, Pool};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::{
    CallbackQuery, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, User,
};
use tokio::sync::RwLock;

const BOT_NAME: &str = "VibeCoderzBot";

const PROMPTS: &[&str] = &[
    "Собери вайб-спринт: цвет, звук, движение. 3 строки, 1 эмодзи максимум.",
    "Код настроения для утреннего созвона: подсвети страх, цель и одну шутку.",
    "Архитектура пруда: три узла (ритм, поток, фокус) и как они синхронизируются.",
    "Напиши ритуал деплоя в стиле хайку: причина, действие, откат.",
    "Подготовь вайб-карту спринта: риск, яркое событие и скрытый баг.",
    "Сделай MIDI-настроение: темп, инструмент, первая нота — всё в тексте.",
    "Опиши \"идеальный вечер кодера\" в формате JSON из 3 ключей.",
    "Сборка команды мечты: роли трёх коев и их короткие суперсилы.",
    "Спринт без дедлайнов: как понять, что ты в потоке? Дай чеклист.",
    "Набросай эмодзи-протокол стендапа: статус, блокер, хайлайт.",
];

#[derive(Clone)]
struct AppState {
    db: Arc<MySqlLogger>,
    games: Arc<RwLock<HashMap<i64, GameState>>>,
    round_duration: Duration,
    vote_duration: Duration,
    allowed_users: HashSet<i64>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct GameState {
    host_id: i64,
    host_name: String,
    players: HashMap<i64, String>,
    scores: HashMap<i64, i32>,
    round: Option<VibeRound>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum RoundStatus {
    Collecting,
    Voting,
    Closed,
}

#[derive(Clone, Debug)]
struct VibeRound {
    round_id: String,
    prompt: String,
    submissions: HashMap<i64, String>,
    voter_choice: HashMap<i64, i64>,
    vote_message_id: Option<i32>,
    status: RoundStatus,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = std::env::var("VIBECODING_BOT_TOKEN")
        .map_err(|_| anyhow!("VIBECODING_BOT_TOKEN not set in environment"))?;
    let api_id: i32 = std::env::var("TELEGRAM_API_ID")
        .context("TELEGRAM_API_ID not set")?
        .parse()
        .context("Invalid TELEGRAM_API_ID")?;
    let api_hash = std::env::var("TELEGRAM_API_HASH").context("TELEGRAM_API_HASH not set")?;

    let round_duration = env_or_default("VIBE_ROUND_DURATION", 90);
    let vote_duration = env_or_default("VIBE_VOTE_DURATION", 45);
    let allowed_users = parse_allowed_users();

    let db = Arc::new(MySqlLogger::new().await?);
    let state = AppState {
        db,
        games: Arc::new(RwLock::new(HashMap::new())),
        round_duration: Duration::from_secs(round_duration),
        vote_duration: Duration::from_secs(vote_duration),
        allowed_users,
    };

    let bot = Bot::new(token);

    // Create the client session (grammers requires API ID/HASH even for bots).
    // Teloxide handles polling internally; API credentials are required for consistency
    // with the rest of the project tooling.
    let _ = (api_id, api_hash); // silence unused if only token is used by teloxide

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint({
            let state = state.clone();
            move |bot, msg| handle_message(bot, state.clone(), msg)
        }))
        .branch(Update::filter_callback_query().endpoint({
            let state = state.clone();
            move |bot, query| handle_callback(bot, state.clone(), query)
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_message(bot: Bot, state: AppState, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let text = match msg.text() {
        Some(t) => t.trim(),
        None => return Ok(()),
    };

    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(chat_id.0);
    let user_name = display_name(msg.from());

    state.db.save_user_if_needed(msg.from()).await;
    state
        .db
        .log_message(
            msg.id.0 as i64,
            user_id,
            "incoming",
            text,
            msg.reply_to_message().map(|m| m.id.0 as i64),
        )
        .await;

    if is_command(text, "start") || is_command(text, "help") {
        let reply = "👋 Я VibeCoderz Bot. Запускаю вайбкодинг-пати, собираю ответы и считаю голоса.\n\
                     \nЧто умею:\n\
                     • Запускать лобби и собирать игроков (/vibe_game, /vibe_join)\n\
                     • Давать промпты и собирать ответы (/vibe_round, /vibe <текст> или >vibe <текст>)\n\
                     • Запускать голосование кнопками и выбирать победителя\n\
                     • Вести счёт и показывать таблицу (/vibe_score)\n\
                     • Останавливать игру (/vibe_stop)\n\
                     \nКак начать: введите /vibe_game, затем приглашайте игроков командой /vibe_join и стартуйте раунд /vibe_round.";
        if let Err(err) = send_and_log(
            &bot,
            &state,
            chat_id,
            user_id,
            reply,
            Some(msg.id.0 as i64),
            None,
        )
        .await
        {
            tracing::error!("Failed to send /start reply: {err}");
        }
        return Ok(());
    }

    if is_command(text, "vibe_game") {
        if !state.allowed_users.is_empty() && !state.allowed_users.contains(&user_id) {
            let reply = "⛔ Доступ запрещен для этой команды.";
            if let Err(err) = send_and_log(
                &bot,
                &state,
                chat_id,
                user_id,
                reply,
                Some(msg.id.0 as i64),
                None,
            )
            .await
            {
                tracing::error!("Failed to send access denied: {err}");
            }
            return Ok(());
        }

        let reply = start_game(&state, chat_id, user_id, &user_name).await;
        if let Err(err) = send_and_log(
            &bot,
            &state,
            chat_id,
            user_id,
            &reply,
            Some(msg.id.0 as i64),
            None,
        )
        .await
        {
            tracing::error!("Failed to send start_game: {err}");
        }
        return Ok(());
    }

    if is_command(text, "vibe_join") {
        let reply = join_game(&state, chat_id, user_id, &user_name).await;
        if let Err(err) = send_and_log(
            &bot,
            &state,
            chat_id,
            user_id,
            &reply,
            Some(msg.id.0 as i64),
            None,
        )
        .await
        {
            tracing::error!("Failed to send join: {err}");
        }
        return Ok(());
    }

    if is_command(text, "vibe_stop") {
        let reply = stop_game(&state, chat_id, user_id).await;
        if let Err(err) = send_and_log(
            &bot,
            &state,
            chat_id,
            user_id,
            &reply,
            Some(msg.id.0 as i64),
            None,
        )
        .await
        {
            tracing::error!("Failed to send stop: {err}");
        }
        return Ok(());
    }

    if is_command(text, "vibe_score") {
        let reply = show_scores(&state, chat_id).await;
        if let Err(err) = send_and_log(
            &bot,
            &state,
            chat_id,
            user_id,
            &reply,
            Some(msg.id.0 as i64),
            None,
        )
        .await
        {
            tracing::error!("Failed to send score: {err}");
        }
        return Ok(());
    }

    if is_command(text, "vibe_round") {
        let prompt = random_prompt();
        let round_id = format!(
            "{}-{}",
            chat_id.0,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let reply = start_round(&state, chat_id, user_id, prompt.clone(), round_id.clone()).await;
        if let Err(err) = send_and_log(
            &bot,
            &state,
            chat_id,
            user_id,
            &reply,
            Some(msg.id.0 as i64),
            None,
        )
        .await
        {
            tracing::error!("Failed to send round start: {err}");
        }

        if reply.starts_with("🎯 Новый раунд") {
            schedule_round_close(bot, state.clone(), chat_id, round_id).await;
        }
        return Ok(());
    }

    if let Some(submission) = extract_submission(text) {
        let reply = submit_vibe(&state, chat_id, user_id, &user_name, submission).await;
        if let Err(err) = send_and_log(
            &bot,
            &state,
            chat_id,
            user_id,
            &reply,
            Some(msg.id.0 as i64),
            None,
        )
        .await
        {
            tracing::error!("Failed to send submission ack: {err}");
        }
        return Ok(());
    }

    Ok(())
}

async fn handle_callback(bot: Bot, state: AppState, query: CallbackQuery) -> ResponseResult<()> {
    let data = match &query.data {
        Some(d) => d,
        None => return Ok(()),
    };

    if !data.starts_with("vote|") {
        return Ok(());
    }

    let parts: Vec<&str> = data.split('|').collect();
    if parts.len() != 3 {
        return Ok(());
    }

    let round_id = parts[1].to_string();
    let target_id: i64 = match parts[2].parse() {
        Ok(id) => id,
        Err(_) => return Ok(()),
    };

    let voter_id = query.from.id.0 as i64;
    let (chat_id, message_id) = match query.message {
        Some(ref msg) => (msg.chat.id, msg.id),
        None => return Ok(()),
    };

    let mut new_markup = None;
    let mut new_text = None;
    let mut answer_text = "Голос принят ✅".to_string();

    {
        let mut games = state.games.write().await;
        if let Some(game) = games.get_mut(&chat_id.0) {
            if let Some(round) = &mut game.round {
                if round.round_id == round_id {
                    if matches!(round.status, RoundStatus::Voting) {
                        round.voter_choice.insert(voter_id, target_id);
                        let round_snapshot = round.clone();
                        let players = game.players.clone();
                        new_text = Some(vote_message_text(&players, &round_snapshot));
                        new_markup = Some(vote_markup(&players, &round_snapshot));
                    } else {
                        answer_text = "Раунд закрыт.".to_string();
                    }
                }
            }
        }
    }

    if let Some(text) = new_text {
        let markup = new_markup.unwrap_or_else(InlineKeyboardMarkup::default);
        if let Err(err) = bot
            .edit_message_text(chat_id, message_id, text.clone())
            .reply_markup(markup)
            .await
        {
            tracing::error!("Failed to edit vote message: {err}");
        }
        if let Err(err) = bot
            .answer_callback_query(query.id.clone())
            .text(answer_text)
            .await
        {
            tracing::error!("Failed to answer callback: {err}");
        }
    } else if let Err(err) = bot
        .answer_callback_query(query.id.clone())
        .text("Раунд уже закрыт.")
        .await
    {
        tracing::error!("Failed to answer callback (closed): {err}");
    }

    Ok(())
}

async fn schedule_round_close(bot: Bot, state: AppState, chat_id: ChatId, round_id: String) {
    let round_delay = state.round_duration;
    let vote_delay = state.vote_duration;
    let db = state.db.clone();

    tokio::spawn(async move {
        tokio::time::sleep(round_delay).await;

        // Move to voting if still collecting
        let (maybe_text, maybe_markup) = {
            let mut games = state.games.write().await;
            if let Some(game) = games.get_mut(&chat_id.0) {
                if let Some(round) = &mut game.round {
                    if round.round_id == round_id && matches!(round.status, RoundStatus::Collecting)
                    {
                        round.status = RoundStatus::Voting;
                        let round_snapshot = round.clone();
                        let players = game.players.clone();
                        let text = vote_message_text(&players, &round_snapshot);
                        let markup = vote_markup(&players, &round_snapshot);
                        (Some(text), Some(markup))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        };

        if let (Some(vote_text), Some(markup)) = (maybe_text, maybe_markup) {
            match bot
                .send_message(chat_id, vote_text.clone())
                .reply_markup(markup)
                .await
            {
                Ok(sent) => {
                    let message_id = sent.id.0;
                    let _ = db
                        .log_message(sent.id.0 as i64, chat_id.0, "outgoing", &vote_text, None)
                        .await;

                    let mut games = state.games.write().await;
                    if let Some(game) = games.get_mut(&chat_id.0) {
                        if let Some(round) = &mut game.round {
                            if round.round_id == round_id
                                && matches!(round.status, RoundStatus::Voting)
                            {
                                round.vote_message_id = Some(message_id);
                            }
                        }
                    }

                    schedule_vote_close(
                        bot.clone(),
                        state.clone(),
                        chat_id,
                        round_id.clone(),
                        vote_delay,
                    )
                    .await;
                }
                Err(err) => {
                    tracing::error!("Failed to send vote message: {err}");
                }
            }
        }
    });
}

async fn schedule_vote_close(
    bot: Bot,
    state: AppState,
    chat_id: ChatId,
    round_id: String,
    delay: Duration,
) {
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;

        let (summary, scoreboard) = {
            let mut games = state.games.write().await;
            if let Some(game) = games.get_mut(&chat_id.0) {
                if let Some(round) = game.round.take() {
                    if round.round_id == round_id && matches!(round.status, RoundStatus::Voting) {
                        let summary = finalize_round(&mut game.scores, &game.players, &round);
                        (Some(summary), format_scores(&game.scores, &game.players))
                    } else {
                        game.round = Some(round);
                        (None, String::new())
                    }
                } else {
                    (None, String::new())
                }
            } else {
                (None, String::new())
            }
        };

        if let Some(summary_text) = summary {
            let full = format!("{summary_text}\n\nНовый счёт:\n{scoreboard}");
            if let Ok(sent) = bot.send_message(chat_id, full.clone()).await {
                let _ = state
                    .db
                    .log_message(sent.id.0 as i64, chat_id.0, "outgoing", &full, None)
                    .await;
            }
        }
    });
}

fn finalize_round(
    scores: &mut HashMap<i64, i32>,
    players: &HashMap<i64, String>,
    round: &VibeRound,
) -> String {
    if round.voter_choice.is_empty() {
        return "Голоса не получены. Очки не начислены.".to_string();
    }

    let mut tally: HashMap<i64, i32> = HashMap::new();
    for target in round.voter_choice.values() {
        *tally.entry(*target).or_insert(0) += 1;
    }

    let max_votes = tally.values().copied().max().unwrap_or(0);
    let winners: Vec<i64> = tally
        .iter()
        .filter_map(|(uid, votes)| {
            if *votes == max_votes {
                Some(*uid)
            } else {
                None
            }
        })
        .collect();

    for uid in &winners {
        *scores.entry(*uid).or_insert(0) += 1;
    }

    let winner_names = winners
        .iter()
        .map(|uid| players.get(uid).cloned().unwrap_or_else(|| uid.to_string()))
        .collect::<Vec<_>>()
        .join(", ");

    format!("🏅 Побеждает: {winner_names} ({max_votes} голосов).")
}

async fn start_game(state: &AppState, chat_id: ChatId, host_id: i64, host_name: &str) -> String {
    let mut games = state.games.write().await;
    let mut game = GameState {
        host_id,
        host_name: host_name.to_string(),
        players: HashMap::new(),
        scores: HashMap::new(),
        round: None,
    };
    game.players.insert(host_id, host_name.to_string());
    games.insert(chat_id.0, game);

    format!(
        "🚀 Вайб-пати запущена. Хост: {}\nЖмите /vibe_join, чтобы зайти. Хост стартует раунды командой /vibe_round.",
        host_name
    )
}

async fn join_game(state: &AppState, chat_id: ChatId, user_id: i64, user_name: &str) -> String {
    let mut games = state.games.write().await;
    if let Some(game) = games.get_mut(&chat_id.0) {
        game.players.insert(user_id, user_name.to_string());
        format!("🤝 {} в лобби. Готовим вайбы!", user_name)
    } else {
        "Сначала запусти игру командой /vibe_game.".to_string()
    }
}

async fn stop_game(state: &AppState, chat_id: ChatId, user_id: i64) -> String {
    let mut games = state.games.write().await;
    if let Some(game) = games.get(&chat_id.0) {
        if game.host_id != user_id {
            return "Только хост может завершить игру.".to_string();
        }
    } else {
        return "Игра ещё не запущена.".to_string();
    }

    games.remove(&chat_id.0);
    "🛑 Игра остановлена.".to_string()
}

async fn show_scores(state: &AppState, chat_id: ChatId) -> String {
    let games = state.games.read().await;
    if let Some(game) = games.get(&chat_id.0) {
        let scoreboard = format_scores(&game.scores, &game.players);
        format!("🏆 Таблица очков:\n{}", scoreboard)
    } else {
        "Игра не запущена. /vibe_game чтобы начать.".to_string()
    }
}

async fn start_round(
    state: &AppState,
    chat_id: ChatId,
    user_id: i64,
    prompt: String,
    round_id: String,
) -> String {
    let mut games = state.games.write().await;

    let Some(game) = games.get_mut(&chat_id.0) else {
        return "Сначала запусти игру: /vibe_game.".to_string();
    };

    if game.host_id != user_id {
        return "Только хост может стартовать раунд.".to_string();
    }

    game.round = Some(VibeRound {
        round_id: round_id.clone(),
        prompt: prompt.clone(),
        submissions: HashMap::new(),
        voter_choice: HashMap::new(),
        vote_message_id: None,
        status: RoundStatus::Collecting,
    });

    format!(
        "🎯 Новый раунд!\nПромпт: {prompt}\n\nОтправь свой ответ: /vibe <текст> или >vibe <текст>\nУ тебя {} секунд.",
        state.round_duration.as_secs()
    )
}

async fn submit_vibe(
    state: &AppState,
    chat_id: ChatId,
    user_id: i64,
    user_name: &str,
    text: &str,
) -> String {
    let mut games = state.games.write().await;
    let Some(game) = games.get_mut(&chat_id.0) else {
        return "Сначала запусти игру командой /vibe_game.".to_string();
    };

    let Some(round) = game.round.as_mut() else {
        return "Сейчас нет активного сбора ответов. Хост: /vibe_round.".to_string();
    };

    if !matches!(round.status, RoundStatus::Collecting) {
        return "Сбор ответов закрыт. Жди голосование.".to_string();
    }

    game.players.insert(user_id, user_name.to_string());
    round.submissions.insert(user_id, text.trim().to_string());

    format!("✅ {user_name}, твой вайб записан.")
}

fn vote_message_text(players: &HashMap<i64, String>, round: &VibeRound) -> String {
    let mut lines = vec![
        "🗳 Голосование за лучший вайб".to_string(),
        format!("Промпт: {}", round.prompt),
        "".to_string(),
        "Участники:".to_string(),
    ];

    for (user_id, submission) in &round.submissions {
        let name = players
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| user_id.to_string());
        let preview = preview_text(submission, 140);
        lines.push(format!("• {name}: {preview}"));
    }

    lines.push("".to_string());
    lines.push("Нажми кнопку, чтобы проголосовать.".to_string());

    lines.join("\n")
}

fn vote_markup(players: &HashMap<i64, String>, round: &VibeRound) -> InlineKeyboardMarkup {
    let votes = {
        let mut counts = HashMap::new();
        for target in round.voter_choice.values() {
            *counts.entry(*target).or_insert(0) += 1;
        }
        counts
    };

    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut current_row: Vec<InlineKeyboardButton> = Vec::new();

    for user_id in round.submissions.keys() {
        let label = players
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| user_id.to_string());
        let count = votes.get(user_id).copied().unwrap_or(0);
        let text = format!("За {label} ({count})");
        let data = format!("vote|{}|{}", round.round_id, user_id);
        current_row.push(InlineKeyboardButton::callback(text, data));

        if current_row.len() == 2 {
            rows.push(current_row);
            current_row = Vec::new();
        }
    }

    if !current_row.is_empty() {
        rows.push(current_row);
    }

    InlineKeyboardMarkup::new(rows)
}

fn format_scores(scores: &HashMap<i64, i32>, players: &HashMap<i64, String>) -> String {
    if scores.is_empty() {
        return "Пока 0:0. Бросай /vibe_round, чтобы начать.".to_string();
    }

    let mut items: Vec<(i64, i32)> = scores.iter().map(|(k, v)| (*k, *v)).collect();
    items.sort_by(|a, b| b.1.cmp(&a.1));

    items
        .iter()
        .map(|(uid, pts)| {
            let name = players.get(uid).cloned().unwrap_or_else(|| uid.to_string());
            format!("{name} — {pts}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn preview_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_string();
    }
    let mut result = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i >= max_len {
            result.push('…');
            break;
        }
        result.push(ch);
    }
    result
}

fn is_command(text: &str, name: &str) -> bool {
    let base = format!("/{name}");
    text == base || text.starts_with(&(base.clone() + " ")) || text.starts_with(&(base + "@"))
}

fn extract_submission(text: &str) -> Option<&str> {
    if let Some(rest) = text.strip_prefix("/vibe") {
        let rest = rest
            .strip_prefix("@VibeCoderzBot")
            .unwrap_or(rest)
            .trim_start();
        if rest.is_empty() {
            None
        } else {
            Some(rest)
        }
    } else if let Some(rest) = text.strip_prefix(">vibe") {
        let trimmed = rest.trim_start();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    } else {
        None
    }
}

fn display_name(user: Option<&User>) -> String {
    match user {
        Some(u) => {
            if !u.first_name.is_empty() {
                return u.first_name.clone();
            }
            if let Some(username) = &u.username {
                if !username.is_empty() {
                    return username.clone();
                }
            }
            u.id.0.to_string()
        }
        None => "Игрок".to_string(),
    }
}

fn random_prompt() -> String {
    let mut rng = thread_rng();
    PROMPTS
        .choose(&mut rng)
        .unwrap_or(&"Придумай свой вайб-код!")
        .to_string()
}

fn env_or_default(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_allowed_users() -> HashSet<i64> {
    std::env::var("VIBECODING_ALLOWED_USERS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect()
}

async fn send_and_log(
    bot: &Bot,
    state: &AppState,
    chat_id: ChatId,
    user_id: i64,
    text: &str,
    reply_to: Option<i64>,
    markup: Option<InlineKeyboardMarkup>,
) -> ResponseResult<Message> {
    let mut req = bot.send_message(chat_id, text.to_string());
    if let Some(m) = markup {
        req = req.reply_markup(m);
    }

    let sent = req.await?;
    state
        .db
        .log_message(sent.id.0 as i64, user_id, "outgoing", text, reply_to)
        .await;
    Ok(sent)
}

struct MySqlLogger {
    pool: Pool,
    bot_name: String,
}

impl MySqlLogger {
    async fn new() -> Result<Self> {
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
        Ok(Self {
            pool,
            bot_name: BOT_NAME.to_string(),
        })
    }

    async fn save_user_if_needed(&self, user: Option<&User>) {
        let Some(u) = user else { return };
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
            "language_code" => u.language_code.clone().unwrap_or_default(),
            "is_premium" => u.is_premium,
            "is_bot" => u.is_bot,
        };

        if let Ok(mut conn) = self.pool.get_conn().await {
            let _ = conn.exec_drop(query, params).await;
        }
    }

    async fn log_message(
        &self,
        message_id: i64,
        user_id: i64,
        direction: &str,
        text: &str,
        reply_to: Option<i64>,
    ) {
        if direction != "incoming" && direction != "outgoing" {
            return;
        }

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

        if let Ok(mut conn) = self.pool.get_conn().await {
            let _ = conn.exec_drop(query, params).await;
        }
    }
}
