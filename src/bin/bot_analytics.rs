//! Bot analytics CLI.
//!
//! Usage:
//!   cargo run --bin bot_analytics -- --days 30

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use dotenvy::dotenv;
use mysql_async::Pool;
use std::env;
use std::path::PathBuf;
use telegram_reader::analytics::BotAnalytics;

#[derive(Parser, Debug)]
#[command(name = "bot_analytics")]
#[command(about = "Bot analytics dashboard: conversions, funnel, retention")]
struct Args {
    /// Bot names to include (default: all bot_names from DB)
    #[arg(long)]
    bots: Option<Vec<String>>,

    /// Time window in days (default: 30)
    #[arg(long, default_value = "30")]
    days: u32,

    /// Path to save Markdown dashboard
    #[arg(long)]
    output: Option<PathBuf>,
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
    let analytics = BotAnalytics::new(pool.clone());

    let metrics = analytics.analyze(args.bots, args.days).await?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let output_path = args.output.unwrap_or_else(|| {
        PathBuf::from("analysis_results").join(format!("bot_analytics_{}.md", timestamp))
    });

    BotAnalytics::render_markdown(&metrics, args.days, &output_path).await?;

    pool.disconnect().await?;
    Ok(())
}
