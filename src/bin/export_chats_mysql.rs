//! CLI entrypoint to export all dialogs into MySQL (`telegram_chats`).

use clap::Parser;
use telegram_reader::commands::export_chats_mysql;

#[derive(Parser)]
#[command(name = "export_chats_mysql")]
#[command(about = "Export all Telegram chats into MySQL")]
struct Args {
    /// Limit number of dialogs to export (0 = all)
    #[arg(long, default_value_t = 0)]
    max_dialogs: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    export_chats_mysql::run(args.max_dialogs).await?;

    Ok(())
}
