//! Analyze individual contacts and suggest how to be useful
//!
//! Reads history from individual chats and generates value propositions

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use grammers_client::types::peer::Peer;
use std::collections::HashMap;
use telegram_reader::session::{get_client, SessionLock};

#[derive(Parser)]
#[command(name = "analyze_contacts")]
#[command(about = "Analyze contacts and suggest value propositions")]
struct Args {
    /// Number of messages to read per chat
    #[arg(short, long, default_value = "50")]
    limit: usize,

    /// Maximum number of chats to analyze
    #[arg(short, long, default_value = "10")]
    chats: usize,
}

#[derive(Debug)]
struct ContactAnalysis {
    name: String,
    id: i64,
    message_count: usize,
    topics: Vec<String>,
    last_messages: Vec<String>,
    user_messages: Vec<String>,
    my_messages: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Acquire session lock
    let _lock = SessionLock::acquire()?;

    // Connect to Telegram
    let client = get_client().await?;

    // Get my user ID
    let me = client.get_me().await?;
    let my_id = me.raw.id();

    println!("📊 Анализ индивидуальных чатов\n");
    println!("Мой ID: {}\n", my_id);

    let mut analyses: Vec<ContactAnalysis> = Vec::new();
    let mut dialogs = client.iter_dialogs();
    let mut chat_count = 0;

    while let Some(dialog) = dialogs.next().await.transpose() {
        let dialog = dialog?;
        let chat = &dialog.peer;

        // Only process individual user chats (not bots)
        if let Peer::User(u) = chat {
            let is_bot = match &u.raw {
                grammers_tl_types::enums::User::User(user) => user.bot,
                grammers_tl_types::enums::User::Empty(_) => false,
            };

            if is_bot {
                continue;
            }

            let name = u.full_name();
            let user_id = u.raw.id();

            // Skip self and Telegram service
            if user_id == my_id || user_id == 777000 {
                continue;
            }

            println!("📱 Анализирую: {} (ID: {})", name, user_id);

            let mut user_messages = Vec::new();
            let mut my_messages = Vec::new();
            let mut all_messages = Vec::new();

            // Read messages
            let mut messages = client.iter_messages(chat);
            let mut count = 0;

            while let Some(msg) = messages.next().await.transpose() {
                let msg = msg?;

                let text = msg.text();
                if !text.is_empty() && text.len() > 5 {
                    // Check if message is from me or the other person
                    let sender_id = msg
                        .sender()
                        .map(|s| match s {
                            Peer::User(u) => u.raw.id(),
                            _ => 0,
                        })
                        .unwrap_or(0);

                    if sender_id == my_id {
                        my_messages.push(text.to_string());
                    } else {
                        user_messages.push(text.to_string());
                    }

                    all_messages.push(text.to_string());
                }

                count += 1;
                if count >= args.limit {
                    break;
                }
            }

            // Extract topics/keywords
            let topics = extract_topics(&all_messages);

            analyses.push(ContactAnalysis {
                name,
                id: user_id,
                message_count: all_messages.len(),
                topics,
                last_messages: all_messages.into_iter().take(10).collect(),
                user_messages: user_messages.into_iter().take(20).collect(),
                my_messages: my_messages.into_iter().take(20).collect(),
            });

            chat_count += 1;
            if chat_count >= args.chats {
                break;
            }
        }
    }

    // Print analysis
    println!("\n{}", "=".repeat(60));
    println!("📊 РЕЗУЛЬТАТЫ АНАЛИЗА");
    println!("{}\n", "=".repeat(60));

    for analysis in &analyses {
        println!("👤 {} (ID: {})", analysis.name, analysis.id);
        println!("   📨 Сообщений: {}", analysis.message_count);

        if !analysis.topics.is_empty() {
            println!("   🏷️  Темы: {}", analysis.topics.join(", "));
        }

        // Show last messages from user
        if !analysis.user_messages.is_empty() {
            println!("\n   💬 Последние сообщения от {}:", analysis.name);
            for (i, msg) in analysis.user_messages.iter().take(5).enumerate() {
                let preview = if msg.chars().count() > 100 {
                    format!("{}...", msg.chars().take(100).collect::<String>())
                } else {
                    msg.clone()
                };
                println!("      {}. {}", i + 1, preview.replace('\n', " "));
            }
        }

        // Suggest value proposition
        let suggestion = suggest_value(&analysis);
        println!("\n   💡 Чем могу быть полезен:");
        println!("      {}", suggestion);

        println!("\n{}\n", "-".repeat(60));
    }

    // Save to file
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("contact_analysis_{}.md", timestamp);

    let mut output = String::new();
    output.push_str("# Анализ контактов Telegram\n\n");
    output.push_str(&format!(
        "Дата: {}\n\n",
        Utc::now().format("%d.%m.%Y %H:%M")
    ));

    for analysis in &analyses {
        output.push_str(&format!("## {} (ID: {})\n\n", analysis.name, analysis.id));
        output.push_str(&format!("- Сообщений: {}\n", analysis.message_count));

        if !analysis.topics.is_empty() {
            output.push_str(&format!("- Темы: {}\n", analysis.topics.join(", ")));
        }

        let suggestion = suggest_value(&analysis);
        output.push_str(&format!(
            "\n### Чем могу быть полезен\n\n{}\n\n",
            suggestion
        ));

        output.push_str("---\n\n");
    }

    std::fs::write(&filename, output)?;
    println!("📄 Отчёт сохранён: {}", filename);

    Ok(())
}

/// Extract main topics from messages
fn extract_topics(messages: &[String]) -> Vec<String> {
    let mut word_count: HashMap<String, usize> = HashMap::new();

    // Keywords to look for
    let keywords = [
        "работа",
        "деньги",
        "бизнес",
        "проект",
        "встреча",
        "курс",
        "обучение",
        "консультация",
        "помощь",
        "вопрос",
        "идея",
        "разработка",
        "код",
        "программа",
        "сайт",
        "бот",
        "телеграм",
        "автоматизация",
        "AI",
        "ИИ",
        "нейросеть",
        "чат",
        "продажи",
        "маркетинг",
        "реклама",
        "клиенты",
        "трафик",
        "воронка",
        "психология",
        "отношения",
        "здоровье",
        "спорт",
        "путешествия",
    ];

    let text = messages.join(" ").to_lowercase();

    for keyword in keywords {
        let count = text.matches(keyword).count();
        if count > 0 {
            word_count.insert(keyword.to_string(), count);
        }
    }

    // Sort by count and take top topics
    let mut topics: Vec<_> = word_count.into_iter().collect();
    topics.sort_by(|a, b| b.1.cmp(&a.1));

    topics.into_iter().take(5).map(|(word, _)| word).collect()
}

/// Suggest how to be useful based on analysis
fn suggest_value(analysis: &ContactAnalysis) -> String {
    let all_text = analysis.user_messages.join(" ").to_lowercase();

    let mut suggestions = Vec::new();

    // Check for specific topics and suggest accordingly
    if all_text.contains("бот")
        || all_text.contains("телеграм")
        || all_text.contains("автоматизация")
    {
        suggestions.push("Разработка Telegram-ботов и автоматизация");
    }

    if all_text.contains("код") || all_text.contains("программ") || all_text.contains("разработ")
    {
        suggestions.push("Помощь с разработкой и программированием");
    }

    if all_text.contains("ai")
        || all_text.contains("ии")
        || all_text.contains("нейросет")
        || all_text.contains("gpt")
    {
        suggestions.push("Консультации по AI/LLM интеграциям");
    }

    if all_text.contains("бизнес") || all_text.contains("продаж") || all_text.contains("клиент")
    {
        suggestions.push("Автоматизация продаж и CRM");
    }

    if all_text.contains("сайт") || all_text.contains("web") || all_text.contains("landing") {
        suggestions.push("Разработка сайтов и лендингов");
    }

    if all_text.contains("консульт") || all_text.contains("совет") || all_text.contains("помо")
    {
        suggestions.push("Техническая консультация");
    }

    if suggestions.is_empty() {
        // Default suggestions based on message activity
        if analysis.message_count > 20 {
            suggestions.push("Активный контакт - стоит предложить сотрудничество");
        } else {
            suggestions.push("Написать и узнать актуальные потребности");
        }
    }

    suggestions.join("\n      ")
}
