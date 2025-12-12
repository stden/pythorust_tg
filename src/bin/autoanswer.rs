//! AI auto-responder binary.

use telegram_reader::commands::autoanswer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    autoanswer::run("gpt-4o-mini").await?;
    Ok(())
}
