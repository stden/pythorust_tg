//! Export Linear tasks to MySQL database.
//!
//! Usage:
//!   LINEAR_API_KEY=... cargo run --bin export_linear_to_mysql

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use mysql_async::{prelude::*, Pool};
use serde::Deserialize;
use std::env;
use tracing::info;

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

/// Linear issue data.
#[derive(Debug, Deserialize)]
struct Issue {
    id: String,
    identifier: String,
    title: String,
    description: Option<String>,
    priority: Option<i32>,
    #[serde(rename = "priorityLabel")]
    priority_label: Option<String>,
    state: Option<IssueState>,
    team: Option<Team>,
    assignee: Option<Assignee>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
    #[serde(rename = "completedAt")]
    completed_at: Option<String>,
    #[serde(rename = "dueDate")]
    due_date: Option<String>,
    estimate: Option<f64>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IssueState {
    id: String,
    name: String,
    #[serde(rename = "type")]
    state_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Team {
    id: String,
    name: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct Assignee {
    id: String,
    name: String,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IssuesResponse {
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    nodes: Vec<Issue>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    issues: Option<IssuesResponse>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

/// Linear API client.
struct LinearClient {
    http: reqwest::Client,
    api_key: String,
}

impl LinearClient {
    fn new(api_key: String) -> Self {
        let http = reqwest::Client::new();
        Self { http, api_key }
    }

    async fn get_all_issues(&self) -> Result<Vec<Issue>> {
        let mut all_issues = Vec::new();
        let mut after: Option<String> = None;

        loop {
            let query = r#"
                query Issues($first: Int!, $after: String) {
                    issues(first: $first, after: $after) {
                        pageInfo {
                            hasNextPage
                            endCursor
                        }
                        nodes {
                            id
                            identifier
                            title
                            description
                            priority
                            priorityLabel
                            state {
                                id
                                name
                                type
                            }
                            team {
                                id
                                name
                                key
                            }
                            assignee {
                                id
                                name
                                email
                            }
                            createdAt
                            updatedAt
                            completedAt
                            dueDate
                            estimate
                            url
                        }
                    }
                }
            "#;

            let variables = serde_json::json!({
                "first": 250,
                "after": after,
            });

            let response: GraphQLResponse = self
                .http
                .post(LINEAR_API_URL)
                .header("Authorization", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "query": query,
                    "variables": variables,
                }))
                .send()
                .await?
                .json()
                .await?;

            if let Some(errors) = response.errors {
                let msg = errors
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                anyhow::bail!("Linear API error: {}", msg);
            }

            let issues_data = response
                .data
                .and_then(|d| d.issues)
                .context("No issues data in response")?;

            all_issues.extend(issues_data.nodes);
            info!("Fetched {} issues so far", all_issues.len());

            if !issues_data.page_info.has_next_page {
                break;
            }

            after = issues_data.page_info.end_cursor;
        }

        Ok(all_issues)
    }
}

/// Parse ISO datetime string.
fn parse_datetime(s: Option<&str>) -> Option<String> {
    s.map(|dt| {
        // Keep as-is for MySQL, just remove timezone
        dt.replace('T', " ")
            .replace('Z', "")
            .chars()
            .take(19)
            .collect()
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let api_key = env::var("LINEAR_API_KEY").context("LINEAR_API_KEY not set")?;

    // MySQL connection
    let mysql_url = env::var("DATABASE_URL").or_else(|_| {
        let host = env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".to_string());
        let db = env::var("MYSQL_DATABASE").unwrap_or_else(|_| "pythorust_tg".to_string());
        let user = env::var("MYSQL_USER").unwrap_or_else(|_| "pythorust_tg".to_string());
        let password = env::var("MYSQL_PASSWORD")?;
        Ok::<_, env::VarError>(format!(
            "mysql://{}:{}@{}:{}/{}",
            user, password, host, port, db
        ))
    })?;

    let pool = Pool::new(mysql_url.as_str());
    let mut conn = pool.get_conn().await?;

    // Create table if not exists
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS linear_issues (
            id VARCHAR(50) PRIMARY KEY,
            identifier VARCHAR(20) NOT NULL,
            title VARCHAR(500) NOT NULL,
            description TEXT,
            priority INT,
            priority_label VARCHAR(50),
            state_id VARCHAR(50),
            state_name VARCHAR(100),
            state_type VARCHAR(50),
            team_id VARCHAR(50),
            team_name VARCHAR(100),
            team_key VARCHAR(20),
            assignee_id VARCHAR(50),
            assignee_name VARCHAR(100),
            assignee_email VARCHAR(200),
            created_at DATETIME,
            updated_at DATETIME,
            completed_at DATETIME,
            due_date DATE,
            estimate DECIMAL(10,2),
            url VARCHAR(500),
            synced_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            INDEX idx_identifier (identifier),
            INDEX idx_state_name (state_name),
            INDEX idx_team_key (team_key),
            INDEX idx_assignee_id (assignee_id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
        "#,
    )
    .await?;

    info!("Fetching issues from Linear...");

    let client = LinearClient::new(api_key);
    let issues = client.get_all_issues().await?;

    info!("Found {} issues", issues.len());

    // Insert into MySQL
    let insert_query = r#"
        INSERT INTO linear_issues
        (id, identifier, title, description, priority, priority_label,
         state_id, state_name, state_type, team_id, team_name, team_key,
         assignee_id, assignee_name, assignee_email, created_at, updated_at,
         completed_at, due_date, estimate, url)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
        title = VALUES(title),
        description = VALUES(description),
        priority = VALUES(priority),
        priority_label = VALUES(priority_label),
        state_id = VALUES(state_id),
        state_name = VALUES(state_name),
        state_type = VALUES(state_type),
        team_id = VALUES(team_id),
        team_name = VALUES(team_name),
        team_key = VALUES(team_key),
        assignee_id = VALUES(assignee_id),
        assignee_name = VALUES(assignee_name),
        assignee_email = VALUES(assignee_email),
        updated_at = VALUES(updated_at),
        completed_at = VALUES(completed_at),
        due_date = VALUES(due_date),
        estimate = VALUES(estimate),
        url = VALUES(url),
        synced_at = CURRENT_TIMESTAMP
    "#;

    let mut success = 0;
    let mut errors = 0;

    for issue in &issues {
        let state_id = issue.state.as_ref().map(|s| s.id.clone());
        let state_name = issue.state.as_ref().map(|s| s.name.clone());
        let state_type = issue.state.as_ref().and_then(|s| s.state_type.clone());

        let team_id = issue.team.as_ref().map(|t| t.id.clone());
        let team_name = issue.team.as_ref().map(|t| t.name.clone());
        let team_key = issue.team.as_ref().map(|t| t.key.clone());

        let assignee_id = issue.assignee.as_ref().map(|a| a.id.clone());
        let assignee_name = issue.assignee.as_ref().map(|a| a.name.clone());
        let assignee_email = issue.assignee.as_ref().and_then(|a| a.email.clone());

        let created_at = parse_datetime(issue.created_at.as_deref());
        let updated_at = parse_datetime(issue.updated_at.as_deref());
        let completed_at = parse_datetime(issue.completed_at.as_deref());
        let due_date = issue.due_date.clone();

        let params = params! {
            "id" => &issue.id,
            "identifier" => &issue.identifier,
            "title" => &issue.title,
            "description" => &issue.description,
            "priority" => issue.priority,
            "priority_label" => &issue.priority_label,
            "state_id" => &state_id,
            "state_name" => &state_name,
            "state_type" => &state_type,
            "team_id" => &team_id,
            "team_name" => &team_name,
            "team_key" => &team_key,
            "assignee_id" => &assignee_id,
            "assignee_name" => &assignee_name,
            "assignee_email" => &assignee_email,
            "created_at" => &created_at,
            "updated_at" => &updated_at,
            "completed_at" => &completed_at,
            "due_date" => &due_date,
            "estimate" => issue.estimate,
            "url" => &issue.url,
        };

        let named_insert = r#"
            INSERT INTO linear_issues
            (id, identifier, title, description, priority, priority_label,
             state_id, state_name, state_type, team_id, team_name, team_key,
             assignee_id, assignee_name, assignee_email, created_at, updated_at,
             completed_at, due_date, estimate, url)
            VALUES (:id, :identifier, :title, :description, :priority, :priority_label,
                    :state_id, :state_name, :state_type, :team_id, :team_name, :team_key,
                    :assignee_id, :assignee_name, :assignee_email, :created_at, :updated_at,
                    :completed_at, :due_date, :estimate, :url)
            ON DUPLICATE KEY UPDATE
            title = VALUES(title),
            description = VALUES(description),
            priority = VALUES(priority),
            priority_label = VALUES(priority_label),
            state_id = VALUES(state_id),
            state_name = VALUES(state_name),
            state_type = VALUES(state_type),
            team_id = VALUES(team_id),
            team_name = VALUES(team_name),
            team_key = VALUES(team_key),
            assignee_id = VALUES(assignee_id),
            assignee_name = VALUES(assignee_name),
            assignee_email = VALUES(assignee_email),
            updated_at = VALUES(updated_at),
            completed_at = VALUES(completed_at),
            due_date = VALUES(due_date),
            estimate = VALUES(estimate),
            url = VALUES(url),
            synced_at = CURRENT_TIMESTAMP
        "#;

        match conn.exec_drop(named_insert, params).await {
            Ok(_) => {
                success += 1;
            }
            Err(e) => {
                errors += 1;
                eprintln!("ERROR {}: {}", issue.identifier, e);
            }
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("Export complete!");
    println!("  Total:   {}", issues.len());
    println!("  Success: {}", success);
    println!("  Errors:  {}", errors);

    drop(conn);
    pool.disconnect().await?;

    Ok(())
}
