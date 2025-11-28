//! Тест Google Gemini API.

use dotenvy::dotenv;
use telegram_reader::integrations::GeminiClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Загружаем .env из корня проекта
    dotenv().ok();

    println!("🔄 Тест Google Gemini API...\n");

    let client = GeminiClient::from_env()?;
    println!("✅ Клиент создан, модель: gemini-2.0-flash\n");

    // Тест простого чата
    println!("📤 Отправляю: \"Привет! Скажи одно слово.\"");
    let response = client.chat("Привет! Скажи одно слово.").await?;
    println!("📥 Ответ: {}\n", response);

    // Тест с системным промптом
    println!("📤 Тест с системным промптом...");
    let response = client
        .chat_with_system(
            "Сколько будет 2+2?",
            Some("Ты калькулятор. Отвечай только числами."),
        )
        .await?;
    println!("📥 Ответ: {}\n", response);

    println!("✅ Все тесты пройдены!");
    Ok(())
}
