//! Отправка виральных вопросов в чаты
//!
//! Sends viral questions to Telegram chats for maximum engagement

use grammers_client::types::peer::Peer;
use telegram_reader::error::{Error, Result};
use telegram_reader::session::{get_client, SessionLock};
use tokio::time::{sleep, Duration};

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

#[tokio::main]
async fn main() -> Result<()> {
    println!("📤 Отправка виральных вопросов...\n");

    // Question 1: Golang GO (highest expected engagement)
    let golang_question = r#"Реально ли попасть в Яндекс/Авито на Go без олимпиадных регалий в 2025?

Или там только ICPC финалисты?

Кто проходил собесы недавно — что спрашивали, сколько этапов, какие алгоритмы?"#;

    println!("📨 Отправка в 'Golang GO'...");
    match send_to_chat_by_name("Golang GO", golang_question).await {
        Ok(_) => println!(""),
        Err(e) => eprintln!("❌ Ошибка: {}\n", e),
    }

    // Wait between messages to avoid rate limits
    sleep(Duration::from_secs(3)).await;

    // Question 2: вайбкодеры (second highest expected engagement)
    let vibe_question = r#"Claude Haiku 4.5 vs GPT-4.5-mini: кто реально выиграл?

Anthropic говорят что "лучше всех на рынке", OpenAI молчит. Кто тестил обе модели на реальных задачах (не бенчмарки)? Поделитесь примерами где одна слила другую."#;

    println!("📨 Отправка в 'вайбкодеры'...");
    match send_to_chat_by_name("вайбкодеры", vibe_question).await {
        Ok(_) => println!(""),
        Err(e) => eprintln!("❌ Ошибка: {}\n", e),
    }

    sleep(Duration::from_secs(3)).await;

    // Question 3: Хара (spiritual community)
    let hara_question = r#"Какая самая безумная синхрония случалась в вашей жизни?

У меня: читала книгу про лотерею → получила 'случайные' числа → поставила → выиграла ровно столько, сколько нужно было на книги.

Поделитесь своими историями 🙏✨"#;

    println!("📨 Отправка в 'Хара'...");
    match send_to_chat_by_name("Хара", hara_question).await {
        Ok(_) => println!(""),
        Err(e) => eprintln!("❌ Ошибка: {}\n", e),
    }

    println!("\n✅ Отправка завершена!");
    println!("\n📊 Теперь отслеживайте реакции и отвечайте на комменты в первые 5 минут для максимального engagement.");
    println!("\n💡 Ожидаемые результаты:");
    println!("   - Golang GO: 60-120 реакций, 40+ комментов");
    println!("   - вайбкодеры: 50-100 реакций, 30+ комментов");
    println!("   - Хара: 30-60 реакций, 20+ комментов");

    Ok(())
}
