//! AI Project Consultant with RAG - ИИ-консультант по проектам.
//!
//! Функции:
//! - Поиск по базе знаний (RAG)
//! - Консультации по техническим вопросам
//! - История диалога
//!
//! Usage:
//!   cargo run --bin ai_project_consultant -- --mode interactive
//!   AI_CONSULTANT_BOT_TOKEN=... cargo run --bin ai_project_consultant -- --mode telegram

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use dotenvy::dotenv;
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::Mutex;
use tracing::{error, info};
use walkdir::WalkDir;

use telegram_reader::integrations::openai::{ChatMessage, OpenAIClient};

const DEFAULT_SYSTEM_PROMPT: &str = r#"Ты - опытный технический консультант и архитектор решений.

Твои задачи:
1. Помогать с техническими вопросами по проектам
2. Предлагать архитектурные решения
3. Анализировать проблемы и предлагать исправления
4. Писать код и примеры реализации
5. Объяснять сложные концепции простым языком

Стиль общения:
- Конкретно и по делу
- С примерами кода
- Пошаговые инструкции
- Указываем потенциальные проблемы

Когда предлагаешь решение:
1. Анализируй контекст проекта
2. Проверь в базе знаний похожие случаи
3. Предложи оптимальное решение с обоснованием
4. Дай код/конфигурацию если нужно
5. Укажи на возможные подводные камни

Если информации недостаточно - задавай уточняющие вопросы."#;

#[derive(Parser)]
#[command(name = "ai_project_consultant")]
#[command(about = "AI Project Consultant with knowledge base RAG")]
struct Args {
    /// Run mode
    #[arg(long, default_value = "interactive")]
    mode: Mode,

    /// Path to knowledge base directory
    #[arg(long, env = "KNOWLEDGE_BASE_PATH")]
    knowledge_base: Option<PathBuf>,

    /// OpenAI model
    #[arg(long, env = "AI_CONSULTANT_MODEL", default_value = "gpt-4o-mini")]
    model: String,
}

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Interactive,
    Telegram,
}

/// Document from knowledge base.
struct KnowledgeDoc {
    #[allow(dead_code)]
    file: String,
    content: String,
}

/// AI Consultant with conversation history.
struct AIProjectConsultant {
    ai: OpenAIClient,
    model: String,
    knowledge_base: PathBuf,
    system_prompt: String,
    history: Vec<ChatMessage>,
}

impl AIProjectConsultant {
    fn new(ai: OpenAIClient, model: String, knowledge_base: PathBuf) -> Self {
        Self {
            ai,
            model,
            knowledge_base,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            history: Vec::new(),
        }
    }

    /// Index markdown files from knowledge base.
    fn index_knowledge_base(&self) -> Vec<KnowledgeDoc> {
        if !self.knowledge_base.exists() {
            return Vec::new();
        }

        let mut documents = Vec::new();

        for entry in WalkDir::new(&self.knowledge_base)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let relative = path
                        .strip_prefix(&self.knowledge_base)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();

                    // Take first 2000 chars
                    let truncated = if content.len() > 2000 {
                        content[..2000].to_string()
                    } else {
                        content
                    };

                    documents.push(KnowledgeDoc {
                        file: relative,
                        content: truncated,
                    });
                }
            }
        }

        info!("Indexed {} documents from knowledge base", documents.len());
        documents
    }

    /// Simple keyword search in knowledge base.
    fn search_knowledge_base(&self, query: &str, top_k: usize) -> Vec<String> {
        let documents = self.index_knowledge_base();
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(usize, &KnowledgeDoc)> = documents
            .iter()
            .map(|doc| {
                let content_lower = doc.content.to_lowercase();
                let score = query_words
                    .iter()
                    .filter(|word| content_lower.contains(*word))
                    .count();
                (score, doc)
            })
            .filter(|(score, _)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        scored
            .into_iter()
            .take(top_k)
            .map(|(_, doc)| doc.content.clone())
            .collect()
    }

    /// Get AI consultation.
    async fn consult(&mut self, user_message: &str, use_rag: bool) -> Result<String> {
        // RAG: search knowledge base
        let context_docs = if use_rag {
            self.search_knowledge_base(user_message, 3)
        } else {
            Vec::new()
        };

        // Build messages
        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: Some(self.system_prompt.clone()),
        }];

        // Add context from RAG
        if !context_docs.is_empty() {
            let context_text = context_docs
                .iter()
                .map(|doc| format!("Релевантная информация из базы знаний:\n{}", doc))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n");

            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(format!(
                    "Используй эту информацию для ответа:\n\n{}",
                    context_text
                )),
            });
        }

        // Add conversation history (last 10 messages)
        let history_start = self.history.len().saturating_sub(10);
        messages.extend(self.history[history_start..].iter().cloned());

        // Add current question
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
        });

        // Get AI response
        let response = self
            .ai
            .chat_completion(messages, &self.model, 0.3, 4096)
            .await?;

        // Save to history
        self.history.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
        });
        self.history.push(ChatMessage {
            role: "assistant".to_string(),
            content: Some(response.clone()),
        });

        Ok(response)
    }

    /// Clear conversation history.
    fn clear_history(&mut self) {
        self.history.clear();
        info!("Conversation history cleared");
    }
}

/// Interactive console mode.
async fn interactive_mode(args: &Args) -> Result<()> {
    println!("🤖 ИИ-консультант по проектам запущен");
    println!("Команды:");
    println!("  /clear - очистить историю");
    println!("  /exit - выход");
    println!("  /norag - вопрос без поиска в базе знаний");
    println!();

    let ai = OpenAIClient::from_env()?;
    let knowledge_base = args
        .knowledge_base
        .clone()
        .unwrap_or_else(|| PathBuf::from("knowledge_base"));

    let mut consultant = AIProjectConsultant::new(ai, args.model.clone(), knowledge_base);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("❓ Ваш вопрос: ");
        stdout.flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            break; // EOF
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/exit" => {
                println!("👋 До свидания!");
                break;
            }
            "/clear" => {
                consultant.clear_history();
                println!("✅ История очищена");
                continue;
            }
            _ => {}
        }

        let (use_rag, query) = if let Some(rest) = input.strip_prefix("/norag ") {
            (false, rest)
        } else {
            (true, input)
        };

        println!("\n🤔 Думаю...\n");

        match consultant.consult(query, use_rag).await {
            Ok(response) => {
                println!("🤖 Ответ:\n{}\n", response);
                println!("{}", "-".repeat(80));
                println!();
            }
            Err(e) => {
                println!("❌ Ошибка: {}", e);
            }
        }
    }

    Ok(())
}

/// User session for telegram bot.
type UserSessions = Arc<Mutex<HashMap<i64, AIProjectConsultant>>>;

/// Telegram bot mode.
async fn telegram_bot_mode(args: &Args) -> Result<()> {
    let token = env::var("AI_CONSULTANT_BOT_TOKEN").context("AI_CONSULTANT_BOT_TOKEN not set")?;

    let _ai = OpenAIClient::from_env()?;
    let model = args.model.clone();
    let knowledge_base = args
        .knowledge_base
        .clone()
        .unwrap_or_else(|| PathBuf::from("knowledge_base"));

    let sessions: UserSessions = Arc::new(Mutex::new(HashMap::new()));

    info!("Starting AI Consultant Telegram Bot...");

    let bot = Bot::new(token);

    let handler = dptree::entry()
        // /start command
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text() == Some("/start"))
                .endpoint(|bot: Bot, msg: Message| async move {
                    let text = "🤖 *ИИ-консультант по проектам*\n\n\
                        Я помогу с:\n\
                        • Техническими вопросами\n\
                        • Архитектурой решений\n\
                        • Отладкой проблем\n\
                        • Написанием кода\n\n\
                        Просто напишите свой вопрос!\n\n\
                        Команды:\n\
                        /clear - очистить историю\n\
                        /help - помощь";

                    bot.send_message(msg.chat.id, text)
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;
                    Ok::<_, anyhow::Error>(())
                }),
        )
        // /clear command
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text() == Some("/clear"))
                .endpoint({
                    let sessions = sessions.clone();
                    move |bot: Bot, msg: Message| {
                        let sessions = sessions.clone();
                        async move {
                            let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
                            let mut lock = sessions.lock().await;
                            if let Some(consultant) = lock.get_mut(&user_id) {
                                consultant.clear_history();
                            }
                            bot.send_message(msg.chat.id, "✅ История диалога очищена")
                                .await?;
                            Ok::<_, anyhow::Error>(())
                        }
                    }
                }),
        )
        // /help command
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text() == Some("/help"))
                .endpoint(|bot: Bot, msg: Message| async move {
                    let text = "📚 *Как пользоваться:*\n\n\
                        1. Просто напишите свой вопрос\n\
                        2. Я найду релевантную информацию в базе знаний\n\
                        3. Дам подробный ответ с примерами\n\n\
                        *Примеры вопросов:*\n\
                        • Как настроить N8N с Caddy?\n\
                        • Почему приложение недоступно извне?\n\
                        • Как интегрировать Telegram бота с Bitrix24?\n\
                        • Напиши скрипт для мониторинга сервиса";

                    bot.send_message(msg.chat.id, text)
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;
                    Ok::<_, anyhow::Error>(())
                }),
        )
        // Regular messages
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text().is_some_and(|t| !t.starts_with('/')))
                .endpoint({
                    let sessions = sessions.clone();
                    let model = model.clone();
                    let knowledge_base = knowledge_base.clone();
                    move |bot: Bot, msg: Message| {
                        let sessions = sessions.clone();
                        let model = model.clone();
                        let knowledge_base = knowledge_base.clone();
                        async move {
                            let text = msg.text().unwrap_or("");
                            let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);

                            // Get or create session
                            let mut lock = sessions.lock().await;
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                lock.entry(user_id)
                            {
                                let ai = OpenAIClient::from_env()?;
                                e.insert(AIProjectConsultant::new(
                                    ai,
                                    model.clone(),
                                    knowledge_base.clone(),
                                ));
                            }
                            let consultant = lock.get_mut(&user_id).unwrap();

                            // Send typing indicator
                            bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                                .await?;

                            match consultant.consult(text, true).await {
                                Ok(response) => {
                                    bot.send_message(msg.chat.id, response).await?;
                                }
                                Err(e) => {
                                    error!("AI error: {}", e);
                                    bot.send_message(msg.chat.id, format!("❌ Ошибка: {}", e))
                                        .await?;
                                }
                            }

                            Ok::<_, anyhow::Error>(())
                        }
                    }
                }),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.mode {
        Mode::Interactive => interactive_mode(&args).await,
        Mode::Telegram => telegram_bot_mode(&args).await,
    }
}
