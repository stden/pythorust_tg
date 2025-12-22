//! Telegram Reader CLI - main entry point
//!
//! This is the unified CLI interface for all Telegram operations.

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Instant;
use tracing_subscriber::EnvFilter;

use telegram_reader::{commands, metrics};
use tracing::warn;

#[derive(Parser)]
#[command(name = "telegram_reader")]
#[command(about = "Telegram Chat Reader & Auto-responder", long_about = None)]
#[command(version)]
struct Cli {
    /// Address to expose Prometheus metrics (e.g., 0.0.0.0:9898)
    #[arg(long, env = "METRICS_ADDR")]
    metrics_addr: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read chat messages and export to markdown
    Read {
        /// Chat name from config (e.g., chat_alpha, chat_beta, chat_gamma)
        #[arg(default_value = "chat_alpha")]
        chat: String,

        /// Maximum number of messages to fetch
        #[arg(short, long)]
        limit: Option<usize>,

        /// Delete messages without reactions or replies
        #[arg(short, long, default_value = "false")]
        delete_unengaged: bool,

        /// Watch chat in real time and log new messages
        #[arg(long, default_value_t = false)]
        watch: bool,
    },

    /// Simple chat export (tg.py equivalent)
    Tg {
        /// Chat name from config
        #[arg(default_value = "chat_delta")]
        chat: String,

        /// Maximum number of messages to fetch
        #[arg(short, long, default_value = "200")]
        limit: usize,
    },

    /// List active chats
    ListChats {
        /// Number of chats to display
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Filter by chat type: all, users, groups, channels
        #[arg(short, long, default_value = "all")]
        filter: String,
    },

    /// Get most active chats
    ActiveChats {
        /// Number of chats to display
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Get dialogs with metadata
    Dialogs {
        /// Maximum number of dialogs to fetch
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Output format: table | json | yaml
        #[arg(long, default_value = "table")]
        format: String,

        /// Optional output file to save results
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Export a single chat by username
    Export {
        /// Username to export (without @)
        username: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Maximum number of messages
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },

    /// Delete Zoom messages from a chat
    DeleteZoom {
        /// Username to clean (without @)
        username: String,

        /// Maximum messages to scan
        #[arg(short, long, default_value = "3000")]
        limit: usize,
    },

    /// Analyze chat content with AI (categorization, insights)
    Analyze {
        /// Chat username/ID/alias to analyze
        chat: String,

        /// LLM provider: openai | claude | gemini | ollama
        #[arg(long, default_value = "openai")]
        provider: String,

        /// Model name (defaults per provider)
        #[arg(long)]
        model: Option<String>,

        /// Maximum number of messages to analyze
        #[arg(long, default_value = "1000")]
        limit: usize,

        /// Days to look back
        #[arg(long, default_value = "30")]
        days: i64,

        /// Output format: json | markdown | both
        #[arg(long, default_value = "both")]
        output_format: String,

        /// Output directory
        #[arg(long, default_value = "analysis_results")]
        output_dir: PathBuf,

        /// Custom prompt file (Markdown)
        #[arg(long)]
        prompt: Option<PathBuf>,

        /// Suppress verbose logging
        #[arg(long, default_value_t = false)]
        quiet: bool,

        /// Include media-only messages
        #[arg(long, default_value_t = false)]
        include_media: bool,

        /// Include bot messages
        #[arg(long, default_value_t = false)]
        include_bots: bool,

        /// Minimum message length
        #[arg(long, default_value = "10")]
        min_length: usize,

        /// Temperature for LLM sampling
        #[arg(long, default_value = "0.3")]
        temperature: f32,

        /// Max tokens for LLM response
        #[arg(long, default_value = "2000")]
        max_tokens: u32,
    },

    /// Start AI auto-responder
    AutoAnswer {
        /// OpenAI model to use
        #[arg(short, long, default_value = "gpt-4o-mini")]
        model: String,
    },

    /// Initialize a new session (use only once!)
    InitSession,

    /// Create a Linear issue via GraphQL API
    Linear {
        /// Linear API key (fallback: LINEAR_API_KEY)
        #[arg(long, env = "LINEAR_API_KEY")]
        api_key: Option<String>,

        /// Linear team key (fallback: LINEAR_TEAM_KEY)
        #[arg(long, env = "LINEAR_TEAM_KEY")]
        team: Option<String>,

        /// Title for the issue
        #[arg(short, long)]
        title: String,

        /// Optional description
        #[arg(short, long)]
        description: Option<String>,

        /// Optional projectId
        #[arg(long, env = "LINEAR_PROJECT_ID")]
        project: Option<String>,

        /// Priority 0..4 (default 1)
        #[arg(long, env = "LINEAR_PRIORITY", default_value_t = 1)]
        priority: i32,

        /// Assign to user id
        #[arg(long, env = "LINEAR_ASSIGNEE_ID")]
        assignee: Option<String>,

        /// Label IDs, comma-separated or repeated
        #[arg(long, value_delimiter = ',', env = "LINEAR_LABEL_IDS")]
        labels: Vec<String>,
    },

    /// Generate AI-powered chat digest/summary
    Digest {
        /// Chat name to analyze
        chat: String,

        /// Period in hours (default: 24)
        #[arg(short = 'H', long, default_value = "24")]
        hours: i64,

        /// Maximum messages to analyze
        #[arg(short, long, default_value = "500")]
        limit: usize,

        /// OpenAI model to use
        #[arg(short, long, default_value = "gpt-4o-mini")]
        model: String,
    },

    /// Moderate chat - filter profanity
    Moderate {
        /// Chat name to moderate
        chat: String,

        /// Delete messages with profanity (requires admin rights)
        #[arg(short, long)]
        delete: bool,

        /// Send warning messages
        #[arg(short, long, default_value = "true")]
        warn: bool,
    },

    /// Analyze chat for profanity statistics
    ProfanityStats {
        /// Chat name to analyze
        chat: String,

        /// Maximum messages to analyze
        #[arg(short, long, default_value = "1000")]
        limit: usize,
    },

    /// Parse chat for CRM data (contacts, deals, action items)
    Crm {
        /// Chat name to analyze
        chat: String,

        /// Maximum messages to analyze
        #[arg(short, long, default_value = "100")]
        limit: usize,

        /// OpenAI model to use
        #[arg(short, long, default_value = "gpt-4o-mini")]
        model: String,

        /// Export contacts to CSV file
        #[arg(long)]
        export_csv: Option<String>,
    },

    /// Like (react to) messages from a specific user in a chat
    Like {
        /// Chat name to search (partial match)
        #[arg(short, long)]
        chat: String,

        /// User name to find messages from (partial match)
        #[arg(short, long)]
        user: String,

        /// Emoji to use for reaction
        #[arg(short, long, default_value = "❤️")]
        emoji: String,

        /// Maximum messages to scan
        #[arg(short, long, default_value = "500")]
        limit: usize,
    },

    /// Send reactions to specific messages (ids/links) or latest messages
    React {
        /// Chat alias/name/@username/id
        #[arg(short, long)]
        chat: String,

        /// Reaction emoji (default ❤️)
        #[arg(short, long, default_value = "❤️")]
        emoji: String,

        /// Message ids or t.me links (space/comma separated)
        #[arg(long, num_args = 0.., value_delimiter = ',')]
        ids: Vec<String>,

        /// File with message ids or t.me links
        #[arg(long)]
        file: Option<PathBuf>,

        /// React to last N messages (optional)
        #[arg(long, default_value_t = 0)]
        recent: usize,

        /// Filter recent messages by sender id
        #[arg(long)]
        user_id: Option<i64>,

        /// Delay between reactions in milliseconds
        #[arg(long, default_value_t = 600)]
        delay_ms: u64,

        /// Preview only, no reactions sent
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Send predefined viral questions to multiple chats
    SendViral,

    /// N8N service monitor with auto-restart
    N8nMonitor,

    /// N8N backup management
    N8nBackup {
        /// Action: backup, list, cleanup, restore
        action: String,

        /// Backup file for restore action
        #[arg(long)]
        file: Option<std::path::PathBuf>,
    },

    /// Hunt for users matching specific criteria
    Hunt {
        /// Chat name(s) to search, comma-separated
        #[arg(short, long, value_delimiter = ',')]
        chats: Vec<String>,

        /// Keywords to search (any match)
        #[arg(short, long, value_delimiter = ',')]
        keywords: Vec<String>,

        /// Required keywords (must all match)
        #[arg(short, long, value_delimiter = ',')]
        required: Vec<String>,

        /// Keywords to exclude
        #[arg(short, long, value_delimiter = ',')]
        exclude: Vec<String>,

        /// Minimum messages from user
        #[arg(long, default_value = "1")]
        min_messages: usize,

        /// Look back N days
        #[arg(long, default_value = "30")]
        days: i64,

        /// Maximum messages to scan per chat
        #[arg(short, long, default_value = "1000")]
        limit: usize,

        /// Export results to CSV
        #[arg(long)]
        export_csv: Option<String>,

        /// Maximum results to display
        #[arg(long, default_value = "50")]
        top: usize,
    },
}

impl Commands {
    fn name(&self) -> &'static str {
        match self {
            Commands::Read { .. } => "read",
            Commands::Tg { .. } => "tg",
            Commands::ListChats { .. } => "list_chats",
            Commands::ActiveChats { .. } => "active_chats",
            Commands::Dialogs { .. } => "dialogs",
            Commands::Export { .. } => "export",
            Commands::DeleteZoom { .. } => "delete_zoom",
            Commands::Analyze { .. } => "analyze",
            Commands::AutoAnswer { .. } => "autoanswer",
            Commands::InitSession => "init_session",
            Commands::Linear { .. } => "linear",
            Commands::Digest { .. } => "digest",
            Commands::Moderate { .. } => "moderate",
            Commands::ProfanityStats { .. } => "profanity_stats",
            Commands::Crm { .. } => "crm",
            Commands::Like { .. } => "like",
            Commands::SendViral => "send_viral",
            Commands::N8nMonitor => "n8n_monitor",
            Commands::N8nBackup { .. } => "n8n_backup",
            Commands::React { .. } => "react",
            Commands::Hunt { .. } => "hunt",
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env for local development
    let _ = dotenvy::dotenv();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("telegram_reader=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    if let Some(addr) = cli.metrics_addr.as_deref() {
        match addr.parse::<SocketAddr>() {
            Ok(socket) => metrics::spawn_metrics_server(socket),
            Err(err) => warn!(%addr, "Invalid metrics address: {}", err),
        }
    }

    let command_name = cli.command.name();
    metrics::record_command_start(command_name);
    let start = Instant::now();

    let result = execute_command(cli.command).await;

    metrics::record_command_result(command_name, start.elapsed(), result.is_ok());

    result
}

async fn execute_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Read {
            chat,
            limit,
            delete_unengaged,
            watch,
        } => {
            commands::read::run(&chat, limit, delete_unengaged, watch).await?;
        }
        Commands::Tg { chat, limit } => {
            commands::tg::run(&chat, limit).await?;
        }
        Commands::ListChats { limit, filter } => {
            let chat_filter = match filter.to_lowercase().as_str() {
                "users" | "user" => commands::list_chats::ChatFilter::Users,
                "groups" | "group" => commands::list_chats::ChatFilter::Groups,
                "channels" | "channel" => commands::list_chats::ChatFilter::Channels,
                _ => commands::list_chats::ChatFilter::All,
            };
            commands::list_chats::run_with_filter(limit, chat_filter).await?;
        }
        Commands::ActiveChats { limit } => {
            commands::active_chats::run(limit).await?;
        }
        Commands::Dialogs {
            limit,
            format,
            output,
        } => {
            commands::dialogs_run(limit, &format, output).await?;
        }
        Commands::Export {
            username,
            output,
            limit,
        } => {
            commands::export::run(&username, output.as_deref(), limit).await?;
        }
        Commands::DeleteZoom { username, limit } => {
            commands::delete_zoom::run(&username, limit).await?;
        }
        Commands::AutoAnswer { model } => {
            commands::autoanswer::run(&model).await?;
        }
        Commands::Analyze {
            chat,
            provider,
            model,
            limit,
            days,
            output_format,
            output_dir,
            prompt,
            quiet,
            include_media,
            include_bots,
            min_length,
            temperature,
            max_tokens,
        } => {
            let cfg = commands::chat_analyzer::AnalyzerConfig {
                message_limit: limit,
                days_back: days,
                llm_provider: commands::chat_analyzer::LlmProvider::parse(&provider),
                model,
                temperature,
                max_tokens,
                min_message_length: min_length,
                include_media,
                exclude_bots: !include_bots,
                output_format: commands::chat_analyzer::OutputFormat::parse(&output_format),
                output_dir,
                prompt_path: prompt,
                verbose: !quiet,
            };

            let result = commands::chat_analyzer::run(&chat, cfg).await?;
            println!(
                "Chat: {}\nCategory: {}\nSentiment: {}\nMessages analyzed: {}",
                result.chat_name,
                result.category,
                result.sentiment,
                result.activity_metrics.total_messages
            );
        }
        Commands::InitSession => {
            commands::init_session::run().await?;
        }
        Commands::Linear {
            api_key,
            team,
            title,
            description,
            project,
            priority,
            assignee,
            labels,
        } => {
            commands::linear::run(commands::linear::LinearArgs {
                api_key,
                team,
                title,
                description,
                project,
                priority,
                assignee,
                labels,
            })
            .await?;
        }
        Commands::Digest {
            chat,
            hours,
            limit,
            model,
        } => {
            let config = commands::digest::DigestConfig {
                hours,
                max_messages: limit,
                model,
                ..Default::default()
            };
            let digest = commands::digest::run(&chat, config).await?;
            println!("{}", digest);
        }
        Commands::Moderate { chat, delete, warn } => {
            let config = commands::moderate::ModerateConfig {
                delete_profanity: delete,
                send_warning: warn,
                ..Default::default()
            };
            commands::moderate::run(&chat, config).await?;
        }
        Commands::ProfanityStats { chat, limit } => {
            let stats = commands::moderate::analyze(&chat, limit).await?;
            println!("\n📊 Profanity Statistics for '{}'", chat);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Total messages analyzed: {}", stats.total_messages);
            println!(
                "Messages with profanity: {} ({:.1}%)",
                stats.messages_with_profanity,
                stats.profanity_rate()
            );
            println!("\n🏴‍☠️ Top offenders:");
            for (user, count) in stats.top_offenders(10) {
                println!("  {} - {} violations", user, count);
            }
        }
        Commands::Crm {
            chat,
            limit,
            model,
            export_csv,
        } => {
            let config = commands::crm::CrmConfig {
                model,
                max_messages: limit,
            };
            let extraction = commands::crm::parse_chat(&chat, config).await?;
            commands::crm::print_extraction(&extraction);

            if let Some(csv_path) = export_csv {
                let csv = commands::crm::export_contacts_csv(&extraction);
                std::fs::write(&csv_path, csv)?;
                println!("\n📁 Contacts exported to {}", csv_path);
            }
        }
        Commands::Like {
            chat,
            user,
            emoji,
            limit,
        } => {
            commands::like::run(&chat, &user, Some(&emoji), limit).await?;
        }
        Commands::React {
            chat,
            emoji,
            ids,
            file,
            recent,
            user_id,
            delay_ms,
            dry_run,
        } => {
            commands::react::run(commands::react::ReactArgs {
                chat,
                emoji,
                ids,
                file,
                recent,
                user_id,
                delay_ms,
                dry_run,
            })
            .await?;
        }
        Commands::SendViral => {
            commands::send_viral::run().await?;
        }
        Commands::N8nMonitor => {
            commands::n8n::run_monitor_cli().await?;
        }
        Commands::N8nBackup { action, file } => {
            commands::n8n::run_backup_cli(&action, file.as_deref()).await?;
        }
        Commands::Hunt {
            chats,
            keywords,
            required,
            exclude,
            min_messages,
            days,
            limit,
            export_csv,
            top,
        } => {
            let criteria = commands::hunt::HuntCriteria {
                keywords,
                required_keywords: required,
                exclude_keywords: exclude,
                min_messages,
                days_back: days,
                ..Default::default()
            };

            let chat_refs: Vec<&str> = chats.iter().map(|s| s.as_str()).collect();
            let results = if chat_refs.len() == 1 {
                commands::hunt::hunt_users(chat_refs[0], criteria, limit).await?
            } else {
                commands::hunt::hunt_multiple_chats(&chat_refs, criteria, limit).await?
            };

            commands::hunt::print_results(&results, top);

            if let Some(csv_path) = export_csv {
                let csv = commands::hunt::export_csv(&results);
                std::fs::write(&csv_path, csv)?;
                println!("\n📁 Results exported to {}", csv_path);
            }
        }
    }

    Ok(())
}

// Commands are in src/commands/ directory
