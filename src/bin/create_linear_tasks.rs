use std::env;
use std::process;
use telegram_reader::linear::{LinearClient, CreateIssueInput};
use tokio;

struct TaskDef {
    category: &'static str,
    title: &'static str,
    description: &'static str,
    priority: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let team_key = env::var("LINEAR_TEAM_KEY")
        .unwrap_or_default()
        .trim()
        .to_string();
    if team_key.is_empty() {
        eprintln!("LINEAR_TEAM_KEY is not set (add it to .env).");
        process::exit(1);
    }
    
    let api_key = env::var("LINEAR_API_KEY").unwrap_or_default().trim().to_string();
    if api_key.is_empty() {
        eprintln!("LINEAR_API_KEY is not set.");
        process::exit(1);
    }

    let client = match LinearClient::new(api_key) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error initializing Linear: {}", e);
            process::exit(1);
        }
    };

    let tasks = vec![
        TaskDef { category: "urgent", title: "Настроить Calendly для консультаций", description: "Регистрация аккаунта, настройка слотов для консультаций по AI/нейросетям", priority: 1 },
        TaskDef { category: "urgent", title: "Написать pitch-шаблон для саппорт автопилота", description: "Шаблон письма для B2B продаж AI саппорта. Целевая аудитория: компании с 1-й линией поддержки", priority: 1 },
        TaskDef { category: "urgent", title: "Пост в LinkedIn про AI автоматизацию", description: "20 мин - написать и опубликовать пост с кейсами AI автоматизации", priority: 1 },
        TaskDef { category: "urgent", title: "Составить список 10 компаний для pitch", description: "Компании для продажи саппорт автопилота: SaaS, e-commerce, финтех", priority: 1 },
        
        TaskDef { category: "this_week", title: "Отправить 3 pitch письма саппорт автопилот", description: "Первые cold outreach письма потенциальным клиентам", priority: 2 },
        TaskDef { category: "this_week", title: "Запланировать партнёрские созвоны", description: "Согласовать даты и формат сотрудничества с потенциальными партнёрами", priority: 2 },
        TaskDef { category: "this_week", title: "Подготовить демо для онлайн-встречи", description: "Обновить сценарий демо для новой аудитории", priority: 2 },
        TaskDef { category: "this_week", title: "Уточнить статус учебных материалов", description: "Уточнить сроки поставки материалов у поставщиков", priority: 3 },
        
        TaskDef { category: "projects", title: "Telegram → Linear бот MVP", description: "Превращение сообщений в задачи с автотегами. Быстрый старт, минимальная разработка", priority: 2 },
        TaskDef { category: "projects", title: "Саппорт/Helpdesk автопилот - specs", description: "Specs готовы: codev/specs/0002-helpdesk-autopilot.md. Следующий шаг: поиск первого клиента", priority: 2 },
        TaskDef { category: "projects", title: "HR/AI продукт для собеседований", description: "Specs готовы: codev/specs/0003-hr-ai-interviewer.md. Потенциал 200-600K/мес", priority: 3 },
        TaskDef { category: "projects", title: "Голосовой AI-продавец", description: "Spec: codev/specs/spec-2025-11-23-neuro-sales-agent.md. Требуется план", priority: 3 },
        TaskDef { category: "projects", title: "Автодайджест чатов", description: "Дневные/недельные резюме чатов chat_alpha, chat_beta, chat_gamma", priority: 3 },
        
        TaskDef { category: "infrastructure", title: "n8n - настроить workflow automation", description: "Open-source, self-hosted. Интеграция с 400+ сервисами", priority: 3 },
        TaskDef { category: "infrastructure", title: "Оплатить браузерный инструмент для ресёрча", description: "Разобраться с оплатой", priority: 4 },
        TaskDef { category: "infrastructure", title: "Rust Telegram - новая сессия grammers", description: "Билды готовы, нужна новая сессия для grammers 0.8", priority: 3 },
        
        TaskDef { category: "wellness", title: "Изучить варианты холистических курсов", description: "Исследовать доступные программы, оценить расписание и стоимость", priority: 4 },
        
        TaskDef { category: "events", title: "Открытый клуб в конце месяца", description: "18:00-20:00, формат обсуждения, участие по регистрации", priority: 2 },
        TaskDef { category: "events", title: "Тематическая лекция", description: "TBD - выбрать тему и спикера", priority: 3 },
        
        TaskDef { category: "fintech", title: "Платёжный сервис - развитие", description: "USDT платёжный сервис. Статус: в разработке/продвижении", priority: 3 },
        TaskDef { category: "fintech", title: "Crypto Card продвижение", description: "Криптокарта, план маркетинга", priority: 3 },
        TaskDef { category: "fintech", title: "Google Ads для крипто", description: "Требуется верифицированный аккаунт", priority: 4 },
        
        TaskDef { category: "learning", title: "Алгоритм Ленстры - подготовить материалы", description: "Сложность, шаги, применение к Ax<=b в Python. Созвон после 19:00 в среду", priority: 3 },
        TaskDef { category: "learning", title: "Консультации по нейросетям - развивать", description: "5,000-15,000 руб/час. Воркшопы 80,000-200,000 руб/день", priority: 2 },
    ];

    let mut created = 0;
    let mut errors = 0;

    let mut current_category = "";

    for task in tasks {
        if task.category != current_category {
            println!("\n📁 Категория: {}", task.category);
            println!("{:-<40}", "");
            current_category = task.category;
        }

        let description = format!("{}\n\nКатегория: {}", task.description, task.category);
        
        let input = CreateIssueInput {
            team_key: team_key.clone(),
            title: task.title.to_string(),
            description: Some(description),
            project_id: None,
            priority: Some(task.priority),
            assignee_id: None,
            label_ids: vec![],
        };
        
        match client.create_issue(input).await {
            Ok(issue) => {
                println!("  ✅ {}: {}", issue.identifier.unwrap_or_default(), issue.title);
                println!("     URL: {}", issue.url.unwrap_or_default());
                created += 1;
            }
            Err(e) => {
                println!("  ❌ {}: {}", task.title, e);
                errors += 1;
            }
        }
    }

    println!("\n{:=<50}", "");
    println!("📊 Результат: создано {} задач, ошибок {}", created, errors);

    Ok(())
}