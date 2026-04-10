//! Collect ideas for improvement from all chats.
//!
//! Analyzes recent messages from chats and extracts:
//! - Requests for new features
//! - User problems
//! - Tool mentions
//! - Improvement ideas
//!
//! Usage: cargo run --bin collect_chat_ideas -- [--chats @vibecod3rs] [--days 7] [--limit 500] [--output analysis_results/chat_ideas.md]

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use clap::Parser;
use dotenvy::dotenv;
use grammers_client::types::{Message, Peer};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use telegram_reader::chat::{find_chat, peer_name};
use telegram_reader::{get_client, SessionLock};

#[derive(Parser)]
struct Args {
    /// Chats to analyze (e.g. @vibecod3rs)
    #[arg(long, default_values = &["@vibecod3rs"])]
    chats: Vec<String>,

    /// How many days back to analyze
    #[arg(long, default_value = "7")]
    days: i64,

    /// Maximum number of messages per chat
    #[arg(long, default_value = "500")]
    limit: usize,

    /// Path for saving the report
    #[arg(long, default_value = "analysis_results/chat_ideas.md")]
    output: PathBuf,
}

const FEATURE_KEYWORDS: &[&str] = &[
    "надо", "нужно", "хочу", "было бы классно", "предлагаю", "идея", "можно добавить",
    "круто было бы", "не хватает", "попросил", "запросил", "автоматизировать",
    "интеграция", "add", "feature", "implement", "need", "want", "idea",
];

const PROBLEM_KEYWORDS: &[&str] = &[
    "не работает", "сломалось", "баг", "ошибка", "проблема", "не могу", "не получается",
    "почему", "как сделать", "broken", "bug", "error", "issue", "problem", "doesn't work",
];

const TOOL_KEYWORDS: &[&str] = &[
    "cursor", "claude", "gpt", "chatgpt", "copilot", "github", "linear", "notion",
    "slack", "telegram", "n8n", "zapier", "api", "webhook", "bot", "automation",
    "ai", "ml", "docker", "kubernetes", "python", "rust", "typescript",
];

#[derive(Debug)]
struct Entry {
    text: String,
    date: DateTime<Utc>,
    sender: String,
    id: i32,
}

struct IdeaCollector {
    ideas: HashMap<String, Vec<Entry>>,
    problems: HashMap<String, Vec<Entry>>,
    tools: HashMap<String, HashSet<String>>,
}

impl IdeaCollector {
    fn new() -> Self {
        Self {
            ideas: HashMap::new(),
            problems: HashMap::new(),
            tools: HashMap::new(),
        }
    }

    async fn collect_from_chat(&mut self, client: &grammers_client::Client, chat_id: &str, days_back: i64, limit: usize) -> Result<()> {
        println!("\n📊 Анализирую чат: {}", chat_id);

        let peer = match find_chat(client, chat_id).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("❌ Не могу найти чат {}: {}", chat_id, e);
                return Ok(());
            }
        };

        let chat_name = peer_name(&peer);
        let offset_date = Utc::now() - Duration::days(days_back);

        let mut messages = client.iter_messages(&peer).limit(limit);
        let mut message_count = 0;

        while let Some(message) = messages.next().await? {
            if message.date() < offset_date {
                break;
            }

            let text = message.text();
            if text.is_empty() {
                continue;
            }

            message_count += 1;
            let text_lower = text.to_lowercase();

            // Check for ideas
            for keyword in FEATURE_KEYWORDS {
                if text_lower.contains(keyword) && text.len() > 20 {
                    let sender = self.get_sender_name(&message).await;
                    self.ideas.entry(chat_name.clone()).or_default().push(Entry {
                        text: text.to_string(),
                        date: message.date(),
                        sender,
                        id: message.id(),
                    });
                    break;
                }
            }

            // Check for problems
            for keyword in PROBLEM_KEYWORDS {
                if text_lower.contains(keyword) && text.len() > 20 {
                    let sender = self.get_sender_name(&message).await;
                    self.problems.entry(chat_name.clone()).or_default().push(Entry {
                        text: text.to_string(),
                        date: message.date(),
                        sender,
                        id: message.id(),
                    });
                    break;
                }
            }

            // Check for tools
            for tool in TOOL_KEYWORDS {
                let re = Regex::new(&format!(r"\b{}\b", regex::escape(tool)))?;
                if re.is_match(&text_lower) {
                    self.tools.entry(chat_name.clone()).or_default().insert(tool.to_string());
                }
            }
        }

        println!("  ✅ Проанализировано сообщений: {}", message_count);
        println!("  💡 Найдено идей: {}", self.ideas.get(&chat_name).map_or(0, |v| v.len()));
        println!("  ⚠️  Найдено проблем: {}", self.problems.get(&chat_name).map_or(0, |v| v.len()));
        println!("  🔧 Упомянуто инструментов: {}", self.tools.get(&chat_name).map_or(0, |v| v.len()));

        Ok(())
    }

    async fn get_sender_name(&self, message: &Message) -> String {
        if let Some(sender) = message.sender() {
            peer_name(&sender)
        } else {
            "Unknown".to_string()
        }
    }

    fn generate_report(&self, output_file: &PathBuf) -> Result<()> {
        let mut lines = Vec::new();

        lines.push("# 💡 Идеи по доработке из чатов".to_string());
        lines.push("".to_string());
        lines.push(format!("**Дата анализа:** {}", Utc::now().format("%Y-%m-%d %H:%M:%S")));
        lines.push("".to_string());

        let total_ideas: usize = self.ideas.values().map(|v| v.len()).sum();
        let total_problems: usize = self.problems.values().map(|v| v.len()).sum();
        let mut all_tools = HashSet::new();
        for t in self.tools.values() {
            for tool in t {
                all_tools.insert(tool);
            }
        }

        lines.push("## 📊 Общая статистика".to_string());
        lines.push("".to_string());
        lines.push(format!("- **Чатов проанализировано:** {}", self.ideas.len()));
        lines.push(format!("- **Идей найдено:** {}", total_ideas));
        lines.push(format!("- **Проблем найдено:** {}", total_problems));
        lines.push(format!("- **Уникальных инструментов:** {}", all_tools.len()));
        lines.push("".to_string());

        lines.push("## 💡 Идеи по чатам".to_string());
        lines.push("".to_string());

        let mut chat_names: Vec<_> = self.ideas.keys().collect();
        chat_names.sort();

        for chat_name in chat_names {
            if let Some(ideas) = self.ideas.get(chat_name) {
                if ideas.is_empty() { continue; }
                lines.push(format!("### {}", chat_name));
                lines.push("".to_string());
                lines.push(format!("Найдено идей: {}", ideas.len()));
                lines.push("".to_string());

                let mut sorted_ideas = ideas.clone();
                sorted_ideas.sort_by(|a, b| b.date.cmp(&a.date));

                for idea in sorted_ideas.iter().take(10) {
                    lines.push(format!("**[{}] {}:**", idea.date.format("%Y-%m-%d %H:%M"), idea.sender));
                    let preview = if idea.text.len() > 200 {
                        format!("{}...", &idea.text[..200])
                    } else {
                        idea.text.clone()
                    };
                    lines.push(format!("> {}", preview.replace('\n', "\n> ")));
                    lines.push("".to_string());
                }
            }
        }

        lines.push("## ⚠️ Проблемы по чатам".to_string());
        lines.push("".to_string());

        let mut prob_chat_names: Vec<_> = self.problems.keys().collect();
        prob_chat_names.sort();

        for chat_name in prob_chat_names {
            if let Some(problems) = self.problems.get(chat_name) {
                if problems.is_empty() { continue; }
                lines.push(format!("### {}", chat_name));
                lines.push("".to_string());
                lines.push(format!("Найдено проблем: {}", problems.len()));
                lines.push("".to_string());

                let mut sorted_problems = problems.clone();
                sorted_problems.sort_by(|a, b| b.date.cmp(&a.date));

                for problem in sorted_problems.iter().take(10) {
                    lines.push(format!("**[{}] {}:**", problem.date.format("%Y-%m-%d %H:%M"), problem.sender));
                    let preview = if problem.text.len() > 200 {
                        format!("{}...", &problem.text[..200])
                    } else {
                        problem.text.clone()
                    };
                    lines.push(format!("> {}", preview.replace('\n', "\n> ")));
                    lines.push("".to_string());
                }
            }
        }

        lines.push("## 🔧 Упоминаемые инструменты".to_string());
        lines.push("".to_string());

        let mut tool_chat_names: Vec<_> = self.tools.keys().collect();
        tool_chat_names.sort();

        for chat_name in tool_chat_names {
            if let Some(tools) = self.tools.get(chat_name) {
                if tools.is_empty() { continue; }
                let mut sorted_tools: Vec<_> = tools.iter().collect();
                sorted_tools.sort();
                let tools_str: Vec<_> = sorted_tools.into_iter().cloned().collect();
                lines.push(format!("### {}", chat_name));
                lines.push("".to_string());
                lines.push(format!("Инструменты: {}", tools_str.join(", ")));
                lines.push("".to_string());
            }
        }

        lines.push("## 🌟 Топ инструментов по популярности".to_string());
        lines.push("".to_string());

        let mut tool_counts = HashMap::new();
        for tools in self.tools.values() {
            for tool in tools {
                *tool_counts.entry(tool).or_insert(0) += 1;
            }
        }

        let mut sorted_counts: Vec<_> = tool_counts.into_iter().collect();
        sorted_counts.sort_by(|a, b| b.1.cmp(&a.1));

        for (tool, count) in sorted_counts.iter().take(20) {
            lines.push(format!("- **{}**: упомянут в {} чатах", tool, count));
        }

        lines.push("".to_string());
        lines.push("## 🎯 Рекомендации по доработке".to_string());
        lines.push("".to_string());
        lines.push("На основе анализа чатов, рекомендую:".to_string());
        lines.push("".to_string());
        lines.push("1. **Приоритетные фичи:** Проанализировать самые частые запросы".to_string());
        lines.push("2. **Критичные проблемы:** Исправить упомянутые баги".to_string());
        lines.push("3. **Интеграции:** Рассмотреть интеграции с популярными инструментами".to_string());
        lines.push("4. **Документация:** Создать гайды по решению частых проблем".to_string());
        lines.push("".to_string());

        if let Some(parent) = output_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_file, lines.join("\n"))?;

        println!("\n✅ Отчёт сохранён: {:?}", output_file);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    println!("🚀 Начинаю сбор идей из чатов...");
    println!("📅 Период анализа: последние {} дней", args.days);
    println!("📝 Лимит сообщений: {} на чат", args.limit);
    println!();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let mut collector = IdeaCollector::new();

    for chat in &args.chats {
        collector.collect_from_chat(&client, chat, args.days, args.limit).await?;
    }

    collector.generate_report(&args.output)?;

    println!("\n✨ Готово!");
    Ok(())
}
