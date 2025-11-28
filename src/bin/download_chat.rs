//! Download chat by ID.
//!
//! Usage: download_chat <chat_id> [limit]

use clap::Parser;
use telegram_reader::commands::download_chat;
use telegram_reader::{get_client, SessionLock};

#[derive(Parser)]
#[command(name = "download_chat")]
#[command(about = "Download chat messages by channel ID")]
struct Args {
    /// Chat/Channel ID
    chat_id: i64,

    /// Message limit
    #[arg(default_value = "200")]
    limit: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let path = download_chat::download_chat(&client, args.chat_id, args.limit).await?;
    println!("Saved: {}", path);

    Ok(())
}
