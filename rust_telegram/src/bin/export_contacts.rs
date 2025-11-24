//! Export contacts from dialogs to JSON/CSV

use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::Serialize;
use telegram_reader::session::SessionLock;
use telegram_reader::get_client;

#[derive(Parser)]
#[command(name = "export_contacts")]
#[command(about = "Export Telegram contacts to JSON or CSV")]
struct Cli {
    /// Output format
    #[arg(short, long, default_value = "json")]
    format: OutputFormat,

    /// Output file (stdout if not specified)
    #[arg(short, long)]
    output: Option<String>,

    /// Limit number of dialogs to process
    #[arg(short, long, default_value = "200")]
    limit: usize,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Csv,
}

#[derive(Serialize)]
struct Contact {
    id: i64,
    name: String,
    is_user: bool,
    is_group: bool,
    is_channel: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let mut contacts: Vec<Contact> = Vec::new();
    let mut dialogs_iter = client.iter_dialogs();
    let mut count = 0;

    while let Some(dialog) = dialogs_iter.next().await? {
        if count >= cli.limit {
            break;
        }
        count += 1;

        let chat = &dialog.peer;
        let id: i64 = chat.id().to_string().parse().unwrap_or(0);
        let name = chat.name().unwrap_or("Unknown").to_string();

        // Determine chat type by checking id patterns
        // Users have positive IDs, groups/channels have negative
        let is_user = id > 0 && id < 1000000000000;
        let is_channel = id < 0 && id > -1000000000000;
        let is_group = id < -1000000000000;

        contacts.push(Contact {
            id,
            name,
            is_user,
            is_group,
            is_channel,
        });
    }

    let output = match cli.format {
        OutputFormat::Json => serde_json::to_string_pretty(&contacts)?,
        OutputFormat::Csv => {
            let mut csv = String::from("id,name,is_user,is_group,is_channel\n");
            for c in &contacts {
                csv.push_str(&format!(
                    "{},\"{}\",{},{},{}\n",
                    c.id,
                    c.name.replace('"', "\"\""),
                    c.is_user,
                    c.is_group,
                    c.is_channel
                ));
            }
            csv
        }
    };

    if let Some(path) = cli.output {
        std::fs::write(&path, &output)?;
        println!("Exported {} contacts to {}", contacts.len(), path);
    } else {
        println!("{}", output);
    }

    Ok(())
}
