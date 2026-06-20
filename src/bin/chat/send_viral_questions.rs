//! Sends viral questions to chats.
//!
//! Sends viral questions to Telegram chats for maximum engagement

use telegram_reader::error::{Error, Result};
use telegram_reader::grammers_client::types::peer::Peer;
use telegram_reader::session::{get_client, SessionLock};
use tokio::time::{sleep, Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ViralQuestion {
    chat_match: String,
    question: String,
}

/// Send message to chat by searching dialogs for matching name
async fn send_to_chat_by_name(chat_name: &str, message: &str) -> Result<()> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    // Search through all dialogs
    let mut dialogs = client.iter_dialogs();

    while let Some(dialog) = dialogs.next().await.transpose() {
        if let Ok(dialog) = dialog {
            let title = match &dialog.peer {
                Peer::User(u) => u.full_name(),
                Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
                Peer::Channel(c) => c.title().to_string(),
            };

            // Case-insensitive search
            if title.to_lowercase().contains(&chat_name.to_lowercase()) {
                client
                    .send_message(&dialog.peer, message)
                    .await
                    .map_err(|e| Error::TelegramError(e.to_string()))?;

                println!("✅ Отправлено в '{}'", title);
                return Ok(());
            }
        }
    }

    Err(Error::InvalidArgument(format!(
        "Чат '{}' не найден",
        chat_name
    )))
}

fn parse_viral_questions(raw: &str) -> Vec<ViralQuestion> {
    raw.split("||")
        .filter_map(|item| {
            let (chat_match, question) = item.split_once("::")?;
            let chat_match = chat_match.trim();
            let question = question.trim();
            if chat_match.is_empty() || question.is_empty() {
                return None;
            }
            Some(ViralQuestion {
                chat_match: chat_match.to_string(),
                question: question.to_string(),
            })
        })
        .collect()
}

fn viral_questions() -> Vec<ViralQuestion> {
    std::env::var("VIRAL_QUESTIONS")
        .map(|raw| parse_viral_questions(&raw))
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("📤 Отправка виральных вопросов...\n");

    let questions = viral_questions();
    if questions.is_empty() {
        return Err(Error::InvalidArgument(
            "VIRAL_QUESTIONS is empty. Use: chat::question||chat2::question2".to_string(),
        ));
    }

    for question in questions {
        println!("📨 Отправка в '{}'...", question.chat_match);
        match send_to_chat_by_name(&question.chat_match, &question.question).await {
            Ok(_) => println!("✓"),
            Err(e) => eprintln!("❌ Ошибка: {}\n", e),
        }
        sleep(Duration::from_secs(3)).await;
    }

    println!("\n✅ Отправка завершена!");
    println!("\n📊 Теперь отслеживайте реакции и отвечайте на комменты в первые 5 минут для максимального engagement.");
    println!("\n💡 Ожидаемые результаты:");
    println!("   - Golang GO: 60-120 реакций, 40+ комментов");
    println!("   - chat_beta: проверьте реакции и ответы");
    println!("   - chat_gamma: проверьте реакции и ответы");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_viral_questions_from_env_format() {
        let questions = parse_viral_questions("chat_one::Question one?||chat_two::Question two?");

        assert_eq!(
            questions,
            vec![
                ViralQuestion {
                    chat_match: "chat_one".to_string(),
                    question: "Question one?".to_string(),
                },
                ViralQuestion {
                    chat_match: "chat_two".to_string(),
                    question: "Question two?".to_string(),
                },
            ]
        );
    }
}
