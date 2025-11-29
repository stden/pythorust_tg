//! Отправка заранее подготовленных вопросов в несколько чатов.
//! Порт Python-скрипта `send_viral_question.py`.

use std::time::Duration;

use grammers_client::types::peer::Peer;
use tokio::time::sleep;

use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

/// Вопрос с указанием подстроки, по которой ищем чат.
struct ViralQuestion {
    chat_match: &'static str,
    question: &'static str,
}

const QUESTIONS: &[ViralQuestion] = &[
    ViralQuestion {
        chat_match: "Golang GO",
        question: "Реально ли попасть в Яндекс/Авито на Go без олимпиадных регалий в 2025?\n\n\
        Или там только ICPC финалисты?\n\n\
        Кто проходил собесы недавно — что спрашивали, сколько этапов, какие алгоритмы?",
    },
    ViralQuestion {
        chat_match: "вайбкодеры",
        question: "Claude Haiku 4.5 vs GPT-4.5-mini: кто реально выиграл?\n\n\
        Anthropic говорят что \"лучше всех на рынке\", OpenAI молчит. \
        Кто тестил обе модели на реальных задачах (не бенчмарки)? \
        Поделитесь примерами где одна слила другую.",
    },
    ViralQuestion {
        chat_match: "Хара",
        question: "Какая самая безумная синхрония случалась в вашей жизни?\n\n\
        У меня: читала книгу про лотерею → получила 'случайные' числа → поставила → \
        выиграла ровно столько, сколько нужно было на книги.\n\n\
        Поделитесь своими историями 🙏✨",
    },
];

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

/// Отправляет вопросы в чаты, найденные по подстроке.
pub async fn run() -> Result<()> {
    println!("📤 Отправка виральных вопросов...");

    // Блокируем сессию на время отправки.
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    // Считаем диалоги заранее, чтобы не итерировать несколько раз.
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

    // Проходим по списку вопросов и отправляем.
    for (idx, question) in QUESTIONS.iter().enumerate() {
        let needle = question.chat_match.to_lowercase();
        let target = chats.iter().find(|c| c.title_lower.contains(&needle));

        if let Some(chat) = target {
            client
                .send_message(&chat.peer, question.question)
                .await
                .map_err(|e| Error::TelegramError(e.to_string()))?;

            println!("✅ [{}] Отправлено в '{}'", idx + 1, chat.title);
            // Лёгкая задержка как в Python-версии.
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
