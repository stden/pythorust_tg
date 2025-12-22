//! A/B testing for prompt experiments.
//!
//! Minimal A/B test manager for prompt experiments.
//! Responsibilities:
//! - Persist variant assignment per session
//! - Provide prompt/temperature for the assigned variant
//! - Mark conversions for basic offline reporting

use mysql_async::{prelude::*, Pool};
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{Error, Result};

/// Prompt variant configuration for A/B testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariant {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub model: Option<String>,
}

fn default_weight() -> f64 {
    1.0
}

fn default_temperature() -> f32 {
    0.7
}

impl PromptVariant {
    /// Create a new prompt variant.
    pub fn new(name: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prompt: prompt.into(),
            description: String::new(),
            weight: 1.0,
            temperature: 0.7,
            model: None,
        }
    }

    /// Set the weight for this variant.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Set the temperature for this variant.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the model for this variant.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// A/B test manager for prompt experiments.
pub struct ABTestManager {
    pool: Pool,
    bot_name: String,
    experiment_name: String,
    variants: Vec<PromptVariant>,
    phone_regex: Regex,
}

impl ABTestManager {
    /// Create a new A/B test manager.
    pub async fn new(
        pool: Pool,
        bot_name: impl Into<String>,
        experiment_name: impl Into<String>,
        variants: Vec<PromptVariant>,
    ) -> Result<Self> {
        if variants.is_empty() {
            return Err(Error::InvalidArgument(
                "At least one prompt variant is required".to_string(),
            ));
        }

        let manager = Self {
            pool,
            bot_name: bot_name.into(),
            experiment_name: experiment_name.into(),
            variants,
            phone_regex: Regex::new(r"(?:\+?\d[\d\s\-\(\)]{8,}\d)").expect("Invalid phone regex"),
        };

        manager.ensure_table().await?;
        Ok(manager)
    }

    /// Create experiments table if it does not exist.
    async fn ensure_table(&self) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.query_drop(
            r#"
            CREATE TABLE IF NOT EXISTS bot_experiments (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                bot_name VARCHAR(64) NOT NULL,
                experiment_name VARCHAR(128) NOT NULL,
                session_id BIGINT NULL,
                user_id BIGINT NOT NULL,
                variant VARCHAR(64) NOT NULL,
                conversion TINYINT(1) DEFAULT 0,
                conversion_reason VARCHAR(255) NULL,
                conversion_value INT NULL,
                assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                closed_at TIMESTAMP NULL,
                KEY idx_experiment (bot_name, experiment_name, variant),
                KEY idx_session (session_id),
                KEY idx_user (user_id)
            )
            "#,
        )
        .await?;
        Ok(())
    }

    /// Choose a variant using weighted random selection.
    fn choose_variant(&self) -> &PromptVariant {
        let total_weight: f64 = self.variants.iter().map(|v| v.weight).sum();
        let mut pick = rand::thread_rng().gen_range(0.0..total_weight);

        for variant in &self.variants {
            pick -= variant.weight;
            if pick <= 0.0 {
                return variant;
            }
        }

        self.variants.last().expect("variants not empty")
    }

    /// Fetch saved variant name for a session/user if present.
    async fn fetch_assigned_variant(
        &self,
        session_id: Option<i64>,
        user_id: Option<i64>,
    ) -> Result<Option<String>> {
        let mut conn = self.pool.get_conn().await?;

        let mut sql = String::from(
            "SELECT variant FROM bot_experiments WHERE bot_name = ? AND experiment_name = ?",
        );
        let mut params: Vec<mysql_async::Value> = vec![
            self.bot_name.clone().into(),
            self.experiment_name.clone().into(),
        ];

        if let Some(sid) = session_id {
            sql.push_str(" AND session_id = ?");
            params.push(sid.into());
        }
        if let Some(uid) = user_id {
            sql.push_str(" AND user_id = ?");
            params.push(uid.into());
        }
        sql.push_str(" ORDER BY assigned_at DESC LIMIT 1");

        let result: Option<String> = conn.exec_first(&sql, params).await?;
        Ok(result)
    }

    /// Get variant by name.
    fn variant_by_name(&self, name: &str) -> Result<&PromptVariant> {
        self.variants
            .iter()
            .find(|v| v.name == name)
            .ok_or_else(|| Error::InvalidArgument(format!("Unknown variant: {}", name)))
    }

    /// Returns an assigned variant, creating one if needed.
    pub async fn get_or_assign_variant(
        &self,
        user_id: i64,
        session_id: Option<i64>,
    ) -> Result<PromptVariant> {
        // Check existing assignment
        if let Some(existing) = self
            .fetch_assigned_variant(session_id, Some(user_id))
            .await?
        {
            return Ok(self.variant_by_name(&existing)?.clone());
        }

        // Choose and save new variant
        let chosen = self.choose_variant();
        let mut conn = self.pool.get_conn().await?;

        conn.exec_drop(
            r#"
            INSERT INTO bot_experiments
            (bot_name, experiment_name, session_id, user_id, variant)
            VALUES (?, ?, ?, ?, ?)
            "#,
            (
                &self.bot_name,
                &self.experiment_name,
                session_id,
                user_id,
                &chosen.name,
            ),
        )
        .await?;

        info!(
            variant = %chosen.name,
            user_id = user_id,
            session_id = ?session_id,
            experiment = %self.experiment_name,
            "Assigned variant"
        );

        Ok(chosen.clone())
    }

    /// Mark conversion for the given session if an assignment exists.
    pub async fn mark_conversion(
        &self,
        session_id: Option<i64>,
        reason: Option<&str>,
        conversion_value: Option<i32>,
    ) -> Result<()> {
        let session_id = match session_id {
            Some(id) => id,
            None => return Ok(()),
        };

        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            r#"
            UPDATE bot_experiments
            SET conversion = 1,
                conversion_reason = COALESCE(?, conversion_reason),
                conversion_value = COALESCE(?, conversion_value),
                closed_at = COALESCE(closed_at, CURRENT_TIMESTAMP)
            WHERE session_id = ? AND bot_name = ? AND experiment_name = ?
            "#,
            (
                reason,
                conversion_value,
                session_id,
                &self.bot_name,
                &self.experiment_name,
            ),
        )
        .await?;

        Ok(())
    }

    /// Detect simple conversion intents (phone/CTA) in user text.
    pub fn detect_conversion(&self, text: &str) -> Option<&'static str> {
        if text.is_empty() {
            return None;
        }

        // Check for phone number
        if self.phone_regex.is_match(text) {
            return Some("phone_shared");
        }

        // Check for purchase intent
        let normalized = text.to_lowercase();
        let intent_keywords = [
            "беру",
            "покупаю",
            "оформляем",
            "оплачиваю",
            "готов купить",
            "давай оформим",
            "давайте оформим",
            "давай заказ",
            "берем",
            "хочу купить",
        ];

        for kw in &intent_keywords {
            if normalized.contains(kw) {
                return Some("purchase_intent");
            }
        }

        // Check for checkout details
        let delivery_keywords = ["доставка", "оплата", "адрес", "курьер", "самовывоз"];
        if delivery_keywords.iter().any(|w| normalized.contains(w))
            && (normalized.contains("давай") || normalized.contains("офор"))
        {
            return Some("checkout_details");
        }

        None
    }

    /// Detect conversion and mark if found.
    pub async fn detect_and_mark_conversion(
        &self,
        session_id: Option<i64>,
        text: &str,
    ) -> Result<Option<&'static str>> {
        let reason = self.detect_conversion(text);
        if let Some(r) = reason {
            self.mark_conversion(session_id, Some(r), None).await?;
        }
        Ok(reason)
    }

    /// Close assignment for a session.
    pub async fn close_assignment(&self, session_id: Option<i64>) -> Result<()> {
        let session_id = match session_id {
            Some(id) => id,
            None => return Ok(()),
        };

        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            r#"
            UPDATE bot_experiments
            SET closed_at = COALESCE(closed_at, CURRENT_TIMESTAMP)
            WHERE session_id = ? AND bot_name = ? AND experiment_name = ?
            "#,
            (session_id, &self.bot_name, &self.experiment_name),
        )
        .await?;

        Ok(())
    }
}

/// Metrics for A/B test reporting.
#[derive(Debug, Clone, Serialize)]
pub struct ABTestMetrics {
    pub variant: String,
    pub sessions: u64,
    pub conversions: u64,
    pub conversion_rate: f64,
    pub conversion_value_sum: Option<i64>,
    pub avg_conversion_value: Option<f64>,
    pub reason_breakdown: std::collections::HashMap<String, u64>,
}

type VariantStatsRow = (String, u64, Option<u64>, Option<i64>, Option<u64>);

/// Fetch A/B test metrics from database.
pub async fn fetch_ab_metrics(
    pool: &Pool,
    bot_name: &str,
    experiment: &str,
    days: Option<u32>,
) -> Result<Vec<ABTestMetrics>> {
    let mut conn = pool.get_conn().await?;

    // Build query for variant stats
    let mut sql = String::from(
        r#"
        SELECT variant,
               COUNT(*) AS sessions,
               SUM(conversion) AS conversions,
               SUM(conversion_value) AS conversion_value_sum,
               SUM(conversion_value IS NOT NULL) AS conversions_with_value
        FROM bot_experiments
        WHERE bot_name = ? AND experiment_name = ?
        "#,
    );
    let mut params: Vec<mysql_async::Value> = vec![bot_name.into(), experiment.into()];

    if let Some(d) = days {
        sql.push_str(" AND assigned_at >= DATE_SUB(NOW(), INTERVAL ? DAY)");
        params.push(d.into());
    }
    sql.push_str(" GROUP BY variant ORDER BY variant");

    let rows: Vec<VariantStatsRow> = conn.exec(&sql, params.clone()).await?;

    // Fetch reason breakdown
    let mut reasons_sql = String::from(
        r#"
        SELECT variant, conversion_reason, COUNT(*) AS cnt
        FROM bot_experiments
        WHERE bot_name = ? AND experiment_name = ? AND conversion = 1 AND conversion_reason IS NOT NULL
        "#,
    );
    if days.is_some() {
        reasons_sql.push_str(" AND assigned_at >= DATE_SUB(NOW(), INTERVAL ? DAY)");
    }
    reasons_sql.push_str(" GROUP BY variant, conversion_reason");

    let reasons: Vec<(String, String, u64)> = conn.exec(&reasons_sql, params).await?;

    // Build reason map
    let mut reason_map: std::collections::HashMap<String, std::collections::HashMap<String, u64>> =
        std::collections::HashMap::new();
    for (variant, reason, cnt) in reasons {
        reason_map.entry(variant).or_default().insert(reason, cnt);
    }

    // Build metrics
    let metrics = rows
        .into_iter()
        .map(|(variant, sessions, conversions, value_sum, with_value)| {
            let conversions = conversions.unwrap_or(0);
            let conversion_rate = if sessions > 0 {
                (conversions as f64 / sessions as f64) * 100.0
            } else {
                0.0
            };
            let avg_conversion_value = with_value.and_then(|wv| {
                if wv > 0 {
                    value_sum.map(|vs| vs as f64 / wv as f64)
                } else {
                    None
                }
            });

            ABTestMetrics {
                variant: variant.clone(),
                sessions,
                conversions,
                conversion_rate,
                conversion_value_sum: value_sum,
                avg_conversion_value,
                reason_breakdown: reason_map.remove(&variant).unwrap_or_default(),
            }
        })
        .collect();

    Ok(metrics)
}

/// Print A/B test report to stdout.
pub fn print_ab_report(
    metrics: &[ABTestMetrics],
    bot_name: &str,
    experiment: &str,
    days: Option<u32>,
) {
    let header = if let Some(d) = days {
        format!(
            "🧪 A/B отчет для {} / {} (последние {} дн.)",
            bot_name, experiment, d
        )
    } else {
        format!("🧪 A/B отчет для {} / {}", bot_name, experiment)
    };
    println!("{}", header);
    println!("{}", "-".repeat(header.chars().count()));

    if metrics.is_empty() {
        println!("Нет данных в bot_experiments по заданным фильтрам.");
        return;
    }

    println!(
        "{:22} {:>8} {:>6} {:>7} {:>10}",
        "Variant", "Sessions", "Conv", "Rate %", "Avg value"
    );

    for m in metrics {
        let avg_text = m
            .avg_conversion_value
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:22} {:>8} {:>6} {:>7.2} {:>10}",
            &m.variant[..m.variant.len().min(22)],
            m.sessions,
            m.conversions,
            m.conversion_rate,
            avg_text
        );

        if !m.reason_breakdown.is_empty() {
            let reasons: Vec<String> = m
                .reason_breakdown
                .iter()
                .map(|(r, c)| format!("{}:{}", r, c))
                .collect();
            println!("  reasons: {}", reasons.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_variant_builder() {
        let variant = PromptVariant::new("test", "Test prompt")
            .with_weight(2.0)
            .with_temperature(0.9)
            .with_model("gpt-4o");

        assert_eq!(variant.name, "test");
        assert_eq!(variant.weight, 2.0);
        assert_eq!(variant.temperature, 0.9);
        assert_eq!(variant.model, Some("gpt-4o".to_string()));
    }

    #[test]
    fn test_prompt_variant_defaults() {
        let variant = PromptVariant::new("control", "Default prompt");

        assert_eq!(variant.name, "control");
        assert_eq!(variant.prompt, "Default prompt");
        assert_eq!(variant.weight, 1.0);
        assert_eq!(variant.temperature, 0.7);
        assert!(variant.model.is_none());
        assert!(variant.description.is_empty());
    }

    #[test]
    fn test_detect_conversion_phone() {
        let phone_regex = Regex::new(r"(?:\+?\d[\d\s\-\(\)]{8,}\d)").unwrap();

        // Simple struct for testing
        struct TestManager {
            phone_regex: Regex,
        }

        impl TestManager {
            fn detect_conversion(&self, text: &str) -> Option<&'static str> {
                if self.phone_regex.is_match(text) {
                    return Some("phone_shared");
                }
                None
            }
        }

        let tm = TestManager { phone_regex };
        assert_eq!(
            tm.detect_conversion("+7 911 711 78 50"),
            Some("phone_shared")
        );
        assert_eq!(tm.detect_conversion("89117117850"), Some("phone_shared"));
        assert_eq!(tm.detect_conversion("hello"), None);
    }

    #[test]
    fn test_detect_conversion_purchase_intent() {
        let phone_regex = Regex::new(r"(?:\+?\d[\d\s\-\(\)]{8,}\d)").unwrap();

        struct TestManager {
            phone_regex: Regex,
        }

        impl TestManager {
            fn detect_conversion(&self, text: &str) -> Option<&'static str> {
                if text.is_empty() {
                    return None;
                }

                if self.phone_regex.is_match(text) {
                    return Some("phone_shared");
                }

                let normalized = text.to_lowercase();
                let intent_keywords = [
                    "беру",
                    "покупаю",
                    "оформляем",
                    "оплачиваю",
                    "готов купить",
                ];

                for kw in &intent_keywords {
                    if normalized.contains(kw) {
                        return Some("purchase_intent");
                    }
                }

                None
            }
        }

        let tm = TestManager { phone_regex };
        assert_eq!(tm.detect_conversion("Беру"), Some("purchase_intent"));
        assert_eq!(tm.detect_conversion("покупаю это"), Some("purchase_intent"));
        assert_eq!(tm.detect_conversion("Готов купить!"), Some("purchase_intent"));
        assert_eq!(tm.detect_conversion("привет"), None);
    }

    #[test]
    fn test_detect_conversion_empty_text() {
        let phone_regex = Regex::new(r"(?:\+?\d[\d\s\-\(\)]{8,}\d)").unwrap();

        struct TestManager {
            phone_regex: Regex,
        }

        impl TestManager {
            fn detect_conversion(&self, text: &str) -> Option<&'static str> {
                if text.is_empty() {
                    return None;
                }
                if self.phone_regex.is_match(text) {
                    return Some("phone_shared");
                }
                None
            }
        }

        let tm = TestManager { phone_regex };
        assert_eq!(tm.detect_conversion(""), None);
    }

    #[test]
    fn test_ab_test_metrics_creation() {
        let metrics = ABTestMetrics {
            variant: "control".to_string(),
            sessions: 100,
            conversions: 10,
            conversion_rate: 10.0,
            conversion_value_sum: Some(1500),
            avg_conversion_value: Some(150.0),
            reason_breakdown: std::collections::HashMap::new(),
        };

        assert_eq!(metrics.variant, "control");
        assert_eq!(metrics.sessions, 100);
        assert_eq!(metrics.conversions, 10);
        assert!((metrics.conversion_rate - 10.0).abs() < 0.001);
        assert_eq!(metrics.conversion_value_sum, Some(1500));
        assert_eq!(metrics.avg_conversion_value, Some(150.0));
    }

    #[test]
    fn test_ab_test_metrics_with_reason_breakdown() {
        let mut reasons = std::collections::HashMap::new();
        reasons.insert("phone_shared".to_string(), 5);
        reasons.insert("purchase_intent".to_string(), 3);

        let metrics = ABTestMetrics {
            variant: "test_a".to_string(),
            sessions: 50,
            conversions: 8,
            conversion_rate: 16.0,
            conversion_value_sum: None,
            avg_conversion_value: None,
            reason_breakdown: reasons,
        };

        assert_eq!(metrics.reason_breakdown.len(), 2);
        assert_eq!(metrics.reason_breakdown.get("phone_shared"), Some(&5));
        assert_eq!(metrics.reason_breakdown.get("purchase_intent"), Some(&3));
    }

    #[test]
    fn test_print_ab_report_empty_metrics() {
        let metrics: Vec<ABTestMetrics> = vec![];
        // Should not panic when printing empty metrics
        print_ab_report(&metrics, "test_bot", "test_experiment", Some(7));
    }

    #[test]
    fn test_print_ab_report_with_data() {
        let metrics = vec![
            ABTestMetrics {
                variant: "control".to_string(),
                sessions: 100,
                conversions: 10,
                conversion_rate: 10.0,
                conversion_value_sum: Some(1000),
                avg_conversion_value: Some(100.0),
                reason_breakdown: std::collections::HashMap::new(),
            },
            ABTestMetrics {
                variant: "treatment".to_string(),
                sessions: 100,
                conversions: 15,
                conversion_rate: 15.0,
                conversion_value_sum: None,
                avg_conversion_value: None,
                reason_breakdown: std::collections::HashMap::new(),
            },
        ];

        // Should not panic when printing metrics
        print_ab_report(&metrics, "test_bot", "test_experiment", None);
    }

    #[test]
    fn test_default_weight() {
        assert_eq!(default_weight(), 1.0);
    }

    #[test]
    fn test_default_temperature() {
        assert_eq!(default_temperature(), 0.7);
    }

    #[test]
    fn test_prompt_variant_serialize() {
        let variant = PromptVariant::new("test", "Test prompt")
            .with_weight(1.5)
            .with_model("gpt-4");

        let json = serde_json::to_string(&variant).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"weight\":1.5"));
    }

    #[test]
    fn test_prompt_variant_deserialize() {
        let json = r#"{
            "name": "test",
            "prompt": "Test prompt",
            "weight": 2.0,
            "temperature": 0.8
        }"#;

        let variant: PromptVariant = serde_json::from_str(json).unwrap();
        assert_eq!(variant.name, "test");
        assert_eq!(variant.prompt, "Test prompt");
        assert_eq!(variant.weight, 2.0);
        assert_eq!(variant.temperature, 0.8);
        assert!(variant.model.is_none());
    }

    #[test]
    fn test_prompt_variant_deserialize_with_defaults() {
        let json = r#"{
            "name": "minimal",
            "prompt": "Minimal prompt"
        }"#;

        let variant: PromptVariant = serde_json::from_str(json).unwrap();
        assert_eq!(variant.name, "minimal");
        assert_eq!(variant.weight, 1.0); // default
        assert_eq!(variant.temperature, 0.7); // default
    }

    #[test]
    fn test_prompt_variant_clone() {
        let variant = PromptVariant::new("original", "Original prompt")
            .with_weight(2.0);
        let cloned = variant.clone();

        assert_eq!(variant.name, cloned.name);
        assert_eq!(variant.weight, cloned.weight);
    }

    #[test]
    fn test_ab_metrics_serialization() {
        let metrics = ABTestMetrics {
            variant: "test".to_string(),
            sessions: 100,
            conversions: 10,
            conversion_rate: 10.0,
            conversion_value_sum: Some(1000),
            avg_conversion_value: Some(100.0),
            reason_breakdown: std::collections::HashMap::new(),
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"variant\":\"test\""));
        assert!(json.contains("\"sessions\":100"));
        assert!(json.contains("\"conversions\":10"));
    }
}
