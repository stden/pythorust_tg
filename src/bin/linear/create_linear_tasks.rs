use serde::Deserialize;
use std::env;
use std::process;
use telegram_reader::linear::{CreateIssueInput, LinearClient};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct TaskDef {
    category: String,
    title: String,
    description: String,
    priority: i32,
}

fn tasks_from_env() -> anyhow::Result<Vec<TaskDef>> {
    let raw = env::var("LINEAR_TASKS_JSON")
        .unwrap_or_default()
        .trim()
        .to_string();
    if raw.is_empty() {
        anyhow::bail!("LINEAR_TASKS_JSON is not set.");
    }
    parse_tasks_json(&raw)
}

fn parse_tasks_json(raw: &str) -> anyhow::Result<Vec<TaskDef>> {
    let tasks: Vec<TaskDef> = serde_json::from_str(raw)?;
    if tasks.is_empty() {
        anyhow::bail!("LINEAR_TASKS_JSON contains no tasks.");
    }
    Ok(tasks)
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

    let api_key = env::var("LINEAR_API_KEY")
        .unwrap_or_default()
        .trim()
        .to_string();
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

    let tasks = tasks_from_env()?;

    let mut created = 0;
    let mut errors = 0;

    let mut current_category = String::new();

    for task in tasks {
        if task.category != current_category {
            println!("\n📁 Категория: {}", task.category);
            println!("{:-<40}", "");
            current_category = task.category.clone();
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
                println!(
                    "  ✅ {}: {}",
                    issue.identifier.unwrap_or_default(),
                    issue.title
                );
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
