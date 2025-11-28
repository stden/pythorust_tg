//! Standalone CLI for AI chat analysis (Rust port of chat_analyzer.py)

use clap::Parser;
use std::path::PathBuf;
use telegram_reader::commands::chat_analyzer::{run, AnalyzerConfig, LlmProvider, OutputFormat};

#[derive(Parser)]
#[command(name = "chat_analyzer")]
#[command(about = "Analyze Telegram chat content with AI categorization")]
struct Args {
    /// Chat username/ID/alias to analyze
    chat: String,

    /// LLM provider: openai | claude | gemini
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let cfg = AnalyzerConfig {
        message_limit: args.limit,
        days_back: args.days,
        llm_provider: LlmProvider::from_str(&args.provider),
        model: args.model,
        temperature: args.temperature,
        max_tokens: args.max_tokens,
        min_message_length: args.min_length,
        include_media: args.include_media,
        exclude_bots: !args.include_bots,
        output_format: OutputFormat::from_str(&args.output_format),
        output_dir: args.output_dir,
        prompt_path: args.prompt,
        verbose: !args.quiet,
    };

    let result = run(&args.chat, cfg).await?;
    println!(
        "Chat: {}\nCategory: {}\nSentiment: {}\nMessages analyzed: {}",
        result.chat_name, result.category, result.sentiment, result.activity_metrics.total_messages
    );

    Ok(())
}
