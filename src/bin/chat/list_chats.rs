//! List chats binary.

use telegram_reader::commands::list_chats;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    list_chats::run(20).await?;
    Ok(())
}
