//! Get active chats binary.

use telegram_reader::commands::active_chats;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    active_chats::run(20).await?;
    Ok(())
}
