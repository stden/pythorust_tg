//! Send message to Telegram
//!
//! Usage:
//!   cargo run --bin send_message -- <target> <message>
//!
//! Target can be:
//!   - User ID: 123456789
//!   - Username: @username
//!   - Chat name: chat_name (from config)

use clap::Parser;
use telegram_reader::commands::send_message;
use telegram_reader::Result;

#[derive(Parser)]
#[command(name = "send_message")]
#[command(about = "Send message to Telegram user or chat")]
struct Args {
    /// Target: user ID, @username, or chat name from config
    #[arg(index = 1)]
    target: String,

    /// Message text to send
    #[arg(index = 2)]
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter("telegram_reader=info")
        .init();

    send_message::run(&args.target, &args.message).await
}
