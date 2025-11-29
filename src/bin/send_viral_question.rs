//! CLI-обёртка для отправки виральных вопросов.
//! Портирует поведение `send_viral_question.py`.

use telegram_reader::commands::send_viral;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    send_viral::run().await?;
    Ok(())
}
