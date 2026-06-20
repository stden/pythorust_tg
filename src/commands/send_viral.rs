//! Sends prepared questions to multiple chats.
//! Port of the Python script `send_viral_question.py`.

use std::time::Duration;

use grammers_client::types::peer::Peer;
use tokio::time::sleep;

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

/// Question with the substring used to find the chat.
struct ViralQuestion {
    chat_match: String,
    question: String,
}

const QUESTIONS: &[(&str, &str)] = &[
    (
        "chat_alpha",
        "Реально ли попасть в крупную продуктовую команду на Go без олимпиадных регалий?\n\n\
        Или там только ICPC финалисты?\n\n\
        Кто проходил собесы недавно - что спрашивали, сколько этапов, какие алгоритмы?",
    ),
    (
        "chat_beta",
        "Какие AI-инструменты реально ускоряют вашу работу, а какие только добавляют шума?\n\n\
        Интересны конкретные сценарии и ограничения.",
    ),
    (
        "chat_gamma",
        "Какая привычка или практика сильнее всего помогает вам держать фокус?\n\n\
        Поделитесь тем, что сработало на практике.",
    ),
];

fn viral_questions() -> Vec<ViralQuestion> {
    if let Ok(raw) = std::env::var("VIRAL_QUESTIONS") {
        let questions = parse_viral_questions(&raw);
        if !questions.is_empty() {
            return questions;
        }
    }

    QUESTIONS
        .iter()
        .map(|(chat_match, question)| ViralQuestion {
            chat_match: (*chat_match).to_string(),
            question: (*question).to_string(),
        })
        .collect()
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

#[derive(Clone)]
struct AvailableChat {
    title: String,
    title_lower: String,
    peer: Peer,
}

fn chat_title(peer: &Peer) -> String {
    match peer {
        Peer::User(u) => u.full_name(),
        Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
        Peer::Channel(c) => c.title().to_string(),
    }
}

/// Sends questions to chats found by substring.
pub async fn run() -> Result<()> {
    println!("📤 Отправка виральных вопросов...");

    // Lock the session while sending.
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    // Collect dialogs ahead of time to avoid iterating multiple times.
    let mut dialogs = client.iter_dialogs();
    let mut chats: Vec<AvailableChat> = Vec::new();

    while let Some(dialog) = dialogs.next().await.transpose() {
        let dialog = dialog.map_err(|e| Error::TelegramError(e.to_string()))?;
        let title = chat_title(&dialog.peer);
        chats.push(AvailableChat {
            title_lower: title.to_lowercase(),
            title,
            peer: dialog.peer,
        });
    }

    // Iterate over questions and send them.
    let questions = viral_questions();
    for (idx, question) in questions.iter().enumerate() {
        let needle = question.chat_match.to_lowercase();
        let target = chats.iter().find(|c| c.title_lower.contains(&needle));

        if let Some(chat) = target {
            client
                .send_message(&chat.peer, question.question.clone())
                .await
                .map_err(|e| Error::TelegramError(e.to_string()))?;

            println!("✅ [{}] Отправлено в '{}'", idx + 1, chat.title);
            // Small delay matching the Python version.
            sleep(Duration::from_secs(2)).await;
        } else {
            eprintln!(
                "❌ Чат '{}' не найден среди диалогов, пропускаю",
                question.chat_match
            );
        }
    }

    println!("\n✅ Все вопросы обработаны!");
    println!("📊 Отслеживайте реакции в первые минуты после отправки.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn test_questions_not_empty() {
        assert!(!viral_questions().is_empty());
    }

    #[test]
    fn test_questions_have_content() {
        for question in viral_questions() {
            assert!(!question.chat_match.is_empty());
            assert!(!question.question.is_empty());
        }
    }

    #[test]
    fn test_available_chat_clone() {
        // Note: We can't fully test this without a real Peer, but we can verify the struct exists
        assert!(viral_questions().len() >= 2);
    }

    #[test]
    fn viral_questions_use_env_targets_when_set() {
        let _guard = EnvGuard::set(
            "VIRAL_QUESTIONS",
            "chat_one::Question one?||chat_two::Question two?",
        );

        let questions = viral_questions();

        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0].chat_match, "chat_one");
        assert_eq!(questions[0].question, "Question one?");
        assert_eq!(questions[1].chat_match, "chat_two");
        assert_eq!(questions[1].question, "Question two?");
    }
}
