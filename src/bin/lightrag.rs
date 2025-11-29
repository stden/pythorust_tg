//! Rust port of `analyze_with_lightrag.py`.
//!
//! Loads Telegram messages from MySQL, builds a LightRAG index, and optionally
//! answers semantic queries using the in-crate retriever.

use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use telegram_reader::commands::lightrag::{
    build_retriever, load_messages, mode_from_str, DEFAULT_BATCH_SIZE, DEFAULT_LIMIT,
};
use telegram_reader::lightrag::RetrievalResult;

#[derive(Parser)]
#[command(name = "lightrag")]
#[command(about = "Graph RAG over Telegram messages stored in MySQL")]
struct Cli {
    /// Index messages from MySQL (always required today because index is in-memory)
    #[arg(long)]
    index: bool,

    /// Query to run after indexing
    #[arg(long)]
    query: Option<String>,

    /// Max messages to load from MySQL
    #[arg(long, default_value_t = DEFAULT_LIMIT)]
    limit: usize,

    /// How many LightRAG results to display
    #[arg(long, default_value_t = 5)]
    results: usize,

    /// Retrieval mode: hybrid | vector | graph | naive | local | global
    #[arg(long, default_value = "hybrid")]
    mode: String,

    /// Documents per embedding batch (smaller = more API calls, larger = heavier requests)
    #[arg(long, default_value_t = DEFAULT_BATCH_SIZE)]
    batch_size: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("telegram_reader=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    if !cli.index && cli.query.is_none() {
        println!(
            "Nothing to do: pass --index to build the RAG index and/or --query <text> to search"
        );
        return Ok(());
    }

    let retrieval_mode = mode_from_str(&cli.mode);

    info!(
        "Loading up to {} messages from MySQL (batch size: {})",
        cli.limit, cli.batch_size
    );
    let messages = load_messages(cli.limit).await?;
    if messages.is_empty() {
        warn!("No messages loaded from MySQL. Check filters or DB contents.");
        return Ok(());
    }

    let rag = build_retriever(&messages, cli.batch_size).await?;

    if cli.index && cli.query.is_none() {
        println!(
            "Indexed {} messages into {} chunks (mode: {:?})",
            messages.len(),
            rag.len(),
            retrieval_mode
        );
        return Ok(());
    }

    if let Some(query) = cli.query {
        info!(
            "Running query '{}' with mode {:?} (top {})",
            query, retrieval_mode, cli.results
        );
        let results = rag.retrieve(&query, cli.results, retrieval_mode).await?;
        print_results(&query, &results);
    }

    Ok(())
}

fn print_results(query: &str, results: &[RetrievalResult]) {
    println!("\n=== LightRAG results for '{}' ===\n", query);

    if results.is_empty() {
        println!("No results found.");
        return;
    }

    for (idx, res) in results.iter().enumerate() {
        println!(
            "{}. score: {:.3} | source: {}",
            idx + 1,
            res.score,
            res.chunk.source
        );

        println!("   {}", truncate(&res.chunk.text.replace('\n', " "), 240));

        if !res.matched_entities.is_empty() {
            println!("   matched entities: {}", res.matched_entities.join(", "));
        }

        if !res.related_entities.is_empty() {
            println!("   related entities: {}", res.related_entities.join(", "));
        }

        println!();
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
