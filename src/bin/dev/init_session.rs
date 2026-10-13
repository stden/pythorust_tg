//! Session initialization binary.

use telegram_reader::commands::init_session;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    init_session::run().await?;
    Ok(())
}
