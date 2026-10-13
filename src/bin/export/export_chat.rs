//! Export chat binary.

use std::env;
use telegram_reader::commands::export;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    let username = args
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("Usage: export_chat <username> [output_file]"))?;
    let output = args.get(2).map(|s| s.as_str());
    export::run(username, output, 100).await?;
    Ok(())
}
