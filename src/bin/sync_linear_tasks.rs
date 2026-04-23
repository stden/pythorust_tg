//! Sync tasks in Linear based on completed work.
//!
//! Usage: cargo run --bin sync_linear_tasks -- [--dry-run] [--category <category>]

use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use std::env;
use telegram_reader::linear::{CreateIssueInput, LinearClient};

#[derive(Parser)]
struct Args {
    /// Show what would be created without actually creating issues
    #[arg(long)]
    dry_run: bool,

    /// Create tasks only from the specified category
    #[arg(long)]
    category: Option<String>,
}

const TASKS: &[(&str, &[(&str, &str, i32)])] = &[
    (
        "completed",
        &[
            (
                "✅ Анализ чата Хара",
                "Проанализирован чат духовного сообщества Хара (369 сообщений)\n\n\
                Результат: создан промпт для дизайна сайта\n\
                Файл: prompts/hara_website_design.md\n\n\
                Включает:\n\
                - Цветовую палитру (золотой, фиолетовый, изумрудный)\n\
                - Типографику (Playfair Display, Inter/Raleway)\n\
                - Структуру из 6 страниц\n\
                - Интерактивные элементы (гадание, калькулятор нумерологии)\n\
                - 10 детальных секций",
                4,
            ),
            (
                "✅ Анализ потребностей вайбкодеров",
                "Проанализирован чат вайбкодеры (475 сообщений за 2 дня)\n\n\
                Результат: выявлено 12 продуктовых возможностей\n\
                Файл: analysis_results/vibecoders_needs_analysis.md\n\n\
                Топ-3 идеи с высоким потенциалом:\n\
                1. Figma Pixel-Perfect Plugin ($15-50/мес)\n\
                2. VoiceGPT Pro с интернетом ($20-40/мес)\n\
                3. CharacterHub - open source character AI\n\n\
                Прогноз: $1,750-13,500 MRR за 6 месяцев",
                4,
            ),
        ],
    ),
    (
        "hara_website",
        &[
            (
                "Дизайн сайта Хара: Выбор дизайнера/агентства",
                "Найти исполнителя для реализации дизайна\n\n\
                Опции:\n\
                1. Freelance дизайнер (Behance, Dribbble)\n\
                2. Дизайн-студия специализирующаяся на wellness\n\
                3. AI-генерация через Midjourney + доработка\n\
                4. Конкурс на 99designs\n\n\
                Бюджет: $1,500-5,000\n\
                Референсы: Gaia.com, The Wild Unknown\n\
                Промпт: prompts/hara_website_design.md",
                2,
            ),
            (
                "Дизайн сайта Хара: Колода карт ХАРА продуктовая страница",
                "Создать продуктовую страницу для предзаказа колоды\n\n\
                Требования:\n\
                - Галерея примеров карт\n\
                - Описание значений карт\n\
                - Форма предзаказа\n\
                - Countdown таймер до 10.12\n\
                - Интеграция оплаты\n\n\
                Дедлайн: до 10.12 (выпуск колоды)\n\
                Цена колоды: TBD",
                1,
            ),
            (
                "Дизайн сайта Хара: Интеграция календаря ретритов",
                "Добавить расписание онлайн-ретритов и эфиров\n\n\
                Функционал:\n\
                - Календарь событий\n\
                - Регистрация на ретрит\n\
                - Telegram-интеграция для напоминаний\n\
                - Профили ведущих (Ирина, Инна, Anna)\n\n\
                Технологии: Google Calendar API / Calendly",
                3,
            ),
        ],
    ),
    (
        "voice_gpt_pro",
        &[
            (
                "VoiceGPT Pro: Research конкурентов",
                "Исследовать существующие решения голосовых AI\n\n\
                Конкуренты:\n\
                - ChatGPT Voice Mode\n\
                - Claude mobile voice\n\
                - Perplexity Voice\n\
                - Google Assistant with Gemini\n\n\
                Анализ:\n\
                - Функционал и ограничения\n\
                - Ценообразование\n\
                - Отзывы пользователей\n\
                - Gap analysis\n\n\
                Время: 4-6 часов",
                3,
            ),
            (
                "VoiceGPT Pro: Техническая спецификация",
                "Написать детальный tech spec для голосового AI\n\n\
                Секции:\n\
                - Архитектура (STT + LLM + TTS + Web Search)\n\
                - Выбор моделей (Whisper, GPT-4o, ElevenLabs)\n\
                - Инфраструктура и costs\n\
                - Mobile app vs Web app\n\
                - Стратегия монетизации\n\n\
                Формат: codev/specs/voice-gpt-pro.md\n\
                Время: 8-10 часов",
                3,
            ),
        ],
    ),
    (
        "character_hub",
        &[
            (
                "CharacterHub: Анализ рынка Character.AI",
                "Исследовать рынок character AI и возможности\n\n\
                Анализ:\n\
                - Character.AI business model\n\
                - Размер рынка и прогнозы\n\
                - Open source альтернативы\n\
                - Grok Aurora функционал\n\
                - Потребности десктоп-версии для стримеров\n\n\
                Вывод из анализа вайбкодеров:\n\
                > 'Мне как идея очень зашло! Но не готов ради этого оплачивать Грок)'\n\n\
                Время: 6-8 часов",
                4,
            ),
        ],
    ),
];

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let team_key = env::var("LINEAR_TEAM_KEY").unwrap_or_else(|_| "".to_string());
    if team_key.is_empty() {
        eprintln!("❌ LINEAR_TEAM_KEY не задан (добавьте в .env).");
        std::process::exit(1);
    }

    let client = LinearClient::from_optional_key(None)?;

    let mut created = 0;
    let mut errors = 0;
    let mut skipped = 0;

    println!("{}", "=".repeat(60));
    println!("🔄 Синхронизация задач с Linear");
    println!("{}", "=".repeat(60));
    println!("Team: {}", team_key);
    println!("Dry run: {}", args.dry_run);
    println!();

    for (category, tasks) in TASKS {
        if let Some(ref cat_filter) = args.category {
            if category != cat_filter {
                continue;
            }
        }

        println!("\n📁 Категория: {}", category);
        println!("{}", "-".repeat(60));

        for (title, description, priority) in *tasks {
            if *category == "completed" {
                println!("  ⏭️  {} (архивная)", title);
                skipped += 1;
                continue;
            }

            if args.dry_run {
                println!("  🔍 [DRY RUN] {}", title);
                println!("     Priority: {}", priority);
                continue;
            }

            let input = CreateIssueInput {
                team_key: team_key.clone(),
                title: title.to_string(),
                description: Some(format!(
                    "{}\n\n---\nКатегория: {}\nСоздано автоматически из sync_linear_tasks.rs",
                    description, category
                )),
                project_id: None,
                priority: Some(*priority),
                assignee_id: None,
                label_ids: vec![],
            };

            match client.create_issue(input).await {
                Ok(issue) => {
                    println!("  ✅ {}: {}", issue.identifier.as_deref().unwrap_or("?"), title);
                    if let Some(url) = issue.url {
                        println!("     URL: {}", url);
                    }
                    created += 1;
                }
                Err(e) => {
                    eprintln!("  ❌ {}", title);
                    eprintln!("     Ошибка: {}", e);
                    errors += 1;
                }
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("📊 Результат:");
    println!("   ✅ Создано: {} задач", created);
    println!("   ❌ Ошибок: {}", errors);
    println!("   ⏭️  Пропущено: {} (архивные)", skipped);
    println!("{}", "=".repeat(60));

    if args.dry_run {
        println!("\n💡 Запустите без --dry-run для создания задач в Linear");
    }

    if errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
