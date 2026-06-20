//! Simple K8s Dashboard in Rust.
//!
//! Basic features:
//! - List pods
//! - Pod logs
//! - List namespaces
//! - Node status

use std::io::{stdout, Write};

use clap::{Parser, Subcommand};
use k8s_openapi::api::core::v1::{Namespace, Node, Pod};
use kube::{
    api::{Api, ListParams, LogParams},
    Client, Config,
};

#[derive(Parser)]
#[command(name = "k8s-dash")]
#[command(about = "Простой K8s Dashboard на Rust")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List pods in a namespace.
    Pods {
        /// Namespace (all by default).
        #[arg(short, long, default_value = "default")]
        namespace: String,
        /// Show all namespaces.
        #[arg(short, long)]
        all: bool,
    },
    /// Pod logs.
    Logs {
        /// Pod name.
        name: String,
        /// Namespace
        #[arg(short, long, default_value = "default")]
        namespace: String,
        /// Container (optional).
        #[arg(short, long)]
        container: Option<String>,
        /// Follow logs.
        #[arg(short, long)]
        follow: bool,
        /// Number of lines.
        #[arg(long, default_value = "100")]
        tail: i64,
    },
    /// List namespaces.
    Namespaces,
    /// Node status.
    Nodes,
    /// Pod information.
    Describe {
        /// Pod name.
        name: String,
        /// Namespace
        #[arg(short, long, default_value = "default")]
        namespace: String,
    },
    /// List all resources.
    All {
        /// Namespace
        #[arg(short, long, default_value = "default")]
        namespace: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load kubeconfig.
    let config = Config::infer().await?;
    let client = Client::try_from(config)?;

    match cli.command {
        Commands::Pods { namespace, all } => {
            list_pods(&client, if all { None } else { Some(&namespace) }).await?;
        }
        Commands::Logs {
            name,
            namespace,
            container,
            follow: _,
            tail,
        } => {
            get_logs(&client, &namespace, &name, container.as_deref(), tail).await?;
        }
        Commands::Namespaces => {
            list_namespaces(&client).await?;
        }
        Commands::Nodes => {
            list_nodes(&client).await?;
        }
        Commands::Describe { name, namespace } => {
            describe_pod(&client, &namespace, &name).await?;
        }
        Commands::All { namespace } => {
            println!("=== Pods in {} ===", namespace);
            list_pods(&client, Some(&namespace)).await?;
        }
    }

    Ok(())
}

async fn list_pods(client: &Client, namespace: Option<&str>) -> anyhow::Result<()> {
    let pods: Api<Pod> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let lp = ListParams::default();
    let pod_list = pods.list(&lp).await?;

    println!(
        "{:<40} {:<15} {:<10} {:<10} {:<20}",
        "NAME", "NAMESPACE", "STATUS", "RESTARTS", "AGE"
    );
    println!("{}", "-".repeat(95));

    for pod in pod_list {
        let name = pod.metadata.name.unwrap_or_default();
        let ns = pod.metadata.namespace.unwrap_or_default();
        let status = pod
            .status
            .as_ref()
            .and_then(|s| s.phase.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let restarts: i32 = pod
            .status
            .as_ref()
            .and_then(|s| s.container_statuses.as_ref())
            .map(|cs| cs.iter().map(|c| c.restart_count).sum())
            .unwrap_or(0);

        let age = pod
            .metadata
            .creation_timestamp
            .map(|t| format_age(t.0))
            .unwrap_or_else(|| "?".to_string());

        println!(
            "{:<40} {:<15} {:<10} {:<10} {:<20}",
            truncate(&name, 40),
            truncate(&ns, 15),
            status,
            restarts,
            age
        );
    }

    Ok(())
}

async fn get_logs(
    client: &Client,
    namespace: &str,
    name: &str,
    container: Option<&str>,
    tail: i64,
) -> anyhow::Result<()> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

    let mut lp = LogParams {
        tail_lines: Some(tail),
        ..Default::default()
    };
    if let Some(c) = container {
        lp.container = Some(c.to_string());
    }

    let logs = pods.logs(name, &lp).await?;
    print!("{}", logs);
    stdout().flush()?;

    Ok(())
}

async fn list_namespaces(client: &Client) -> anyhow::Result<()> {
    let namespaces: Api<Namespace> = Api::all(client.clone());
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await?;

    println!("{:<30} {:<10} {:<20}", "NAME", "STATUS", "AGE");
    println!("{}", "-".repeat(60));

    for ns in ns_list {
        let name = ns.metadata.name.unwrap_or_default();
        let status = ns
            .status
            .as_ref()
            .and_then(|s| s.phase.clone())
            .unwrap_or_else(|| "Active".to_string());
        let age = ns
            .metadata
            .creation_timestamp
            .map(|t| format_age(t.0))
            .unwrap_or_else(|| "?".to_string());

        println!("{:<30} {:<10} {:<20}", name, status, age);
    }

    Ok(())
}

async fn list_nodes(client: &Client) -> anyhow::Result<()> {
    let nodes: Api<Node> = Api::all(client.clone());
    let lp = ListParams::default();
    let node_list = nodes.list(&lp).await?;

    println!(
        "{:<40} {:<10} {:<15} {:<20}",
        "NAME", "STATUS", "VERSION", "AGE"
    );
    println!("{}", "-".repeat(85));

    for node in node_list {
        let name = node.metadata.name.unwrap_or_default();

        let status = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .and_then(|c| c.iter().find(|cond| cond.type_ == "Ready"))
            .map(|c| {
                if c.status == "True" {
                    "Ready"
                } else {
                    "NotReady"
                }
            })
            .unwrap_or("Unknown");

        let version = node
            .status
            .as_ref()
            .and_then(|s| s.node_info.as_ref())
            .map(|i| i.kubelet_version.clone())
            .unwrap_or_default();

        let age = node
            .metadata
            .creation_timestamp
            .map(|t| format_age(t.0))
            .unwrap_or_else(|| "?".to_string());

        println!("{:<40} {:<10} {:<15} {:<20}", name, status, version, age);
    }

    Ok(())
}

async fn describe_pod(client: &Client, namespace: &str, name: &str) -> anyhow::Result<()> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let pod = pods.get(name).await?;

    println!("Name:         {}", pod.metadata.name.unwrap_or_default());
    println!(
        "Namespace:    {}",
        pod.metadata.namespace.unwrap_or_default()
    );

    if let Some(labels) = &pod.metadata.labels {
        println!("Labels:");
        for (k, v) in labels {
            println!("              {}={}", k, v);
        }
    }

    if let Some(status) = &pod.status {
        println!("Status:       {}", status.phase.clone().unwrap_or_default());
        println!(
            "IP:           {}",
            status.pod_ip.clone().unwrap_or_default()
        );
        println!(
            "Node:         {}",
            status.nominated_node_name.clone().unwrap_or_default()
        );

        if let Some(containers) = &status.container_statuses {
            println!("\nContainers:");
            for c in containers {
                println!("  {}:", c.name);
                println!("    Image:    {}", c.image);
                println!("    Ready:    {}", c.ready);
                println!("    Restarts: {}", c.restart_count);
            }
        }
    }

    Ok(())
}

fn format_age(time: k8s_openapi::jiff::Timestamp) -> String {
    let now = k8s_openapi::jiff::Timestamp::now();
    let duration = now.duration_since(time);

    let secs = duration.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;

    if days > 0 {
        format!("{}d", days)
    } else if hours > 0 {
        format!("{}h", hours)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", secs)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}
