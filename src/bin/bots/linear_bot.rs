//! Linear bot - creates Linear issues from Telegram messages.
//!
//! Usage: linear_bot

use telegram_reader::commands::linear_bot;
use telegram_reader::{get_client, SessionLock};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let _lock = SessionLock::acquire()?;
    let telegram = get_client().await?;

    linear_bot::run_linear_bot(telegram.client).await?;

    Ok(())
}
