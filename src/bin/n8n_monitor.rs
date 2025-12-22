//! N8N Monitor CLI.
//!
//! Usage:
//!   cargo run --bin n8n_monitor -- run     # Start monitoring loop
//!   cargo run --bin n8n_monitor -- check   # Single health check

use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use telegram_reader::n8n::N8NMonitor;

#[derive(Parser, Debug)]
#[command(name = "n8n_monitor")]
#[command(about = "N8N Service Monitor with Auto-Restart")]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug)]
enum Action {
    /// Start the monitoring loop
    Run,
    /// Run a single health check
    Check,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let mut monitor = N8NMonitor::from_env()?;

    match args.action {
        Action::Run => {
            // Handle Ctrl+C gracefully
            tokio::select! {
                result = monitor.monitor_loop() => {
                    result?;
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n👋 Monitor stopped by user");
                }
            }
        }
        Action::Check => {
            let is_healthy = monitor.run_check().await;
            std::process::exit(if is_healthy { 0 } else { 1 });
        }
    }

    Ok(())
}
