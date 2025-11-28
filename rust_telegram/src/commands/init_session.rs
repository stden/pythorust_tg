//! Session initialization command
//!
//! Equivalent to Python's init_session.py

use std::io::{self, Write};

use crate::config::Config;
use crate::session::get_client_for_init;
use crate::error::{Result, Error};

pub async fn run() -> Result<()> {
    let config = Config::new();

    println!(
        r#"
╔═══════════════════════════════════════════════════════════════╗
║  ИНИЦИАЛИЗАЦИЯ НОВОЙ TELEGRAM СЕССИИ                          ║
╚═══════════════════════════════════════════════════════════════╝

⚠️  КРИТИЧЕСКОЕ ПРЕДУПРЕЖДЕНИЕ:
   Этот скрипт создаст НОВУЮ сессию для номера {}

   ЭТО ПРИВЕДЁТ К:
   - Выходу из Telegram на всех других устройствах
   - Потере активных сессий

   Вы УВЕРЕНЫ, что хотите продолжить?

   Введите 'YES' (заглавными) для подтверждения: "#,
        config.phone
    );

    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim() != "YES" {
        println!("\n❌ Отменено. Session файл не создан.");
        return Ok(());
    }

    println!("\n🔄 Создаю новую сессию для {}...", config.phone);
    println!("📱 Ожидайте код подтверждения в Telegram...\n");

    // Connect without existing session
    let client = get_client_for_init().await?;

    // Request login code
    let token = client
        .request_login_code(&config.phone, &config.api_hash)
        .await
        .map_err(|e| Error::TelegramError(format!("Failed to request code: {}", e)))?;

    println!("Введите код из Telegram: ");
    io::stdout().flush()?;

    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let code = code.trim();

    // Sign in
    let user = client
        .sign_in(&token, code)
        .await
        .map_err(|e| Error::TelegramError(format!("Failed to sign in: {}", e)))?;

    // Session is auto-saved by SqliteSession

    println!(
        r#"
╔═══════════════════════════════════════════════════════════════╗
║  ✅ СЕССИЯ УСПЕШНО СОЗДАНА                                    ║
╚═══════════════════════════════════════════════════════════════╝

Профиль:
  Имя: {}
  Username: @{}

Файл сессии: telegram_session.session

Теперь вы можете:
1. Запускать команды (read, tg, list-chats и т.д.)
2. Скрипты будут использовать эту сессию автоматически
3. НИКОГДА больше не запускайте init-session!

⚠️  ВАЖНО: Сделайте резервную копию файла telegram_session.session
"#,
        user.full_name(),
        user.username().unwrap_or("не указан"),
    );

    Ok(())
}
