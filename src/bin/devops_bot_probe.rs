//! Probe the DevOps AI bot via Telegram API.
//!
//! Example:
//!   cargo run --bin devops_bot_probe -- --bot @my_devops_bot --command /start --command /status

use std::cmp;
use std::time::{Duration, Instant};

use clap::Parser;
use grammers_client::client::UpdatesConfiguration;
use grammers_client::types::peer::Peer;
use grammers_client::types::update::Update;
use grammers_client::types::Message;
use grammers_session::defs::PeerId;
use serde::Serialize;
use telegram_reader::{
    chat::find_chat,
    error::{Error, Result},
    session::{get_client, SessionLock},
};

#[derive(Parser, Debug)]
#[command(name = "devops_bot_probe")]
#[command(about = "Send commands to the DevOps bot and read its replies")]
struct Args {
    /// Bot username, user ID, or chat name from config.yml
    #[arg(long, env = "DEVOPS_BOT_TARGET")]
    bot: String,

    /// Commands to send (comma-separated or repeated flags)
    #[arg(
        short = 'c',
        long = "command",
        value_delimiter = ',',
        default_value = "/start,/status"
    )]
    commands: Vec<String>,

    /// Timeout per command in seconds
    #[arg(long, default_value_t = 25)]
    timeout_seconds: u64,

    /// Output results as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct CommandResult {
    command: String,
    response: Option<String>,
    duration_ms: u128,
    timed_out: bool,
    error: Option<String>,
}

async fn latest_message_id(
    client: &telegram_reader::session::TelegramClient,
    peer: &Peer,
) -> Result<i32> {
    let mut iter = client.iter_messages(peer);
    if let Some(msg) = iter.next().await.transpose() {
        let msg = msg.map_err(|e| Error::TelegramError(e.to_string()))?;
        Ok(msg.id())
    } else {
        Ok(0)
    }
}

fn format_response_text(msg: &Message) -> String {
    let text = msg.text().trim();
    if !text.is_empty() {
        return text.to_string();
    }

    if msg.media().is_some() {
        "[media message]".to_string()
    } else {
        "[empty message]".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().init();

    let args = Args::parse();
    let timeout = Duration::from_secs(args.timeout_seconds);

    let _lock = SessionLock::acquire()?;
    let mut client = get_client().await?;

    let bot_peer = find_chat(&client, &args.bot).await?;
    let target_peer_id: PeerId = bot_peer.id();

    let updates_rx = client.take_updates().ok_or_else(|| {
        Error::TelegramError("Updates stream already taken. Restart the probe.".into())
    })?;

    let mut updates = client.stream_updates(
        updates_rx,
        UpdatesConfiguration {
            catch_up: false,
            ..Default::default()
        },
    );

    let mut results = Vec::new();

    for command in args.commands.iter() {
        let baseline_id = latest_message_id(&client, &bot_peer).await.unwrap_or(0);
        let start = Instant::now();
        let mut response: Option<String> = None;
        let mut error: Option<String> = None;

        if let Err(err) = client.send_message(&bot_peer, command).await {
            error = Some(format!("send failed: {}", err));
        } else {
            let deadline = start + timeout;
            while Instant::now() < deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let wait_for = cmp::min(remaining, Duration::from_secs(5));

                match tokio::time::timeout(wait_for, updates.next()).await {
                    Ok(Ok(Update::NewMessage(msg))) => {
                        if msg.peer_id() != target_peer_id
                            || msg.outgoing()
                            || msg.id() <= baseline_id
                        {
                            continue;
                        }
                        response = Some(format_response_text(&msg));
                        break;
                    }
                    Ok(Ok(_)) => continue,
                    Ok(Err(err)) => {
                        error = Some(err.to_string());
                        break;
                    }
                    Err(_) => continue, // chunk timeout
                }
            }
        }

        let duration_ms = start.elapsed().as_millis();
        let timed_out = response.is_none() && error.is_none();

        results.push(CommandResult {
            command: command.to_string(),
            response,
            duration_ms,
            timed_out,
            error,
        });
    }

    if args.json {
        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| Error::SerializationError(e.to_string()))?;
        println!("{json}");
    } else {
        for result in &results {
            if let Some(err) = &result.error {
                println!("{} -> error: {}", result.command, err);
                continue;
            }

            if result.timed_out {
                println!(
                    "{} -> timed out after {}ms",
                    result.command, result.duration_ms
                );
            } else if let Some(resp) = &result.response {
                println!("{} -> {} ({}ms)", result.command, resp, result.duration_ms);
            } else {
                println!(
                    "{} -> no response ({}ms)",
                    result.command, result.duration_ms
                );
            }
        }
    }

    Ok(())
}
