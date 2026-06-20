//! Read chat binary (equivalent to read.py).

use std::env;
use telegram_reader::commands::read;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    let default_chat =
        std::env::var("READ_DEFAULT_CHAT").unwrap_or_else(|_| "chat_alpha".to_string());
    let chat = args.get(1).map(String::as_str).unwrap_or(&default_chat);
    // By default, delete unengaged messages (like the Python version)
    read::run(chat, None, true, false).await?;
    Ok(())
}
