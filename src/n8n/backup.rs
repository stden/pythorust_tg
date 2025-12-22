//! N8N Configuration Backup
//!
//! Автоматический бэкап конфигураций N8N

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

use crate::{Error, Result};

/// Backup configuration.
#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub n8n_url: String,
    pub api_key: Option<String>,
    pub backup_dir: PathBuf,
    pub retention_days: u32,
    pub max_backups: u32,
}

impl BackupConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let n8n_url = env::var("N8N_URL")
            .map_err(|_| Error::InvalidArgument("N8N_URL not set".to_string()))?;

        let backup_dir = env::var("BACKUP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/srv/backups/n8n"));

        Ok(Self {
            n8n_url,
            api_key: env::var("N8N_API_KEY").ok(),
            backup_dir,
            retention_days: env::var("RETENTION_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_backups: env::var("MAX_BACKUPS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        })
    }
}

/// Workflow data from N8N API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub active: bool,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Credentials metadata from N8N API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialMeta {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub cred_type: String,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// API response wrapper.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: Vec<T>,
}

/// Backup info metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub timestamp: String,
    pub datetime: String,
    pub n8n_url: String,
    pub workflows_count: usize,
    pub credentials_count: usize,
}

/// N8N backup manager.
pub struct N8NBackup {
    config: BackupConfig,
    http: Client,
}

impl N8NBackup {
    /// Create new backup manager.
    pub fn new(config: BackupConfig) -> Result<Self> {
        let http = Client::builder()
            .user_agent("n8n_backup/0.1.0")
            .danger_accept_invalid_certs(true) // For self-signed certs
            .build()
            .map_err(|e| Error::InvalidArgument(format!("HTTP client error: {}", e)))?;

        Ok(Self { config, http })
    }

    /// Create from environment.
    pub fn from_env() -> Result<Self> {
        let config = BackupConfig::from_env()?;
        Self::new(config)
    }

    /// Get headers for API requests.
    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref api_key) = self.config.api_key {
            headers.insert("X-N8N-API-KEY", api_key.parse().expect("Invalid API key"));
        }
        headers
    }

    /// Get all workflows from N8N API.
    pub async fn get_workflows(&self) -> Result<Vec<Workflow>> {
        let url = format!("{}/api/v1/workflows", self.config.n8n_url);

        let response = self
            .http
            .get(&url)
            .headers(self.get_headers())
            .send()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to get workflows: {}", e)))?;

        if !response.status().is_success() {
            warn!(status = %response.status(), "Failed to get workflows");
            return Ok(vec![]);
        }

        let api_response: ApiResponse<Workflow> = response
            .json()
            .await
            .map_err(|e| Error::InvalidArgument(format!("Failed to parse workflows: {}", e)))?;

        info!(count = api_response.data.len(), "✅ Retrieved workflows");
        Ok(api_response.data)
    }

    /// Get all credentials from N8N API (metadata only).
    pub async fn get_credentials(&self) -> Result<Vec<CredentialMeta>> {
        let url = format!("{}/api/v1/credentials", self.config.n8n_url);

        let response = self.http.get(&url).headers(self.get_headers()).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let api_response: ApiResponse<CredentialMeta> = resp.json().await.map_err(|e| {
                    Error::InvalidArgument(format!("Failed to parse credentials: {}", e))
                })?;

                info!(
                    count = api_response.data.len(),
                    "✅ Retrieved credentials metadata"
                );
                Ok(api_response.data)
            }
            Ok(resp) => {
                warn!(status = %resp.status(), "⚠️ Could not get credentials");
                Ok(vec![])
            }
            Err(e) => {
                warn!(error = %e, "⚠️ Could not get credentials");
                Ok(vec![])
            }
        }
    }

    /// Create a backup of N8N configuration.
    pub async fn create_backup(&self) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_name = format!("n8n_backup_{}", timestamp);
        let backup_path = self.config.backup_dir.join(&backup_name);

        // Create backup directory
        fs::create_dir_all(&backup_path).await?;

        info!(name = %backup_name, "🔄 Creating backup");

        // 1. Backup workflows
        let workflows = self.get_workflows().await?;
        if !workflows.is_empty() {
            let workflows_file = backup_path.join("workflows.json");
            let content = serde_json::to_string_pretty(&workflows)?;
            fs::write(&workflows_file, content).await?;
            info!(count = workflows.len(), "✅ Saved workflows");
        }

        // 2. Backup credentials metadata
        let credentials = self.get_credentials().await?;
        if !credentials.is_empty() {
            let creds_file = backup_path.join("credentials_meta.json");
            let content = serde_json::to_string_pretty(&credentials)?;
            fs::write(&creds_file, content).await?;
            info!(count = credentials.len(), "✅ Saved credentials metadata");
        }

        // 3. Create backup info file
        let backup_info = BackupInfo {
            timestamp: timestamp.clone(),
            datetime: Utc::now().to_rfc3339(),
            n8n_url: self.config.n8n_url.clone(),
            workflows_count: workflows.len(),
            credentials_count: credentials.len(),
        };
        let info_file = backup_path.join("backup_info.json");
        let content = serde_json::to_string_pretty(&backup_info)?;
        fs::write(&info_file, content).await?;

        // 4. Create tar.gz archive
        let archive_path = self
            .config
            .backup_dir
            .join(format!("{}.tar.gz", backup_name));
        self.create_tar_gz(&backup_path, &archive_path).await?;

        info!(path = %archive_path.display(), "✅ Created archive");

        // 5. Cleanup temporary directory
        fs::remove_dir_all(&backup_path).await?;

        Ok(archive_path)
    }

    /// Create tar.gz archive.
    async fn create_tar_gz(&self, source: &Path, dest: &Path) -> Result<()> {
        use std::process::Command;

        let status = Command::new("tar")
            .arg("-czf")
            .arg(dest)
            .arg("-C")
            .arg(source.parent().unwrap_or(Path::new(".")))
            .arg(source.file_name().unwrap())
            .status()
            .map_err(|e| Error::InvalidArgument(format!("Failed to create archive: {}", e)))?;

        if !status.success() {
            return Err(Error::InvalidArgument(
                "Failed to create tar archive".to_string(),
            ));
        }

        Ok(())
    }

    /// Remove old backups based on retention policy.
    pub async fn cleanup_old_backups(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.config.backup_dir).await?;
        let mut backups: Vec<(PathBuf, DateTime<Utc>)> = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "gz")
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("n8n_backup_"))
            {
                let metadata = fs::metadata(&path).await?;
                let modified: DateTime<Utc> = metadata.modified()?.into();
                backups.push((path, modified));
            }
        }

        backups.sort_by_key(|(_, mtime)| *mtime);

        let now = Utc::now();
        let mut removed_by_age = 0;
        let mut removed_by_count = 0;

        // Remove by age
        for (path, mtime) in &backups {
            let age_days = (now - *mtime).num_days();
            if age_days > self.config.retention_days as i64 {
                fs::remove_file(path).await?;
                info!(
                    file = %path.display(),
                    age_days = age_days,
                    "🗑️ Removed old backup"
                );
                removed_by_age += 1;
            }
        }

        // Refresh list after age removal
        backups.retain(|(path, _)| path.exists());

        // Remove by count
        while backups.len() > self.config.max_backups as usize {
            if let Some((oldest, _)) = backups.first() {
                fs::remove_file(oldest).await?;
                info!(file = %oldest.display(), "🗑️ Removed excess backup");
                removed_by_count += 1;
                backups.remove(0);
            }
        }

        if removed_by_age > 0 || removed_by_count > 0 {
            info!(
                by_age = removed_by_age,
                by_count = removed_by_count,
                "✅ Cleanup completed"
            );
        } else {
            info!("✅ No backups to remove");
        }

        Ok(())
    }

    /// List all available backups.
    pub async fn list_backups(&self) -> Result<Vec<BackupEntry>> {
        let mut entries = fs::read_dir(&self.config.backup_dir).await?;
        let mut backups: Vec<BackupEntry> = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "gz")
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("n8n_backup_"))
            {
                let metadata = fs::metadata(&path).await?;
                let modified: DateTime<Utc> = metadata.modified()?.into();
                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                let age_days = (Utc::now() - modified).num_days();

                backups.push(BackupEntry {
                    path,
                    modified,
                    size_mb,
                    age_days,
                });
            }
        }

        backups.sort_by_key(|b| std::cmp::Reverse(b.modified));
        Ok(backups)
    }

    /// Restore N8N configuration from backup.
    pub async fn restore_backup(&self, backup_file: &Path) -> Result<()> {
        if !backup_file.exists() {
            return Err(Error::InvalidArgument(format!(
                "Backup file not found: {}",
                backup_file.display()
            )));
        }

        info!(file = %backup_file.display(), "🔄 Restoring from backup");

        let temp_dir = self.config.backup_dir.join("restore_temp");
        fs::create_dir_all(&temp_dir).await?;

        // Extract archive
        let status = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(backup_file)
            .arg("-C")
            .arg(&temp_dir)
            .status()
            .map_err(|e| Error::InvalidArgument(format!("Failed to extract archive: {}", e)))?;

        if !status.success() {
            fs::remove_dir_all(&temp_dir).await.ok();
            return Err(Error::InvalidArgument(
                "Failed to extract tar archive".to_string(),
            ));
        }

        // Find extracted directory
        let mut entries = fs::read_dir(&temp_dir).await?;
        let mut backup_data_dir = None;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("n8n_backup_"))
            {
                backup_data_dir = Some(path);
                break;
            }
        }

        let backup_data_dir = backup_data_dir
            .ok_or_else(|| Error::InvalidArgument("No backup data found in archive".to_string()))?;

        // Check workflows file
        let workflows_file = backup_data_dir.join("workflows.json");
        if workflows_file.exists() {
            let content = fs::read_to_string(&workflows_file).await?;
            let workflows: Vec<Workflow> = serde_json::from_str(&content)?;
            info!(count = workflows.len(), "🔄 Found workflows in backup");
            warn!("⚠️ Workflow restoration via API not yet implemented");
            info!(file = %workflows_file.display(), "ℹ️ You can manually import workflows from this file");
        }

        info!("✅ Backup extracted successfully");

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();

        Ok(())
    }
}

/// Backup entry for listing.
#[derive(Debug)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub modified: DateTime<Utc>,
    pub size_mb: f64,
    pub age_days: i64,
}

impl std::fmt::Display for BackupEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "• {} ({:.2} MB, {} days old)",
            self.path.file_name().unwrap_or_default().to_string_lossy(),
            self.size_mb,
            self.age_days
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;
    use std::sync::{LazyLock, Mutex};
    use tempfile::tempdir;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct EnvGuard {
        key: String,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                key: key.to_string(),
                original,
            }
        }

        fn unset(key: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::remove_var(key);
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(v) => std::env::set_var(&self.key, v),
                None => std::env::remove_var(&self.key),
            }
        }
    }

    #[test]
    fn backup_config_from_env_requires_n8n_url() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _unset = EnvGuard::unset("N8N_URL");
        let err = BackupConfig::from_env().unwrap_err();
        assert!(err.to_string().contains("N8N_URL not set"));
    }

    #[test]
    fn backup_config_from_env_parses_values_and_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        let tmp = tempdir().expect("tempdir");
        let tmp_path = tmp.path().to_string_lossy().to_string();

        let _guards = [
            EnvGuard::set("N8N_URL", "http://localhost:5678"),
            EnvGuard::set("BACKUP_DIR", &tmp_path),
            EnvGuard::set("N8N_API_KEY", "k"),
            EnvGuard::set("RETENTION_DAYS", "7"),
            EnvGuard::set("MAX_BACKUPS", "3"),
        ];

        let cfg = BackupConfig::from_env().unwrap();
        assert_eq!(cfg.n8n_url, "http://localhost:5678");
        assert_eq!(cfg.api_key.as_deref(), Some("k"));
        assert_eq!(cfg.backup_dir, PathBuf::from(tmp_path));
        assert_eq!(cfg.retention_days, 7);
        assert_eq!(cfg.max_backups, 3);
    }

    fn backup_for(server: &MockServer, backup_dir: &Path) -> N8NBackup {
        let cfg = BackupConfig {
            n8n_url: server.base_url(),
            api_key: Some("k".to_string()),
            backup_dir: backup_dir.to_path_buf(),
            retention_days: 365,
            max_backups: 10,
        };
        N8NBackup::new(cfg).expect("backup")
    }

    #[test]
    fn get_headers_includes_api_key_when_present() {
        let tmp = tempdir().expect("tempdir");
        let server = MockServer::start();
        let backup = backup_for(&server, tmp.path());

        let headers = backup.get_headers();
        assert_eq!(headers.get("X-N8N-API-KEY").unwrap(), "k");
    }

    #[test]
    fn get_headers_is_empty_without_api_key() {
        let tmp = tempdir().expect("tempdir");
        let cfg = BackupConfig {
            n8n_url: "http://localhost".to_string(),
            api_key: None,
            backup_dir: tmp.path().to_path_buf(),
            retention_days: 30,
            max_backups: 10,
        };
        let backup = N8NBackup::new(cfg).expect("backup");

        let headers = backup.get_headers();
        assert!(headers.get("X-N8N-API-KEY").is_none());
    }

    #[tokio::test]
    async fn get_workflows_parses_data_and_extra_fields() {
        let server = MockServer::start_async().await;

        let workflows_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/api/v1/workflows")
                .header("X-N8N-API-KEY", "k");
            then.status(200).json_body(json!({
                "data": [
                    {
                        "id": "1",
                        "name": "Main",
                        "active": true,
                        "extra_field": "x"
                    }
                ]
            }));
        });

        let tmp = tempdir().expect("tempdir");
        let backup = backup_for(&server, tmp.path());

        let workflows = backup.get_workflows().await.unwrap();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].id, "1");
        assert_eq!(workflows[0].name, "Main");
        assert!(workflows[0].active);
        assert_eq!(
            workflows[0]
                .extra
                .get("extra_field")
                .and_then(|v| v.as_str()),
            Some("x")
        );

        workflows_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn get_workflows_returns_empty_vec_on_http_error() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/workflows");
            then.status(500);
        });

        let tmp = tempdir().expect("tempdir");
        let backup = backup_for(&server, tmp.path());

        let workflows = backup.get_workflows().await.unwrap();
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn get_credentials_returns_empty_vec_on_error_status() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/credentials");
            then.status(403);
        });

        let tmp = tempdir().expect("tempdir");
        let backup = backup_for(&server, tmp.path());

        let creds = backup.get_credentials().await.unwrap();
        assert!(creds.is_empty());
    }

    #[tokio::test]
    async fn create_backup_writes_archive_and_cleans_up_temp_dir() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(GET)
                .path("/api/v1/workflows")
                .header("X-N8N-API-KEY", "k");
            then.status(200).json_body(json!({
                "data": [
                    { "id": "1", "name": "WF", "active": true }
                ]
            }));
        });

        server.mock(|when, then| {
            when.method(GET)
                .path("/api/v1/credentials")
                .header("X-N8N-API-KEY", "k");
            then.status(200).json_body(json!({
                "data": [
                    { "id": "c1", "name": "Cred", "type": "some" }
                ]
            }));
        });

        let tmp = tempdir().expect("tempdir");
        let backup = backup_for(&server, tmp.path());

        let archive_path = backup.create_backup().await.unwrap();
        assert!(archive_path.exists());

        let filename = archive_path
            .file_name()
            .expect("filename")
            .to_string_lossy();
        assert!(filename.starts_with("n8n_backup_"));
        assert!(filename.ends_with(".tar.gz"));

        let backup_name = filename.strip_suffix(".tar.gz").expect("suffix");
        assert!(!tmp.path().join(backup_name).exists());

        let output = std::process::Command::new("tar")
            .arg("-tzf")
            .arg(&archive_path)
            .output()
            .expect("tar list");
        assert!(output.status.success());
        let listing = String::from_utf8_lossy(&output.stdout);
        assert!(listing.contains("workflows.json"));
        assert!(listing.contains("credentials_meta.json"));
        assert!(listing.contains("backup_info.json"));
    }

    #[tokio::test]
    async fn restore_backup_extracts_and_cleans_up_restore_dir() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(GET).path("/api/v1/workflows");
            then.status(200).json_body(json!({
                "data": [
                    { "id": "1", "name": "WF", "active": true }
                ]
            }));
        });

        server.mock(|when, then| {
            when.method(GET).path("/api/v1/credentials");
            then.status(200).json_body(json!({ "data": [] }));
        });

        let tmp = tempdir().expect("tempdir");
        let cfg = BackupConfig {
            n8n_url: server.base_url(),
            api_key: None,
            backup_dir: tmp.path().to_path_buf(),
            retention_days: 365,
            max_backups: 10,
        };
        let backup = N8NBackup::new(cfg).expect("backup");

        let archive_path = backup.create_backup().await.unwrap();
        backup.restore_backup(&archive_path).await.unwrap();

        assert!(!tmp.path().join("restore_temp").exists());
    }

    #[tokio::test]
    async fn cleanup_old_backups_respects_max_backups_and_ignores_unrelated_files() {
        let server = MockServer::start_async().await;
        let tmp = tempdir().expect("tempdir");

        let cfg = BackupConfig {
            n8n_url: server.base_url(),
            api_key: None,
            backup_dir: tmp.path().to_path_buf(),
            retention_days: 365,
            max_backups: 2,
        };
        let backup = N8NBackup::new(cfg).expect("backup");

        for name in [
            "n8n_backup_20250101_000000.tar.gz",
            "n8n_backup_20250102_000000.tar.gz",
            "n8n_backup_20250103_000000.tar.gz",
        ] {
            tokio::fs::write(tmp.path().join(name), b"x")
                .await
                .expect("write dummy");
        }
        tokio::fs::write(tmp.path().join("unrelated.gz"), b"y")
            .await
            .expect("write unrelated");

        backup.cleanup_old_backups().await.unwrap();

        let mut kept = 0;
        let mut entries = tokio::fs::read_dir(tmp.path()).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("n8n_backup_"))
                && path.extension().is_some_and(|e| e == "gz")
            {
                kept += 1;
            }
        }

        assert_eq!(kept, 2);
        assert!(tmp.path().join("unrelated.gz").exists());
    }
}
