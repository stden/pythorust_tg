//! Like messages from a specific user with contextual reactions

use anyhow::Result;
use chrono::{DateTime, Utc, Local};
use clap::Parser;
use telegram_reader::chat::resolve_chat;
use telegram_reader::config::{ChatEntity, Config};
use telegram_reader::session::SessionLock;
use telegram_reader::get_client;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "like_messages")]
#[command(about = "Like messages from a specific user")]
struct Cli {
    /// Chat name from config or username
    chat: String,

    /// User ID to like messages from
    user_id: i64,

    /// Message limit
    #[arg(short, long, default_value = "100")]
    limit: usize,

    /// Only like messages from today
    #[arg(long)]
    today: bool,

    /// Specific reaction emoji (otherwise contextual)
    #[arg(short, long)]
    reaction: Option<String>,

    /// Dry run - don't actually like
    #[arg(long)]
    dry_run: bool,
}

fn choose_reaction(text: &str) -> &'static str {
    let text_lower = text.to_lowercase();

    // Positive emotions
    if text_lower.contains("спасибо") || text_lower.contains("благодар") {
        return "🙏";
    }
    if text_lower.contains("люблю") || text_lower.contains("❤") || text_lower.contains("любовь") {
        return "❤";
    }
    if text_lower.contains("смешно") || text_lower.contains("ржу") || text_lower.contains("😂") || text_lower.contains("🤣") {
        return "😂";
    }
    if text_lower.contains("круто") || text_lower.contains("класс") || text_lower.contains("супер") || text_lower.contains("офигенн") {
        return "🔥";
    }
    if text_lower.contains("красив") || text_lower.contains("прекрасн") || text_lower.contains("восхит") {
        return "😍";
    }
    if text_lower.contains("грустно") || text_lower.contains("печаль") || text_lower.contains("жаль") {
        return "😢";
    }
    if text_lower.contains("ужас") || text_lower.contains("шок") || text_lower.contains("офигеть") {
        return "😱";
    }
    if text_lower.contains("поздравля") || text_lower.contains("день рожден") || text_lower.contains("праздник") {
        return "🎉";
    }
    if text_lower.contains("вопрос") || text_lower.contains("?") {
        return "🤔";
    }
    if text_lower.contains("еда") || text_lower.contains("вкусн") || text_lower.contains("готов") || text_lower.contains("рецепт") {
        return "😋";
    }

    // Default reactions
    let defaults = ["👍", "❤", "🔥", "👏", "😊"];
    let idx = text.len() % defaults.len();
    defaults[idx]
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let config = Config::new();
    let chat_entity = config
        .chats
        .get(&cli.chat)
        .cloned()
        .unwrap_or_else(|| ChatEntity::Username(cli.chat.clone()));

    let peer = resolve_chat(&client, &chat_entity).await?;

    let today_start = if cli.today {
        Some(Local::now().date_naive())
    } else {
        None
    };

    println!("Looking for messages from user {} in {}...\n", cli.user_id, cli.chat);

    let mut messages_iter = client.iter_messages(&peer);
    let mut count = 0;
    let mut liked = 0;

    while let Some(message) = messages_iter.next().await? {
        if count >= cli.limit {
            break;
        }
        count += 1;

        // Check if message is from target user
        let sender_id: i64 = message
            .sender()
            .map(|s| s.id().to_string().parse().unwrap_or(0))
            .unwrap_or(0);

        if sender_id != cli.user_id {
            continue;
        }

        // Check date if today filter enabled
        if let Some(today) = today_start {
            let msg_date = DateTime::from_timestamp(message.date().timestamp(), 0)
                .unwrap_or_else(Utc::now);
            if msg_date.date_naive() != today {
                continue;
            }
        }

        let text = message.text();
        let reaction = cli.reaction.as_deref().unwrap_or_else(|| choose_reaction(text));

        let msg_date = DateTime::from_timestamp(message.date().timestamp(), 0)
            .unwrap_or_else(Utc::now);
        let text_preview = truncate(text, 40);

        if cli.dry_run {
            println!("  WOULD LIKE: {} {} - {}", reaction, msg_date.format("%d.%m.%Y %H:%M"), text_preview);
        } else {
            println!("  {} {} - {}", reaction, msg_date.format("%d.%m.%Y %H:%M"), text_preview);
            // Note: grammers doesn't have a direct reaction API yet
            // This would need to use raw API calls
            // For now, just count what we would like
            liked += 1;

            // Rate limiting
            if liked % 10 == 0 {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }

    println!("\nProcessed {} messages, {} would be liked", count, liked);

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.is_empty() {
        "[media]".to_string()
    } else if s.chars().count() <= max_len {
        s.replace('\n', " ")
    } else {
        format!("{}...", s.chars().take(max_len).collect::<String>().replace('\n', " "))
    }
}
