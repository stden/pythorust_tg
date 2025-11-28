//! Download chat with a user by username.
//!
//! Usage: download_user_chat <username> [limit]

use clap::Parser;
use telegram_reader::commands::download_user_chat;
use telegram_reader::{get_client, SessionLock};

#[derive(Parser)]
#[command(name = "download_user_chat")]
#[command(about = "Download chat messages with a user by username")]
struct Args {
    /// Username (with or without @)
    username: String,

    /// Message limit
    #[arg(default_value = "500")]
    limit: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let path = download_user_chat::download_user_chat(&client, &args.username, args.limit).await?;
    println!("Saved: {}", path);

    Ok(())
}
