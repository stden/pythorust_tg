//! Delete Zoom messages binary.

use std::env;
use telegram_reader::commands::delete_zoom;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    let username = args.get(1).ok_or_else(|| anyhow::anyhow!("Usage: delete_zoom_messages <username>"))?;
    delete_zoom::run(username, 3000).await?;
    Ok(())
}
