//! N8N Monitor and Backup commands
//!
//! Rust implementation of n8n_monitor.py and n8n_backup.py
//!
//! Features:
//! - Health check with auto-restart
//! - Telegram alerts
//! - Workflow backup to JSON
//! - Backup rotation by age and count

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tokio::fs;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// N8N Monitor configuration from environment
#[derive(Debug, Clone)]
pub struct N8nMonitorConfig {
    pub n8n_url: String,
    pub api_key: Option<String>,
    pub check_interval_secs: u64,
    pub restart_command: String,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
}

impl Default for N8nMonitorConfig {
    fn default() -> Self {
        Self {
            n8n_url: std::env::var("N8N_URL").unwrap_or_else(|_| "http://localhost:5678".into()),
            api_key: std::env::var("N8N_API_KEY").ok(),
            check_interval_secs: std::env::var("CHECK_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            restart_command: std::env::var("N8N_RESTART_COMMAND")
                .unwrap_or_else(|_| "systemctl restart n8n".into()),
            max_retries: std::env::var("MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            timeout_secs: std::env::var("TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").ok(),
        }
    }
}

/// N8N Backup configuration
#[derive(Debug, Clone)]
pub struct N8nBackupConfig {
    pub n8n_url: String,
    pub api_key: Option<String>,
    pub backup_dir: PathBuf,
    pub retention_days: u32,
    pub max_backups: u32,
}

impl Default for N8nBackupConfig {
    fn default() -> Self {
        Self {
            n8n_url: std::env::var("N8N_URL").unwrap_or_else(|_| "http://localhost:5678".into()),
            api_key: std::env::var("N8N_API_KEY").ok(),
            backup_dir: std::env::var("BACKUP_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/srv/backups/n8n")),
            retention_days: std::env::var("RETENTION_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_backups: std::env::var("MAX_BACKUPS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        }
    }
}

/// Health check result
#[derive(Debug)]
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub status_code: Option<u16>,
    pub response_time_ms: u64,
    pub error: Option<String>,
}

/// N8N Workflow from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub active: bool,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// N8N API response wrapper
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

/// Backup info metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub timestamp: String,
    pub datetime: String,
    pub n8n_url: String,
    pub workflows_count: usize,
    pub credentials_count: usize,
}

// ============================================================================
// N8N Monitor Functions
// ============================================================================

/// Check N8N health endpoint
pub async fn check_health(config: &N8nMonitorConfig) -> HealthCheckResult {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default();

    let url = format!("{}/healthz", config.n8n_url);
    let start = Instant::now();

    let mut request = client.get(&url);
    if let Some(ref api_key) = config.api_key {
        request = request.header("X-N8N-API-KEY", api_key);
    }

    match request.send().await {
        Ok(response) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status = response.status();

            if status.is_success() {
                info!("N8N is healthy ({}ms)", elapsed);
                HealthCheckResult {
                    is_healthy: true,
                    status_code: Some(status.as_u16()),
                    response_time_ms: elapsed,
                    error: None,
                }
            } else {
                warn!("N8N returned status {}", status);
                HealthCheckResult {
                    is_healthy: false,
                    status_code: Some(status.as_u16()),
                    response_time_ms: elapsed,
                    error: Some(format!("HTTP {}", status)),
                }
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            error!("N8N health check failed: {}", e);
            HealthCheckResult {
                is_healthy: false,
                status_code: None,
                response_time_ms: elapsed,
                error: Some(e.to_string()),
            }
        }
    }
}

/// Send Telegram alert via Bot API
pub async fn send_telegram_alert(
    bot_token: &str,
    chat_id: &str,
    message: &str,
) -> Result<()> {
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let full_message = format!("N8N Monitor Alert\n\n{}\n\nTime: {}", message, timestamp);

    let params = [("chat_id", chat_id), ("text", &full_message)];

    client
        .post(&url)
        .form(&params)
        .send()
        .await
        .context("Failed to send Telegram alert")?;

    info!("Telegram alert sent: {}", message);
    Ok(())
}

/// Restart N8N service
pub async fn restart_n8n(config: &N8nMonitorConfig) -> Result<bool> {
    info!("Attempting to restart N8N...");

    // Send alert before restart
    if let (Some(ref token), Some(ref chat_id)) =
        (&config.telegram_bot_token, &config.telegram_chat_id)
    {
        let _ = send_telegram_alert(token, chat_id, "Restarting N8N...").await;
    }

    // Execute restart command
    let output = Command::new("sh")
        .arg("-c")
        .arg(&config.restart_command)
        .output()
        .context("Failed to execute restart command")?;

    if output.status.success() {
        info!("N8N restart command executed successfully");

        // Wait for service to start
        sleep(std::time::Duration::from_secs(10)).await;

        // Check if healthy
        let health = check_health(config).await;
        if health.is_healthy {
            if let (Some(ref token), Some(ref chat_id)) =
                (&config.telegram_bot_token, &config.telegram_chat_id)
            {
                let _ = send_telegram_alert(token, chat_id, "N8N restarted successfully").await;
            }
            Ok(true)
        } else {
            error!("N8N still unhealthy after restart");
            if let (Some(ref token), Some(ref chat_id)) =
                (&config.telegram_bot_token, &config.telegram_chat_id)
            {
                let _ = send_telegram_alert(
                    token,
                    chat_id,
                    "N8N restarted but still unhealthy",
                )
                .await;
            }
            Ok(false)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to restart N8N: {}", stderr);
        if let (Some(ref token), Some(ref chat_id)) =
            (&config.telegram_bot_token, &config.telegram_chat_id)
        {
            let _ = send_telegram_alert(
                token,
                chat_id,
                &format!("Failed to restart N8N: {}", stderr),
            )
            .await;
        }
        Ok(false)
    }
}

/// Run the N8N monitor loop
pub async fn run_monitor(config: N8nMonitorConfig) -> Result<()> {
    info!("Starting N8N monitor for {}", config.n8n_url);
    info!("Check interval: {}s", config.check_interval_secs);
    info!("Restart command: {}", config.restart_command);

    // Send startup alert
    if let (Some(ref token), Some(ref chat_id)) =
        (&config.telegram_bot_token, &config.telegram_chat_id)
    {
        let _ = send_telegram_alert(token, chat_id, "N8N Monitor started").await;
    }

    let mut consecutive_failures: u32 = 0;
    let mut last_restart: Option<DateTime<Utc>> = None;

    loop {
        let result = check_health(&config).await;

        if result.is_healthy {
            if consecutive_failures > 0 {
                info!("N8N recovered after {} failures", consecutive_failures);
                if let (Some(ref token), Some(ref chat_id)) =
                    (&config.telegram_bot_token, &config.telegram_chat_id)
                {
                    let _ = send_telegram_alert(
                        token,
                        chat_id,
                        &format!("N8N recovered after {} failures", consecutive_failures),
                    )
                    .await;
                }
            }
            consecutive_failures = 0;
        } else {
            consecutive_failures += 1;
            warn!(
                "Consecutive failures: {}/{}",
                consecutive_failures, config.max_retries
            );

            if consecutive_failures >= config.max_retries {
                // Check if enough time has passed since last restart
                let can_restart = match last_restart {
                    None => true,
                    Some(last) => {
                        let elapsed = Utc::now() - last;
                        elapsed > Duration::seconds(300) // 5 min cooldown
                    }
                };

                if can_restart {
                    error!(
                        "N8N failed {} health checks, initiating restart",
                        config.max_retries
                    );
                    if restart_n8n(&config).await? {
                        consecutive_failures = 0;
                    }
                    last_restart = Some(Utc::now());
                } else {
                    warn!("Skipping restart - cooldown period not elapsed");
                }
            }
        }

        sleep(std::time::Duration::from_secs(config.check_interval_secs)).await;
    }
}

// ============================================================================
// N8N Backup Functions
// ============================================================================

/// Get all workflows from N8N API
pub async fn get_workflows(config: &N8nBackupConfig) -> Result<Vec<Workflow>> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/api/v1/workflows", config.n8n_url);
    let mut request = client.get(&url);

    if let Some(ref api_key) = config.api_key {
        request = request.header("X-N8N-API-KEY", api_key);
    }

    let response = request.send().await.context("Failed to connect to N8N API")?;

    if response.status().is_success() {
        let api_response: ApiResponse<Vec<Workflow>> = response.json().await?;
        info!("Retrieved {} workflows", api_response.data.len());
        Ok(api_response.data)
    } else {
        error!("Failed to get workflows: HTTP {}", response.status());
        Ok(Vec::new())
    }
}

/// Get credentials metadata from N8N API
pub async fn get_credentials(config: &N8nBackupConfig) -> Result<Vec<serde_json::Value>> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/api/v1/credentials", config.n8n_url);
    let mut request = client.get(&url);

    if let Some(ref api_key) = config.api_key {
        request = request.header("X-N8N-API-KEY", api_key);
    }

    match request.send().await {
        Ok(response) if response.status().is_success() => {
            let api_response: ApiResponse<Vec<serde_json::Value>> = response.json().await?;
            info!(
                "Retrieved {} credentials (metadata only)",
                api_response.data.len()
            );
            Ok(api_response.data)
        }
        Ok(response) => {
            warn!("Could not get credentials: HTTP {}", response.status());
            Ok(Vec::new())
        }
        Err(e) => {
            warn!("Could not get credentials: {}", e);
            Ok(Vec::new())
        }
    }
}

/// Create a backup of N8N configuration
pub async fn create_backup(config: &N8nBackupConfig) -> Result<PathBuf> {
    // Ensure backup directory exists
    fs::create_dir_all(&config.backup_dir).await?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_name = format!("n8n_backup_{}", timestamp);
    let backup_dir = config.backup_dir.join(&backup_name);
    fs::create_dir_all(&backup_dir).await?;

    info!("Creating backup: {}", backup_name);

    // 1. Backup workflows
    let workflows = get_workflows(config).await?;
    if !workflows.is_empty() {
        let workflows_file = backup_dir.join("workflows.json");
        let workflows_json = serde_json::to_string_pretty(&workflows)?;
        fs::write(&workflows_file, workflows_json).await?;
        info!("Saved {} workflows", workflows.len());
    }

    // 2. Backup credentials metadata
    let credentials = get_credentials(config).await?;
    if !credentials.is_empty() {
        let credentials_file = backup_dir.join("credentials_meta.json");
        let credentials_json = serde_json::to_string_pretty(&credentials)?;
        fs::write(&credentials_file, credentials_json).await?;
        info!("Saved {} credentials metadata", credentials.len());
    }

    // 3. Create backup info
    let backup_info = BackupInfo {
        timestamp: timestamp.clone(),
        datetime: Utc::now().to_rfc3339(),
        n8n_url: config.n8n_url.clone(),
        workflows_count: workflows.len(),
        credentials_count: credentials.len(),
    };
    let info_file = backup_dir.join("backup_info.json");
    let info_json = serde_json::to_string_pretty(&backup_info)?;
    fs::write(&info_file, info_json).await?;

    // 4. Create tar.gz archive
    let archive_path = config.backup_dir.join(format!("{}.tar.gz", backup_name));

    // Use tar command for simplicity
    let output = Command::new("tar")
        .args([
            "-czf",
            archive_path.to_str().unwrap(),
            "-C",
            config.backup_dir.to_str().unwrap(),
            &backup_name,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to create archive: {}", stderr);
        anyhow::bail!("Failed to create tar archive");
    }

    info!("Created archive: {:?}", archive_path);

    // 5. Cleanup temporary directory
    fs::remove_dir_all(&backup_dir).await?;

    Ok(archive_path)
}

/// List all available backups
pub async fn list_backups(config: &N8nBackupConfig) -> Result<Vec<(PathBuf, u64, i64)>> {
    let mut backups = Vec::new();

    if !config.backup_dir.exists() {
        info!("No backups found");
        return Ok(backups);
    }

    let mut entries = fs::read_dir(&config.backup_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with("n8n_backup_"))
            .unwrap_or(false)
            && path.extension().map(|e| e == "gz").unwrap_or(false)
        {
            let metadata = entry.metadata().await?;
            let size = metadata.len();
            let modified = metadata.modified()?;
            let modified_dt: DateTime<Utc> = modified.into();
            let age_days = (Utc::now() - modified_dt).num_days();
            backups.push((path, size, age_days));
        }
    }

    // Sort by modification time (newest first)
    backups.sort_by(|a, b| b.2.cmp(&a.2).reverse());

    if backups.is_empty() {
        info!("No backups found");
    } else {
        info!("Available backups ({}):", backups.len());
        for (path, size, age) in &backups {
            let size_mb = *size as f64 / (1024.0 * 1024.0);
            info!(
                "  {} ({:.2} MB, {} days old)",
                path.file_name().unwrap().to_string_lossy(),
                size_mb,
                age
            );
        }
    }

    Ok(backups)
}

/// Cleanup old backups based on retention policy
pub async fn cleanup_backups(config: &N8nBackupConfig) -> Result<(u32, u32)> {
    let mut backups = list_backups(config).await?;
    let mut removed_by_age = 0u32;
    let mut removed_by_count = 0u32;

    // Remove by age
    let retention_days = config.retention_days as i64;
    backups.retain(|(path, _, age)| {
        if *age > retention_days {
            if let Err(e) = std::fs::remove_file(path) {
                error!("Failed to remove {:?}: {}", path, e);
            } else {
                info!(
                    "Removed old backup: {} (age: {} days)",
                    path.file_name().unwrap().to_string_lossy(),
                    age
                );
                removed_by_age += 1;
            }
            false
        } else {
            true
        }
    });

    // Remove by count (oldest first)
    while backups.len() > config.max_backups as usize {
        if let Some((path, _, _)) = backups.pop() {
            if let Err(e) = std::fs::remove_file(&path) {
                error!("Failed to remove {:?}: {}", path, e);
            } else {
                info!(
                    "Removed excess backup: {}",
                    path.file_name().unwrap().to_string_lossy()
                );
                removed_by_count += 1;
            }
        }
    }

    if removed_by_age > 0 || removed_by_count > 0 {
        info!(
            "Cleanup: removed {} by age, {} by count",
            removed_by_age, removed_by_count
        );
    } else {
        info!("No backups to remove");
    }

    Ok((removed_by_age, removed_by_count))
}

/// Restore N8N configuration from backup
pub async fn restore_backup(config: &N8nBackupConfig, backup_file: &Path) -> Result<bool> {
    if !backup_file.exists() {
        error!("Backup file not found: {:?}", backup_file);
        return Ok(false);
    }

    info!("Restoring from: {:?}", backup_file);

    // Extract archive to temp directory
    let temp_dir = config.backup_dir.join("restore_temp");
    fs::create_dir_all(&temp_dir).await?;

    let output = Command::new("tar")
        .args([
            "-xzf",
            backup_file.to_str().unwrap(),
            "-C",
            temp_dir.to_str().unwrap(),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to extract archive: {}", stderr);
        fs::remove_dir_all(&temp_dir).await?;
        return Ok(false);
    }

    // Find extracted directory
    let mut entries = fs::read_dir(&temp_dir).await?;
    let mut backup_data_dir = None;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("n8n_backup_") {
            backup_data_dir = Some(entry.path());
            break;
        }
    }

    let backup_data_dir = match backup_data_dir {
        Some(dir) => dir,
        None => {
            error!("No backup data found in archive");
            fs::remove_dir_all(&temp_dir).await?;
            return Ok(false);
        }
    };

    // Read workflows
    let workflows_file = backup_data_dir.join("workflows.json");
    if workflows_file.exists() {
        let content = fs::read_to_string(&workflows_file).await?;
        let workflows: Vec<serde_json::Value> = serde_json::from_str(&content)?;
        info!("Found {} workflows in backup", workflows.len());
        warn!("Workflow restoration via API not yet implemented");
        info!("You can manually import workflows from: {:?}", workflows_file);
    }

    info!("Backup extracted successfully");

    // Cleanup temp directory
    fs::remove_dir_all(&temp_dir).await?;

    Ok(true)
}

// ============================================================================
// CLI Entry Points
// ============================================================================

/// Run N8N monitor (CLI entry point)
pub async fn run_monitor_cli() -> Result<()> {
    let config = N8nMonitorConfig::default();
    run_monitor(config).await
}

/// Run N8N backup command
pub async fn run_backup_cli(action: &str, file: Option<&Path>) -> Result<()> {
    let config = N8nBackupConfig::default();

    match action {
        "backup" => {
            let archive = create_backup(&config).await?;
            println!("Backup created: {:?}", archive);
        }
        "list" => {
            list_backups(&config).await?;
        }
        "cleanup" => {
            cleanup_backups(&config).await?;
        }
        "restore" => {
            let file = file.context("Please specify backup file with --file")?;
            restore_backup(&config, file).await?;
        }
        _ => {
            anyhow::bail!("Unknown action: {}. Use: backup, list, cleanup, restore", action);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_monitor_config() {
        let config = N8nMonitorConfig::default();
        assert!(config.n8n_url.contains("localhost") || config.n8n_url.contains("5678"));
        assert!(config.max_retries >= 1);
    }

    #[test]
    fn test_default_backup_config() {
        let config = N8nBackupConfig::default();
        assert!(config.retention_days >= 1);
        assert!(config.max_backups >= 1);
    }
}
