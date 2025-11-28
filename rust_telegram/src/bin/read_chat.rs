//! Read chat binary (equivalent to read.py).

use std::env;
use telegram_reader::commands::read;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    let chat = args.get(1).map(|s| s.as_str()).unwrap_or("chat_alpha");
    // By default, delete unengaged messages (like the Python version)
    read::run(chat, None, true).await?;
    Ok(())
}
