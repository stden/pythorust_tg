//! N8N Service Monitor with Auto-Restart
//!
//! Мониторинг N8N с автоматическим перезапуском при недоступности

use chrono::{DateTime, Utc};
use reqwest::Client;
use std::env;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::{Error, Result};

/// Monitor configuration.
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub n8n_url: String,
    pub api_key: Option<String>,
    pub check_interval_secs: u64,
    pub restart_command: String,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<i64>,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

impl MonitorConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let n8n_url = env::var("N8N_URL")
            .map_err(|_| Error::InvalidArgument("N8N_URL not set".to_string()))?;

        let restart_command =
            env::var("N8N_RESTART_COMMAND").unwrap_or_else(|_| "systemctl restart n8n".to_string());

        Ok(Self {
            n8n_url,
            api_key: env::var("N8N_API_KEY").ok(),
            check_interval_secs: env::var("CHECK_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            restart_command,
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: env::var("TELEGRAM_CHAT_ID")
                .ok()
                .and_then(|s| s.parse().ok()),
            max_retries: env::var("MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            timeout_secs: env::var("TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        })
    }
}

/// N8N service monitor.
pub struct N8NMonitor {
    config: MonitorConfig,
    http: Client,
    consecutive_failures: u32,
    last_restart: Option<DateTime<Utc>>,
}

impl N8NMonitor {
    /// Create new monitor.
    pub fn new(config: MonitorConfig) -> Result<Self> {
        let http = Client::builder()
            .user_agent("n8n_monitor/0.1.0")
            .timeout(Duration::from_secs(config.timeout_secs))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| Error::InvalidArgument(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            config,
            http,
            consecutive_failures: 0,
            last_restart: None,
        })
    }

    /// Create from environment.
    pub fn from_env() -> Result<Self> {
        let config = MonitorConfig::from_env()?;
        Self::new(config)
    }

    /// Send alert to Telegram.
    async fn send_telegram_alert(&self, message: &str) {
        let (token, chat_id) = match (
            &self.config.telegram_bot_token,
            self.config.telegram_chat_id,
        ) {
            (Some(t), Some(c)) => (t, c),
            _ => return,
        };

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let text = format!(
            "🚨 N8N Monitor Alert\n\n{}\n\nTime: {}",
            message,
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        );

        let params = [("chat_id", chat_id.to_string()), ("text", text)];

        match self.http.post(&url).form(&params).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!(message = message, "Telegram alert sent");
            }
            Ok(resp) => {
                error!(status = %resp.status(), "Failed to send Telegram alert");
            }
            Err(e) => {
                error!(error = %e, "Failed to send Telegram alert");
            }
        }
    }

    /// Check if N8N is responding.
    pub async fn check_health(&self) -> bool {
        let url = format!("{}/healthz", self.config.n8n_url);

        let mut request = self.http.get(&url);
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-N8N-API-KEY", api_key);
        }

        match request.send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("✅ N8N is healthy");
                true
            }
            Ok(resp) => {
                warn!(status = %resp.status(), "❌ N8N returned error status");
                false
            }
            Err(e) => {
                error!(error = %e, "❌ N8N connection error");
                false
            }
        }
    }

    /// Restart N8N service.
    async fn restart(&mut self) -> bool {
        // Check minimum time between restarts (5 minutes)
        if let Some(last) = self.last_restart {
            let since_restart = (Utc::now() - last).num_seconds();
            if since_restart < 300 {
                warn!(
                    seconds_since_restart = since_restart,
                    "Skipping restart, too soon since last restart"
                );
                return false;
            }
        }

        info!("🔄 Attempting to restart N8N...");
        self.send_telegram_alert(&format!(
            "Restarting N8N after {} failed checks",
            self.consecutive_failures
        ))
        .await;

        let output = Command::new("sh")
            .arg("-c")
            .arg(&self.config.restart_command)
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                info!("✅ N8N restart command executed successfully");
                self.last_restart = Some(Utc::now());
                self.send_telegram_alert("✅ N8N restarted successfully")
                    .await;

                // Wait 10 seconds before checking
                sleep(Duration::from_secs(10)).await;

                // Verify service is up
                let is_healthy = self.check_health().await;
                if is_healthy {
                    self.consecutive_failures = 0;
                    true
                } else {
                    error!("❌ N8N still unhealthy after restart");
                    self.send_telegram_alert("⚠️ N8N restarted but still unhealthy")
                        .await;
                    false
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                error!(stderr = %stderr, "❌ Failed to restart N8N");
                self.send_telegram_alert(&format!("❌ Failed to restart N8N: {}", stderr))
                    .await;
                false
            }
            Err(e) => {
                error!(error = %e, "❌ Exception during restart");
                self.send_telegram_alert(&format!("❌ Exception during restart: {}", e))
                    .await;
                false
            }
        }
    }

    /// Main monitoring loop.
    pub async fn monitor_loop(&mut self) -> Result<()> {
        info!(
            url = %self.config.n8n_url,
            interval = self.config.check_interval_secs,
            command = %self.config.restart_command,
            "🚀 Starting N8N monitor"
        );

        self.send_telegram_alert("🚀 N8N Monitor started").await;

        loop {
            let is_healthy = self.check_health().await;

            if is_healthy {
                if self.consecutive_failures > 0 {
                    info!(failures = self.consecutive_failures, "✅ N8N recovered");
                    self.send_telegram_alert(&format!(
                        "✅ N8N recovered after {} failures",
                        self.consecutive_failures
                    ))
                    .await;
                }
                self.consecutive_failures = 0;
            } else {
                self.consecutive_failures += 1;
                warn!(
                    failures = self.consecutive_failures,
                    max = self.config.max_retries,
                    "⚠️ Consecutive failures"
                );

                if self.consecutive_failures >= self.config.max_retries {
                    error!(
                        retries = self.config.max_retries,
                        "❌ N8N failed health checks, initiating restart"
                    );
                    self.restart().await;
                }
            }

            sleep(Duration::from_secs(self.config.check_interval_secs)).await;
        }
    }

    /// Run single health check (for CLI).
    pub async fn run_check(&self) -> bool {
        self.check_health().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use std::sync::{LazyLock, Mutex};

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
    fn monitor_config_from_env_requires_n8n_url() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _unset = EnvGuard::unset("N8N_URL");
        let err = MonitorConfig::from_env().unwrap_err();
        assert!(err.to_string().contains("N8N_URL not set"));
    }

    #[test]
    fn monitor_config_from_env_applies_defaults() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guards = [
            EnvGuard::set("N8N_URL", "http://localhost:5678"),
            EnvGuard::unset("N8N_RESTART_COMMAND"),
            EnvGuard::unset("CHECK_INTERVAL"),
            EnvGuard::unset("MAX_RETRIES"),
            EnvGuard::unset("TIMEOUT"),
        ];

        let cfg = MonitorConfig::from_env().unwrap();
        assert_eq!(cfg.n8n_url, "http://localhost:5678");
        assert_eq!(cfg.restart_command, "systemctl restart n8n");
        assert_eq!(cfg.check_interval_secs, 60);
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.timeout_secs, 10);
    }

    fn monitor_for(server: &MockServer, api_key: Option<String>) -> N8NMonitor {
        let cfg = MonitorConfig {
            n8n_url: server.base_url(),
            api_key,
            check_interval_secs: 60,
            restart_command: "true".to_string(),
            telegram_bot_token: None,
            telegram_chat_id: None,
            max_retries: 3,
            timeout_secs: 2,
        };
        N8NMonitor::new(cfg).expect("monitor")
    }

    #[tokio::test]
    async fn check_health_returns_true_on_success() {
        let server = MockServer::start_async().await;

        let health_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/healthz")
                .header("X-N8N-API-KEY", "k");
            then.status(200);
        });

        let monitor = monitor_for(&server, Some("k".to_string()));
        assert!(monitor.check_health().await);
        health_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn check_health_returns_false_on_error_status() {
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path("/healthz");
            then.status(503);
        });

        let monitor = monitor_for(&server, None);
        assert!(!monitor.check_health().await);
    }

    #[tokio::test]
    async fn send_telegram_alert_is_noop_without_credentials() {
        let server = MockServer::start_async().await;
        let monitor = monitor_for(&server, None);
        monitor.send_telegram_alert("test").await;
    }

    #[tokio::test]
    async fn restart_is_skipped_when_too_soon_since_last_restart() {
        let server = MockServer::start_async().await;
        let mut monitor = monitor_for(&server, None);
        monitor.last_restart = Some(Utc::now());
        monitor.consecutive_failures = 10;

        let restarted = monitor.restart().await;
        assert!(!restarted);
    }
}
