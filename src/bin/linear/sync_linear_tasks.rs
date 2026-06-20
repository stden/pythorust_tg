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

/// A single Linear task spec: (title, description, priority).
type TaskSpec = (&'static str, &'static str, i32);
/// A named category and its task specs.
type TaskCategory = (&'static str, &'static [TaskSpec]);

const TASKS: &[TaskCategory] = &[("completed", &[]), ("project_backlog", &[])];

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
                    println!(
                        "  ✅ {}: {}",
                        issue.identifier.as_deref().unwrap_or("?"),
                        title
                    );
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
