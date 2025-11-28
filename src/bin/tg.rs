//! Simple chat export binary (equivalent to tg.py).

use std::env;
use telegram_reader::commands::tg;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    let chat = args.get(1).map(|s| s.as_str()).unwrap_or("chat_delta");
    tg::run(chat, 200).await?;
    Ok(())
}
