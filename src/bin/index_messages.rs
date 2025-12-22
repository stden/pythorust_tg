//! CLI tool for indexing Telegram messages to vector and graph databases

use anyhow::Result;
use clap::{Parser, Subcommand};
use telegram_reader::commands::{
    index::{index_all_chats, IndexConfig},
    search::{find_contacts, get_stats, search_messages, SearchConfig},
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "index_messages")]
#[command(about = "Index Telegram messages to vector and graph databases")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index messages from all configured chats
    Index {
        /// Message limit per chat
        #[arg(short, long, default_value = "1000")]
        limit: usize,

        /// Skip vector database indexing
        #[arg(long)]
        no_vector: bool,

        /// Skip graph database indexing
        #[arg(long)]
        no_graph: bool,

        /// Skip embedding generation
        #[arg(long)]
        no_embeddings: bool,

        /// Qdrant URL
        #[arg(long, env = "QDRANT_URL", default_value = "http://localhost:6333")]
        qdrant_url: String,
    },

    /// Search indexed messages semantically
    Search {
        /// Search query
        query: String,

        /// Number of results
        #[arg(short, long, default_value = "10")]
        limit: u64,

        /// Filter by chat ID
        #[arg(long)]
        chat_id: Option<i64>,

        /// Filter by sender ID
        #[arg(long)]
        sender_id: Option<i64>,

        /// Only show outgoing messages
        #[arg(long)]
        outgoing: bool,
    },

    /// Find users who interact most with a given user
    Contacts {
        /// User ID to find contacts for
        user_id: i64,

        /// Number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show database statistics
    Stats,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("telegram_reader=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            limit,
            no_vector,
            no_graph,
            no_embeddings,
            qdrant_url,
        } => {
            let config = IndexConfig {
                qdrant_url,
                use_vector_db: !no_vector,
                use_graph_db: !no_graph,
                limit,
                generate_embeddings: !no_embeddings,
            };

            info!("Starting indexing with config:");
            info!("  Limit: {}", limit);
            info!("  Vector DB: {}", config.use_vector_db);
            info!("  Graph DB: {}", config.use_graph_db);
            info!("  Embeddings: {}", config.generate_embeddings);

            let results = index_all_chats(&config).await?;

            println!("\n=== Indexing Results ===");
            for result in &results {
                println!(
                    "{}: {} messages, {} embeddings, {} vector, {} graph",
                    result.chat_name,
                    result.messages_processed,
                    result.embeddings_generated,
                    result.vector_db_indexed,
                    result.graph_db_indexed
                );
            }

            let total_messages: usize = results.iter().map(|r| r.messages_processed).sum();
            let total_embeddings: usize = results.iter().map(|r| r.embeddings_generated).sum();
            println!(
                "\nTotal: {} messages, {} embeddings",
                total_messages, total_embeddings
            );
        }

        Commands::Search {
            query,
            limit,
            chat_id,
            sender_id,
            outgoing,
        } => {
            let config = SearchConfig {
                limit,
                chat_id,
                sender_id,
                outgoing_only: outgoing,
                ..Default::default()
            };

            let results = search_messages(&query, &config).await?;

            println!("\n=== Search Results for '{}' ===\n", query);
            for (i, result) in results.iter().enumerate() {
                println!(
                    "{}. [Score: {:.3}] {} in {}",
                    i + 1,
                    result.score,
                    result.message.sender_name,
                    result.message.chat_name
                );
                println!("   {}", result.message.timestamp.format("%Y-%m-%d %H:%M"));
                println!("   {}", truncate(&result.message.text, 100));
                println!();
            }
        }

        Commands::Contacts { user_id, limit } => {
            let contacts = find_contacts(user_id, limit).await?;

            println!("\n=== Contacts for User {} ===\n", user_id);
            for (i, contact) in contacts.iter().enumerate() {
                println!(
                    "{}. {} (ID: {}) - {} messages",
                    i + 1,
                    contact.name,
                    contact.user_id,
                    contact.message_count
                );
            }
        }

        Commands::Stats => {
            let stats = get_stats().await?;

            println!("\n=== Database Statistics ===\n");
            println!("Vector Database (Qdrant):");
            println!("  Points: {}", stats.vector_points);
            println!("  Dimension: {}", stats.vector_dimension);

            if let (Some(users), Some(chats), Some(messages), Some(relations)) = (
                stats.graph_users,
                stats.graph_chats,
                stats.graph_messages,
                stats.graph_relations,
            ) {
                println!("\nGraph Database (Neo4j):");
                println!("  Users: {}", users);
                println!("  Chats: {}", chats);
                println!("  Messages: {}", messages);
                println!("  Relations: {}", relations);
            } else {
                println!("\nGraph Database (Neo4j): Not connected");
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
