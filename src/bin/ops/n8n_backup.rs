//! N8N Backup CLI.
//!
//! Usage:
//!   cargo run --bin n8n_backup -- backup
//!   cargo run --bin n8n_backup -- list
//!   cargo run --bin n8n_backup -- cleanup
//!   cargo run --bin n8n_backup -- restore --file /path/to/backup.tar.gz

use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use std::path::PathBuf;
use telegram_reader::n8n::N8NBackup;

#[derive(Parser, Debug)]
#[command(name = "n8n_backup")]
#[command(about = "N8N Configuration Backup Manager")]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug)]
enum Action {
    /// Create a new backup
    Backup,
    /// Restore from a backup file
    Restore {
        /// Path to backup file
        #[arg(long)]
        file: PathBuf,
    },
    /// List all available backups
    List,
    /// Remove old backups based on retention policy
    Cleanup,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let backup = N8NBackup::from_env()?;

    match args.action {
        Action::Backup => {
            let archive = backup.create_backup().await?;
            println!("✅ Backup created: {}", archive.display());
        }
        Action::Restore { file } => {
            backup.restore_backup(&file).await?;
        }
        Action::List => {
            let backups = backup.list_backups().await?;
            if backups.is_empty() {
                println!("📦 No backups found");
            } else {
                println!("📦 Available backups ({}):", backups.len());
                for entry in backups {
                    println!("  {}", entry);
                }
            }
        }
        Action::Cleanup => {
            backup.cleanup_old_backups().await?;
        }
    }

    Ok(())
}
