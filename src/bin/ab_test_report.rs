//! A/B test report CLI.
//!
//! Usage:
//!   cargo run --bin ab_test_report -- --bot-name BFL_sales_bot --experiment bfl_prompt_ab

use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use mysql_async::Pool;
use std::env;
use telegram_reader::analytics::ab_testing::{fetch_ab_metrics, print_ab_report};

#[derive(Parser, Debug)]
#[command(name = "ab_test_report")]
#[command(about = "A/B test report for prompt experiments")]
struct Args {
    /// Bot name in database
    #[arg(long, default_value = "BFL_sales_bot")]
    bot_name: String,

    /// Experiment name (experiment_name in bot_experiments)
    #[arg(long, env = "BFL_PROMPT_EXPERIMENT", default_value = "bfl_prompt_ab")]
    experiment: String,

    /// Filter by date (last N days), default all records
    #[arg(long)]
    days: Option<u32>,
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

    let args = Args::parse();

    let pool = Pool::new(build_mysql_url().as_str());

    match fetch_ab_metrics(&pool, &args.bot_name, &args.experiment, args.days).await {
        Ok(metrics) => {
            print_ab_report(&metrics, &args.bot_name, &args.experiment, args.days);
        }
        Err(e) => {
            eprintln!("bot_experiments не найдена. Запустите бота с A/B менеджером и повторите.");
            eprintln!("SQL error: {}", e);
        }
    }

    pool.disconnect().await?;
    Ok(())
}
