//! Dialog evaluation CLI.
//!
//! Usage:
//!   cargo run --bin evaluate_dialogs -- --limit 5

use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use mysql_async::Pool;
use std::env;
use telegram_reader::analytics::DialogEvaluator;

#[derive(Parser, Debug)]
#[command(name = "evaluate_dialogs")]
#[command(about = "Evaluate bot dialogues using AI")]
struct Args {
    /// Number of recent sessions to evaluate
    #[arg(long, default_value = "5")]
    limit: u32,

    /// Model to use for evaluation
    #[arg(long, default_value = "gpt-4o")]
    model: String,
}

fn build_mysql_url() -> String {
    let host = env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".to_string());
    let database = env::var("MYSQL_DATABASE").unwrap_or_else(|_| "pythorust_tg".to_string());
    let user = env::var("MYSQL_USER").unwrap_or_else(|_| "pythorust_tg".to_string());
    let password = env::var("MYSQL_PASSWORD").unwrap_or_default();

    format!(
        "mysql://{}:{}@{}:{}/{}",
        user, password, host, port, database
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let pool = Pool::new(build_mysql_url().as_str());
    let evaluator = DialogEvaluator::new(pool.clone())?.with_model(args.model);

    let results = evaluator.evaluate_recent_sessions(args.limit).await?;

    println!("\n📊 Оценено диалогов: {}", results.len());

    pool.disconnect().await?;
    Ok(())
}
