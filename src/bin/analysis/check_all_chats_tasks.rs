//! Check all chats for tasks and important discussions.
//!
//! Analyzes recent messages from all chats and searches for:
//! - Action requests (@your_username do, need, must)
//! - Questions requiring answers
//! - Mentions of tasks, bugs, problems
//! - Important discussions
//!
//! Usage: cargo run --bin check_all_chats_tasks -- [--days 3] [--limit 100] [--output analysis_results/all_chats_tasks.md]

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use clap::Parser;
use dotenvy::dotenv;
use grammers_client::types::{Message, Peer};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use telegram_reader::chat::peer_name;
use telegram_reader::{get_client, SessionLock};

#[derive(Parser)]
struct Args {
    /// How many days back to check
    #[arg(long, default_value = "3")]
    days: i64,

    /// Maximum messages per chat
    #[arg(long, default_value = "100")]
    limit: usize,

    /// Path for saving the report
    #[arg(long, default_value = "analysis_results/all_chats_tasks.md")]
    output: PathBuf,
}

#[derive(Debug, Clone)]
struct Entry {
    text: String,
    date: DateTime<Utc>,
    sender: String,
}

struct ChatTaskChecker {
    my_username: String,
    tasks_by_chat: HashMap<String, Vec<Entry>>,
    questions_by_chat: HashMap<String, Vec<Entry>>,
    problems_by_chat: HashMap<String, Vec<Entry>>,
}

const TASK_KEYWORDS: &[&str] = &[
    "сделай",
    "нужно",
    "надо",
    "можешь",
    "помоги",
    "исправь",
    "добавь",
    "удали",
    "настрой",
    "проверь",
    "запусти",
];

const QUESTION_KEYWORDS: &[&str] = &["?", "как", "почему", "что делать", "где", "когда", "кто"];

const PROBLEM_KEYWORDS: &[&str] = &[
    "не работает",
    "ошибка",
    "баг",
    "сломалось",
    "проблема",
    "упало",
    "крашится",
    "зависло",
];

impl ChatTaskChecker {
    fn new(my_username: String) -> Self {
        Self {
            my_username,
            tasks_by_chat: HashMap::new(),
            questions_by_chat: HashMap::new(),
            problems_by_chat: HashMap::new(),
        }
    }

    async fn check_chat(
        &mut self,
        _client: &grammers_client::Client,
        message: &Message,
        chat_name: &str,
    ) -> Result<()> {
        let text = message.text();
        if text.is_empty() {
            return Ok(());
        }

        let text_lower = text.to_lowercase();
        let sender = if let Some(s) = message.sender() {
            peer_name(s)
        } else {
            "Unknown".to_string()
        };

        let entry = Entry {
            text: text.to_string(),
            date: message.date(),
            sender,
        };

        // Check for tasks
        if text_lower.contains(&format!("@{}", self.my_username.to_lowercase()))
            || (TASK_KEYWORDS.iter().any(|k| text_lower.contains(k)) && text.len() > 20)
        {
            self.tasks_by_chat
                .entry(chat_name.to_string())
                .or_default()
                .push(entry.clone());
        }

        // Check for questions
        if QUESTION_KEYWORDS.iter().any(|k| text_lower.contains(k)) {
            self.questions_by_chat
                .entry(chat_name.to_string())
                .or_default()
                .push(entry.clone());
        }

        // Check for problems
        if PROBLEM_KEYWORDS.iter().any(|k| text_lower.contains(k)) {
            self.problems_by_chat
                .entry(chat_name.to_string())
                .or_default()
                .push(entry);
        }

        Ok(())
    }

    fn generate_report(&self, output_file: &PathBuf) -> Result<()> {
        let mut lines = Vec::new();

        lines.push("# 📋 Задачи и обсуждения из всех чатов".to_string());
        lines.push("".to_string());
        lines.push(format!(
            "**Дата проверки:** {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));
        lines.push("".to_string());

        let all_chats: std::collections::HashSet<_> = self
            .tasks_by_chat
            .keys()
            .chain(self.questions_by_chat.keys())
            .chain(self.problems_by_chat.keys())
            .collect();

        let total_tasks: usize = self.tasks_by_chat.values().map(|v| v.len()).sum();
        let total_questions: usize = self.questions_by_chat.values().map(|v| v.len()).sum();
        let total_problems: usize = self.problems_by_chat.values().map(|v| v.len()).sum();

        lines.push("## 📊 Общая статистика".to_string());
        lines.push("".to_string());
        lines.push(format!("- **Чатов проверено:** {}", all_chats.len()));
        lines.push(format!("- **Задач найдено:** {}", total_tasks));
        lines.push(format!("- **Вопросов найдено:** {}", total_questions));
        lines.push(format!("- **Проблем найдено:** {}", total_problems));
        lines.push("".to_string());

        if !self.tasks_by_chat.is_empty() {
            lines.push("## ✅ Задачи по чатам".to_string());
            lines.push("".to_string());

            let mut sorted_chats: Vec<_> = self.tasks_by_chat.iter().collect();
            sorted_chats.sort_by_key(|x| std::cmp::Reverse(x.1.len()));

            for (chat_name, tasks) in sorted_chats {
                lines.push(format!("### {} ({} задач)", chat_name, tasks.len()));
                lines.push("".to_string());

                let mut sorted_tasks = tasks.clone();
                sorted_tasks.sort_by_key(|x| std::cmp::Reverse(x.date));

                for task in sorted_tasks.iter().take(5) {
                    lines.push(format!(
                        "**[{}] {}:**",
                        task.date.format("%Y-%m-%d %H:%M"),
                        task.sender
                    ));
                    let preview = if task.text.len() > 200 {
                        format!("{}...", &task.text[..200])
                    } else {
                        task.text.clone()
                    };
                    lines.push(format!("> {}", preview.replace('\n', "\n> ")));
                    lines.push("".to_string());
                }
            }
        }

        if !self.questions_by_chat.is_empty() {
            lines.push("## ❓ Вопросы по чатам".to_string());
            lines.push("".to_string());

            let mut sorted_chats: Vec<_> = self.questions_by_chat.iter().collect();
            sorted_chats.sort_by_key(|x| std::cmp::Reverse(x.1.len()));

            for (chat_name, questions) in sorted_chats {
                if questions.len() < 3 {
                    continue;
                }
                lines.push(format!("### {} ({} вопросов)", chat_name, questions.len()));
                lines.push("".to_string());

                let mut sorted_q = questions.clone();
                sorted_q.sort_by_key(|x| std::cmp::Reverse(x.date));

                for q in sorted_q.iter().take(3) {
                    lines.push(format!(
                        "**[{}] {}:**",
                        q.date.format("%Y-%m-%d %H:%M"),
                        q.sender
                    ));
                    let preview = if q.text.len() > 200 {
                        format!("{}...", &q.text[..200])
                    } else {
                        q.text.clone()
                    };
                    lines.push(format!("> {}", preview.replace('\n', "\n> ")));
                    lines.push("".to_string());
                }
            }
        }

        if !self.problems_by_chat.is_empty() {
            lines.push("## ⚠️ Проблемы по чатам".to_string());
            lines.push("".to_string());

            let mut sorted_chats: Vec<_> = self.problems_by_chat.iter().collect();
            sorted_chats.sort_by_key(|x| std::cmp::Reverse(x.1.len()));

            for (chat_name, problems) in sorted_chats {
                lines.push(format!("### {} ({} проблем)", chat_name, problems.len()));
                lines.push("".to_string());

                let mut sorted_p = problems.clone();
                sorted_p.sort_by_key(|x| std::cmp::Reverse(x.date));

                for p in sorted_p.iter().take(5) {
                    lines.push(format!(
                        "**[{}] {}:**",
                        p.date.format("%Y-%m-%d %H:%M"),
                        p.sender
                    ));
                    let preview = if p.text.len() > 200 {
                        format!("{}...", &p.text[..200])
                    } else {
                        p.text.clone()
                    };
                    lines.push(format!("> {}", preview.replace('\n', "\n> ")));
                    lines.push("".to_string());
                }
            }
        }

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
    let my_username = env::var("MY_NAME").unwrap_or_else(|_| "your_username".to_string());

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    println!("\n🔍 Проверяю чаты за последние {} дней...", args.days);
    println!("Лимит сообщений на чат: {}\n", args.limit);

    let offset_date = Utc::now() - Duration::days(args.days);
    let mut checker = ChatTaskChecker::new(my_username);

    let mut dialogs = client.iter_dialogs();
    while let Some(dialog) = dialogs.next().await? {
        // Only channels and groups
        if matches!(dialog.peer, Peer::User(_)) {
            continue;
        }
        // Skip broadcast channels
        if let Peer::Channel(ref c) = dialog.peer {
            if !c.raw.megagroup && c.raw.broadcast {
                continue;
            }
        }

        let chat_name = peer_name(&dialog.peer);
        let mut messages = client.iter_messages(&dialog.peer).limit(args.limit);
        let mut count = 0;
        let mut found_anything = false;

        while let Some(message) = messages.next().await? {
            if message.date() < offset_date {
                break;
            }
            checker.check_chat(&client, &message, &chat_name).await?;
            count += 1;
            found_anything = true;
        }

        if found_anything {
            println!("📌 {} (проверено {})", chat_name, count);
        }
    }

    checker.generate_report(&args.output)?;

    println!("\n✨ Готово!");
    Ok(())
}
