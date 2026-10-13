//! Site monitoring utilities.
//!
//! Features:
//! - Availability checks (HTTP status)
//! - Response time measurement
//! - Content change tracking
//! - Telegram notifications for problems

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs;

/// Site check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub url: String,
    pub status: Option<u16>,
    pub response_time_ms: u64,
    pub content_hash: Option<String>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub is_healthy: bool,
}

/// Monitoring configuration for one site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub url: String,
    pub name: Option<String>,
    pub timeout_secs: Option<u64>,
    pub expected_status: Option<u16>,
    pub check_content: Option<bool>,
    pub content_selector: Option<String>,
}

/// Monitoring configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub sites: Vec<SiteConfig>,
    pub check_interval_secs: Option<u64>,
    pub notify_telegram_chat: Option<String>,
}

/// Check one URL.
pub async fn check_url(url: &str, timeout_secs: u64) -> CheckResult {
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("RustMonitor/1.0")
        .build()
        .unwrap_or_default();

    let start = Instant::now();
    let timestamp = Utc::now();

    match client.get(url).send().await {
        Ok(response) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status = response.status();
            let is_healthy = status.is_success();

            // Get a content hash to track changes.
            let content_hash = match response.text().await {
                Ok(text) => Some(format!("{:x}", md5_hash(&text))),
                Err(_) => None,
            };

            CheckResult {
                url: url.to_string(),
                status: Some(status.as_u16()),
                response_time_ms: elapsed,
                content_hash,
                error: if is_healthy {
                    None
                } else {
                    Some(format!("HTTP {}", status))
                },
                timestamp,
                is_healthy,
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            CheckResult {
                url: url.to_string(),
                status: None,
                response_time_ms: elapsed,
                content_hash: None,
                error: Some(e.to_string()),
                timestamp,
                is_healthy: false,
            }
        }
    }
}

/// Simple MD5 hash for content change tracking.
fn md5_hash(content: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish() as u128
}

/// Check multiple sites in parallel.
pub async fn check_sites(configs: &[SiteConfig]) -> Vec<CheckResult> {
    let futures: Vec<_> = configs
        .iter()
        .map(|config| {
            let url = config.url.clone();
            let timeout = config.timeout_secs.unwrap_or(30);
            async move { check_url(&url, timeout).await }
        })
        .collect();

    futures::future::join_all(futures).await
}

/// Load monitoring config from a YAML file.
pub async fn load_config(path: &Path) -> Result<MonitorConfig> {
    let content = fs::read_to_string(path)
        .await
        .context("Не удалось прочитать файл конфигурации")?;
    let config: MonitorConfig =
        serde_yaml::from_str(&content).context("Не удалось распарсить YAML конфигурацию")?;
    Ok(config)
}

/// Save check history to a JSON file.
pub async fn save_history(results: &[CheckResult], path: &Path) -> Result<()> {
    let content = serde_json::to_string_pretty(results)?;
    fs::write(path, content).await?;
    Ok(())
}

/// Load check history from a JSON file.
pub async fn load_history(path: &Path) -> Result<Vec<CheckResult>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).await?;
    let history: Vec<CheckResult> = serde_json::from_str(&content)?;
    Ok(history)
}

/// Format a check result for output.
pub fn format_result(result: &CheckResult) -> String {
    let status_icon = if result.is_healthy { "✅" } else { "❌" };
    let status_text = result
        .status
        .map(|s| format!("HTTP {}", s))
        .unwrap_or_else(|| "N/A".to_string());

    format!(
        "{} {} | {} | {}ms",
        status_icon, result.url, status_text, result.response_time_ms
    )
}

/// Format results for a Telegram notification.
pub fn format_telegram_alert(results: &[CheckResult]) -> Option<String> {
    let failed: Vec<_> = results.iter().filter(|r| !r.is_healthy).collect();

    if failed.is_empty() {
        return None;
    }

    let mut msg = String::from("🚨 *Проблемы с сайтами:*\n\n");
    for result in failed {
        msg.push_str(&format!(
            "❌ `{}`\n   Ошибка: {}\n   Время: {}ms\n\n",
            result.url,
            result.error.as_deref().unwrap_or("Unknown"),
            result.response_time_ms
        ));
    }

    Some(msg)
}

/// Compare current results with previous results to detect changes.
pub fn detect_changes(
    current: &[CheckResult],
    previous: &HashMap<String, CheckResult>,
) -> Vec<String> {
    let mut changes = Vec::new();

    for result in current {
        if let Some(prev) = previous.get(&result.url) {
            // Status changed.
            if prev.is_healthy != result.is_healthy {
                if result.is_healthy {
                    changes.push(format!("✅ {} восстановлен", result.url));
                } else {
                    changes.push(format!(
                        "❌ {} недоступен: {}",
                        result.url,
                        result.error.as_deref().unwrap_or("Unknown")
                    ));
                }
            }

            // Content changed.
            if let (Some(curr_hash), Some(prev_hash)) = (&result.content_hash, &prev.content_hash) {
                if curr_hash != prev_hash {
                    changes.push(format!("📝 {} контент изменился", result.url));
                }
            }
        } else {
            // New site.
            let icon = if result.is_healthy { "✅" } else { "❌" };
            changes.push(format!("{} {} добавлен в мониторинг", icon, result.url));
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_url_success() {
        use httpmock::prelude::*;
        let server = MockServer::start_async().await;

        let _mock = server.mock(|when, then| {
            when.method(GET).path("/status/200");
            then.status(200);
        });

        let url = server.url("/status/200");
        let result = check_url(&url, 10).await;
        assert!(result.is_healthy);
        assert_eq!(result.status, Some(200));
    }

    #[tokio::test]
    async fn test_check_url_timeout() {
        use httpmock::prelude::*;
        use std::time::Duration;

        let server = MockServer::start_async().await;

        let _mock = server.mock(|when, then| {
            when.method(GET).path("/delay");
            then.status(200).delay(Duration::from_secs(2));
        });

        let url = server.url("/delay");
        let result = check_url(&url, 1).await;
        assert!(!result.is_healthy);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_format_result() {
        let result = CheckResult {
            url: "https://example.com".to_string(),
            status: Some(200),
            response_time_ms: 150,
            content_hash: None,
            error: None,
            timestamp: Utc::now(),
            is_healthy: true,
        };
        let formatted = format_result(&result);
        assert!(formatted.contains("✅"));
        assert!(formatted.contains("200"));
        assert!(formatted.contains("150ms"));
    }
}
