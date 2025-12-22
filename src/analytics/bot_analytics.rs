//! Bot analytics: conversions, funnel, retention.
//!
//! Collects session/message stats from MySQL tables:
//! - bot_users
//! - bot_sessions
//! - bot_messages

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use mysql_async::{prelude::*, Pool, Row};
use regex::Regex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::fs;
use tracing::info;

use crate::{Error, Result};

const GRACE_SECONDS: i64 = 30;

/// Aggregated stats for a single session.
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub id: i64,
    pub user_id: i64,
    pub bot_name: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub messages_in: u32,
    pub messages_out: u32,
    pub non_command_in: u32,
    pub phone_shared: bool,
    pub last_message: Option<DateTime<Utc>>,
}

impl SessionStats {
    /// Create new session stats.
    pub fn new(
        id: i64,
        user_id: i64,
        bot_name: String,
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            bot_name,
            start,
            end,
            messages_in: 0,
            messages_out: 0,
            non_command_in: 0,
            phone_shared: false,
            last_message: None,
        }
    }

    /// Total messages in session.
    pub fn total_messages(&self) -> u32 {
        self.messages_in + self.messages_out
    }

    /// Session has meaningful engagement.
    pub fn engaged(&self) -> bool {
        self.non_command_in >= 1
    }

    /// Session has multi-turn engagement.
    pub fn multi_turn(&self) -> bool {
        self.non_command_in >= 2
    }
}

/// Retention data.
#[derive(Debug, Clone, Serialize)]
pub struct RetentionStats {
    pub base: u32,
    pub returned: u32,
    pub rate: f64,
}

/// Bot metrics aggregate.
#[derive(Debug, Clone, Serialize)]
pub struct BotMetrics {
    pub sessions: u32,
    pub engaged: u32,
    pub multi_turn: u32,
    pub converted: u32,
    pub conversion_rate: f64,
    pub engaged_rate: f64,
    pub multi_rate: f64,
    pub avg_user_messages: f64,
    pub avg_bot_messages: f64,
    pub new_users: u32,
    pub active_users: u32,
    pub retention_d1: RetentionStats,
    pub retention_d7: RetentionStats,
    pub daily: Vec<DailyStats>,
}

/// Daily conversion stats.
#[derive(Debug, Clone, Serialize)]
pub struct DailyStats {
    pub date: NaiveDate,
    pub sessions: u32,
    pub conversions: u32,
}

/// Bot analytics engine.
pub struct BotAnalytics {
    pool: Pool,
    phone_regex: Regex,
}

impl BotAnalytics {
    /// Create new analytics engine.
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            phone_regex: Regex::new(r"\+?\d[\d\-\s\(\)]{8,}\d").expect("Invalid phone regex"),
        }
    }

    /// Get available bot names from database.
    pub async fn get_available_bots(&self) -> Result<Vec<String>> {
        let mut conn = self.pool.get_conn().await?;

        let session_bots: Vec<String> = conn
            .query("SELECT DISTINCT bot_name FROM bot_sessions")
            .await?;

        let message_bots: Vec<String> = conn
            .query("SELECT DISTINCT bot_name FROM bot_messages")
            .await?;

        let mut all_bots: HashSet<String> = session_bots.into_iter().collect();
        all_bots.extend(message_bots);

        if all_bots.is_empty() {
            return Err(Error::InvalidArgument(
                "No bots found in database".to_string(),
            ));
        }

        let mut bots: Vec<String> = all_bots.into_iter().collect();
        bots.sort();
        Ok(bots)
    }

    /// Fetch sessions from database.
    async fn fetch_sessions(
        &self,
        bot_names: &[String],
        start_dt: DateTime<Utc>,
    ) -> Result<Vec<SessionStats>> {
        let mut conn = self.pool.get_conn().await?;

        let placeholders = bot_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            r#"
            SELECT id, user_id, bot_name, state, is_active, session_start, session_end
            FROM bot_sessions
            WHERE bot_name IN ({})
              AND session_start >= ?
            ORDER BY session_start ASC
            "#,
            placeholders
        );

        let start_str = start_dt.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut params: Vec<mysql_async::Value> =
            bot_names.iter().map(|b| b.clone().into()).collect();
        params.push(start_str.into());

        let rows: Vec<Row> = conn.exec(&sql, params).await?;

        let sessions = rows
            .into_iter()
            .filter_map(|row| {
                let id: i64 = row.get("id")?;
                let user_id: i64 = row.get("user_id")?;
                let bot_name: String = row.get("bot_name")?;
                let start_naive: NaiveDateTime = row.get("session_start")?;
                let end_naive: Option<NaiveDateTime> = row.get("session_end");

                let start = Utc.from_utc_datetime(&start_naive);
                let end = end_naive.map(|e| Utc.from_utc_datetime(&e));

                Some(SessionStats::new(id, user_id, bot_name, start, end))
            })
            .collect();

        Ok(sessions)
    }

    /// Fetch messages from database.
    async fn fetch_messages(
        &self,
        bot_names: &[String],
        start_dt: DateTime<Utc>,
    ) -> Result<Vec<MessageRow>> {
        let mut conn = self.pool.get_conn().await?;

        let placeholders = bot_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            r#"
            SELECT user_id, bot_name, direction, message_text, created_at
            FROM bot_messages
            WHERE bot_name IN ({})
              AND created_at >= ?
            ORDER BY created_at ASC
            "#,
            placeholders
        );

        let start_str = start_dt.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut params: Vec<mysql_async::Value> =
            bot_names.iter().map(|b| b.clone().into()).collect();
        params.push(start_str.into());

        let rows: Vec<Row> = conn.exec(&sql, params).await?;

        let messages = rows
            .into_iter()
            .filter_map(|row| {
                let user_id: i64 = row.get("user_id")?;
                let bot_name: String = row.get("bot_name")?;
                let direction: String = row.get("direction")?;
                let message_text: Option<String> = row.get("message_text");
                let created_naive: NaiveDateTime = row.get("created_at")?;
                let created_at = Utc.from_utc_datetime(&created_naive);

                Some(MessageRow {
                    user_id,
                    bot_name,
                    direction,
                    message_text,
                    created_at,
                })
            })
            .collect();

        Ok(messages)
    }

    /// Fetch first message timestamp per user for each bot.
    async fn fetch_first_message_map(
        &self,
        bot_names: &[String],
    ) -> Result<HashMap<String, HashMap<i64, DateTime<Utc>>>> {
        let mut conn = self.pool.get_conn().await?;

        let placeholders = bot_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            r#"
            SELECT bot_name, user_id, MIN(created_at) AS first_message
            FROM bot_messages
            WHERE bot_name IN ({})
            GROUP BY bot_name, user_id
            "#,
            placeholders
        );

        let params: Vec<mysql_async::Value> = bot_names.iter().map(|b| b.clone().into()).collect();

        let rows: Vec<Row> = conn.exec(&sql, params).await?;

        let mut result: HashMap<String, HashMap<i64, DateTime<Utc>>> = HashMap::new();
        for row in rows {
            if let (Some(bot_name), Some(user_id), Some(first_naive)) = (
                row.get::<String, _>("bot_name"),
                row.get::<i64, _>("user_id"),
                row.get::<NaiveDateTime, _>("first_message"),
            ) {
                let first_message = Utc.from_utc_datetime(&first_naive);
                result
                    .entry(bot_name)
                    .or_default()
                    .insert(user_id, first_message);
            }
        }

        Ok(result)
    }

    /// Check if message is meaningful user message.
    fn is_meaningful_message(text: &Option<String>) -> bool {
        match text {
            None => false,
            Some(t) => {
                let cleaned = t.trim();
                !cleaned.is_empty() && !cleaned.starts_with('/')
            }
        }
    }

    /// Check if message contains phone number.
    fn contains_phone(&self, text: &Option<String>) -> bool {
        match text {
            None => false,
            Some(t) => {
                let digits: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
                if (10..=15).contains(&digits.len()) {
                    return true;
                }
                self.phone_regex.is_match(t)
            }
        }
    }

    /// Compute retention metrics.
    fn compute_retention(
        messages_by_user: &HashMap<i64, Vec<&MessageRow>>,
        first_message_map: &HashMap<i64, DateTime<Utc>>,
        window_start: DateTime<Utc>,
    ) -> (RetentionStats, RetentionStats) {
        let today = Utc::now().date_naive();
        let mut d1_base = 0u32;
        let mut d1_return = 0u32;
        let mut d7_base = 0u32;
        let mut d7_return = 0u32;

        for (user_id, first_dt) in first_message_map {
            if *first_dt < window_start {
                continue;
            }

            let message_days: HashSet<NaiveDate> = messages_by_user
                .get(user_id)
                .map(|msgs| msgs.iter().map(|m| m.created_at.date_naive()).collect())
                .unwrap_or_default();

            let first_day = first_dt.date_naive();

            // D1 retention
            let d1_target = first_day + chrono::Duration::days(1);
            if d1_target <= today {
                d1_base += 1;
                if message_days.contains(&d1_target) {
                    d1_return += 1;
                }
            }

            // D7 retention
            let d7_target = first_day + chrono::Duration::days(7);
            if d7_target <= today {
                d7_base += 1;
                if message_days.contains(&d7_target) {
                    d7_return += 1;
                }
            }
        }

        let d1_rate = if d1_base > 0 {
            d1_return as f64 / d1_base as f64 * 100.0
        } else {
            0.0
        };

        let d7_rate = if d7_base > 0 {
            d7_return as f64 / d7_base as f64 * 100.0
        } else {
            0.0
        };

        (
            RetentionStats {
                base: d1_base,
                returned: d1_return,
                rate: d1_rate,
            },
            RetentionStats {
                base: d7_base,
                returned: d7_return,
                rate: d7_rate,
            },
        )
    }

    /// Build metrics for a bot.
    fn build_metrics(
        &self,
        sessions: &[SessionStats],
        messages_by_user: &HashMap<i64, Vec<&MessageRow>>,
        first_message_map: &HashMap<i64, DateTime<Utc>>,
        window_start: DateTime<Utc>,
    ) -> BotMetrics {
        let total_sessions = sessions.len() as u32;
        let engaged_sessions: Vec<_> = sessions.iter().filter(|s| s.engaged()).collect();
        let multi_turn_sessions: Vec<_> = sessions.iter().filter(|s| s.multi_turn()).collect();
        let converted_sessions: Vec<_> = sessions.iter().filter(|s| s.phone_shared).collect();

        // Daily breakdown
        let mut daily_map: HashMap<NaiveDate, (u32, u32)> = HashMap::new();
        for s in sessions {
            let day = s.start.date_naive();
            let entry = daily_map.entry(day).or_insert((0, 0));
            entry.0 += 1;
            if s.phone_shared {
                entry.1 += 1;
            }
        }
        let mut daily: Vec<DailyStats> = daily_map
            .into_iter()
            .map(|(date, (sessions, conversions))| DailyStats {
                date,
                sessions,
                conversions,
            })
            .collect();
        daily.sort_by_key(|d| d.date);

        // Retention
        let (retention_d1, retention_d7) =
            Self::compute_retention(messages_by_user, first_message_map, window_start);

        // New users
        let new_users = first_message_map
            .values()
            .filter(|dt| **dt >= window_start)
            .count() as u32;

        // Average messages
        let avg_user_messages = if !sessions.is_empty() {
            sessions.iter().map(|s| s.messages_in as f64).sum::<f64>() / sessions.len() as f64
        } else {
            0.0
        };

        let avg_bot_messages = if !sessions.is_empty() {
            sessions.iter().map(|s| s.messages_out as f64).sum::<f64>() / sessions.len() as f64
        } else {
            0.0
        };

        let safe_div = |num: u32, denom: u32| -> f64 {
            if denom > 0 {
                num as f64 / denom as f64 * 100.0
            } else {
                0.0
            }
        };

        BotMetrics {
            sessions: total_sessions,
            engaged: engaged_sessions.len() as u32,
            multi_turn: multi_turn_sessions.len() as u32,
            converted: converted_sessions.len() as u32,
            conversion_rate: safe_div(converted_sessions.len() as u32, total_sessions),
            engaged_rate: safe_div(engaged_sessions.len() as u32, total_sessions),
            multi_rate: safe_div(
                multi_turn_sessions.len() as u32,
                engaged_sessions.len() as u32,
            ),
            avg_user_messages,
            avg_bot_messages,
            new_users,
            active_users: messages_by_user.len() as u32,
            retention_d1,
            retention_d7,
            daily,
        }
    }

    /// Run full analytics for specified bots and time window.
    pub async fn analyze(
        &self,
        bot_names: Option<Vec<String>>,
        days: u32,
    ) -> Result<HashMap<String, BotMetrics>> {
        let bots = match bot_names {
            Some(b) => b,
            None => self.get_available_bots().await?,
        };

        let window_start = Utc::now() - Duration::days(days as i64);
        let grace = Duration::seconds(GRACE_SECONDS);

        // Fetch data
        let sessions = self.fetch_sessions(&bots, window_start).await?;
        let messages = self.fetch_messages(&bots, window_start - grace).await?;
        let first_message_map = self.fetch_first_message_map(&bots).await?;

        // Group sessions by bot and user
        let mut session_lookup: HashMap<String, HashMap<i64, Vec<SessionStats>>> = HashMap::new();
        for session in sessions {
            session_lookup
                .entry(session.bot_name.clone())
                .or_default()
                .entry(session.user_id)
                .or_default()
                .push(session);
        }

        // Sort sessions by start time
        for bot_sessions in session_lookup.values_mut() {
            for user_sessions in bot_sessions.values_mut() {
                user_sessions.sort_by_key(|s| s.start);
            }
        }

        // Attach messages to sessions and build user message map
        let mut messages_by_bot_user: HashMap<String, HashMap<i64, Vec<&MessageRow>>> =
            HashMap::new();

        for msg in &messages {
            messages_by_bot_user
                .entry(msg.bot_name.clone())
                .or_default()
                .entry(msg.user_id)
                .or_default()
                .push(msg);

            // Update session stats
            if let Some(bot_sessions) = session_lookup.get_mut(&msg.bot_name) {
                if let Some(user_sessions) = bot_sessions.get_mut(&msg.user_id) {
                    for session in user_sessions.iter_mut() {
                        let start_with_grace = session.start - grace;
                        let session_end = session.end.unwrap_or(DateTime::<Utc>::MAX_UTC);

                        if msg.created_at >= start_with_grace && msg.created_at <= session_end {
                            if msg.direction == "incoming" {
                                session.messages_in += 1;
                                if Self::is_meaningful_message(&msg.message_text) {
                                    session.non_command_in += 1;
                                    if self.contains_phone(&msg.message_text) {
                                        session.phone_shared = true;
                                    }
                                }
                            } else {
                                session.messages_out += 1;
                            }
                            session.last_message = Some(msg.created_at);
                            break;
                        }
                    }
                }
            }
        }

        // Build metrics for each bot
        let mut metrics = HashMap::new();
        for bot in &bots {
            let bot_sessions: Vec<SessionStats> = session_lookup
                .get(bot)
                .map(|user_sessions| {
                    user_sessions
                        .values()
                        .flat_map(|sessions| sessions.clone())
                        .collect()
                })
                .unwrap_or_default();

            let bot_messages = messages_by_bot_user.get(bot).cloned().unwrap_or_default();
            let bot_first_map = first_message_map.get(bot).cloned().unwrap_or_default();

            metrics.insert(
                bot.clone(),
                self.build_metrics(&bot_sessions, &bot_messages, &bot_first_map, window_start),
            );
        }

        Ok(metrics)
    }

    /// Render markdown report.
    pub async fn render_markdown(
        metrics: &HashMap<String, BotMetrics>,
        days: u32,
        output_path: &Path,
    ) -> Result<()> {
        let mut lines = Vec::new();
        let window_start = Utc::now() - Duration::days(days as i64);

        lines.push(format!("# Bot Analytics Dashboard (last {} days)", days));
        lines.push(String::new());
        lines.push(format!(
            "- Period: {} → {} UTC",
            window_start.format("%Y-%m-%d"),
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));
        lines.push("- Definitions:".to_string());
        lines.push(
            "  - Engaged: session has ≥1 осмысленных входящих сообщений (не команды).".to_string(),
        );
        lines.push("  - Multi-turn: ≥2 осмысленных входящих сообщений.".to_string());
        lines.push("  - Conversion: сообщение содержит телефон (10-15 цифр).".to_string());
        lines.push(String::new());

        let mut bot_names: Vec<_> = metrics.keys().collect();
        bot_names.sort();

        for bot_name in bot_names {
            let data = &metrics[bot_name];

            lines.push(format!("## {}", bot_name));
            lines.push(format!(
                "- Sessions: {} | Engaged: {} ({:.1}%) | Multi-turn: {} ({:.1}% от engaged)",
                data.sessions, data.engaged, data.engaged_rate, data.multi_turn, data.multi_rate
            ));
            lines.push(format!(
                "- Conversions (phone shared): {} ({:.1}% of sessions)",
                data.converted, data.conversion_rate
            ));
            lines.push(format!(
                "- Users: new {}, active {}, avg user msgs/session {:.1}, avg bot msgs/session {:.1}",
                data.new_users, data.active_users, data.avg_user_messages, data.avg_bot_messages
            ));
            lines.push(format!(
                "- Retention: D1 {:.1}% ({}/{}), D7 {:.1}% ({}/{})",
                data.retention_d1.rate,
                data.retention_d1.returned,
                data.retention_d1.base,
                data.retention_d7.rate,
                data.retention_d7.returned,
                data.retention_d7.base
            ));
            lines.push(String::new());

            lines.push("**Funnel**".to_string());
            lines.push("| Stage | Sessions | Rate |".to_string());
            lines.push("| --- | --- | --- |".to_string());
            lines.push(format!("| Start | {} | 100% |", data.sessions));
            lines.push(format!(
                "| Engaged | {} | {:.1}% |",
                data.engaged, data.engaged_rate
            ));
            lines.push(format!(
                "| Multi-turn | {} | {:.1}% of engaged |",
                data.multi_turn, data.multi_rate
            ));
            lines.push(format!(
                "| Phone shared | {} | {:.1}% |",
                data.converted, data.conversion_rate
            ));
            lines.push(String::new());

            if !data.daily.is_empty() {
                lines.push("**Daily conversion**".to_string());
                lines.push("| Date | Sessions | Conversions | Rate |".to_string());
                lines.push("| --- | --- | --- | --- |".to_string());

                for d in &data.daily {
                    let rate = if d.sessions > 0 {
                        d.conversions as f64 / d.sessions as f64 * 100.0
                    } else {
                        0.0
                    };
                    lines.push(format!(
                        "| {} | {} | {} | {:.1}% |",
                        d.date, d.sessions, d.conversions, rate
                    ));
                }
                lines.push(String::new());
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(output_path, lines.join("\n")).await?;
        info!("✅ Saved dashboard to {}", output_path.display());

        Ok(())
    }
}

/// Internal message row.
#[derive(Debug)]
struct MessageRow {
    user_id: i64,
    bot_name: String,
    direction: String,
    message_text: Option<String>,
    created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_stats() {
        let mut session = SessionStats::new(1, 12345, "test_bot".to_string(), Utc::now(), None);

        assert!(!session.engaged());
        assert!(!session.multi_turn());

        session.non_command_in = 1;
        assert!(session.engaged());
        assert!(!session.multi_turn());

        session.non_command_in = 2;
        assert!(session.multi_turn());
    }

    #[test]
    fn test_is_meaningful_message() {
        assert!(!BotAnalytics::is_meaningful_message(&None));
        assert!(!BotAnalytics::is_meaningful_message(&Some("".to_string())));
        assert!(!BotAnalytics::is_meaningful_message(&Some(
            "  ".to_string()
        )));
        assert!(!BotAnalytics::is_meaningful_message(&Some(
            "/start".to_string()
        )));
        assert!(BotAnalytics::is_meaningful_message(&Some(
            "Hello".to_string()
        )));
    }

    #[test]
    fn test_session_stats_new() {
        let start = Utc::now();
        let session = SessionStats::new(1, 100, "test_bot".to_string(), start, None);
        
        assert_eq!(session.id, 1);
        assert_eq!(session.user_id, 100);
        assert_eq!(session.bot_name, "test_bot");
        assert_eq!(session.messages_in, 0);
        assert_eq!(session.messages_out, 0);
        assert_eq!(session.non_command_in, 0);
        assert!(!session.phone_shared);
        assert!(session.last_message.is_none());
    }

    #[test]
    fn test_session_stats_engaged() {
        let mut session = SessionStats::new(1, 100, "bot".to_string(), Utc::now(), None);
        
        // Not engaged initially
        assert!(!session.engaged());
        
        // Engaged after non-command message
        session.non_command_in = 1;
        assert!(session.engaged());
    }

    #[test]
    fn test_session_stats_multi_turn() {
        let mut session = SessionStats::new(1, 100, "bot".to_string(), Utc::now(), None);
        
        session.non_command_in = 1;
        assert!(!session.multi_turn());
        
        session.non_command_in = 2;
        assert!(session.multi_turn());
        
        session.non_command_in = 10;
        assert!(session.multi_turn());
    }

    #[test]
    fn test_is_meaningful_message_commands() {
        assert!(!BotAnalytics::is_meaningful_message(&Some("/help".to_string())));
        assert!(!BotAnalytics::is_meaningful_message(&Some("/cancel".to_string())));
        assert!(!BotAnalytics::is_meaningful_message(&Some("/status".to_string())));
    }

    #[test]
    fn test_is_meaningful_message_whitespace() {
        assert!(!BotAnalytics::is_meaningful_message(&Some("\n\t  \n".to_string())));
        assert!(!BotAnalytics::is_meaningful_message(&Some("   ".to_string())));
    }

    #[test]
    fn test_is_meaningful_message_valid() {
        assert!(BotAnalytics::is_meaningful_message(&Some("Hi there".to_string())));
        assert!(BotAnalytics::is_meaningful_message(&Some("123".to_string())));
        assert!(BotAnalytics::is_meaningful_message(&Some("🎉".to_string())));
        assert!(BotAnalytics::is_meaningful_message(&Some("a".to_string())));
    }

    #[test]
    fn test_session_stats_total_messages() {
        let mut session = SessionStats::new(1, 100, "bot".to_string(), Utc::now(), None);
        
        assert_eq!(session.total_messages(), 0);
        
        session.messages_in = 5;
        session.messages_out = 3;
        assert_eq!(session.total_messages(), 8);
    }

    #[test]
    fn test_session_stats_clone() {
        let session = SessionStats::new(1, 100, "test_bot".to_string(), Utc::now(), None);
        let cloned = session.clone();
        
        assert_eq!(session.id, cloned.id);
        assert_eq!(session.user_id, cloned.user_id);
        assert_eq!(session.bot_name, cloned.bot_name);
    }

    #[test]
    fn test_retention_stats_creation() {
        let stats = RetentionStats {
            base: 100,
            returned: 25,
            rate: 25.0,
        };
        
        assert_eq!(stats.base, 100);
        assert_eq!(stats.returned, 25);
        assert!((stats.rate - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_retention_stats_serialize() {
        let stats = RetentionStats {
            base: 50,
            returned: 10,
            rate: 20.0,
        };
        
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"base\":50"));
        assert!(json.contains("\"returned\":10"));
    }

    #[test]
    fn test_retention_stats_clone() {
        let stats = RetentionStats {
            base: 100,
            returned: 20,
            rate: 20.0,
        };
        
        let cloned = stats.clone();
        assert_eq!(stats.base, cloned.base);
        assert_eq!(stats.rate, cloned.rate);
    }

    #[test]
    fn test_daily_stats_creation() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let stats = DailyStats {
            date,
            sessions: 50,
            conversions: 5,
        };
        
        assert_eq!(stats.sessions, 50);
        assert_eq!(stats.conversions, 5);
        assert_eq!(stats.date, chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn test_daily_stats_serialize() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();
        let stats = DailyStats {
            date,
            sessions: 100,
            conversions: 10,
        };
        
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"sessions\":100"));
        assert!(json.contains("\"conversions\":10"));
    }

    #[test]
    fn test_daily_stats_clone() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let stats = DailyStats {
            date,
            sessions: 75,
            conversions: 8,
        };
        
        let cloned = stats.clone();
        assert_eq!(stats.sessions, cloned.sessions);
        assert_eq!(stats.date, cloned.date);
    }

    #[test]
    fn test_bot_metrics_creation() {
        let metrics = BotMetrics {
            sessions: 100,
            engaged: 80,
            multi_turn: 50,
            converted: 10,
            conversion_rate: 10.0,
            engaged_rate: 80.0,
            multi_rate: 62.5,
            avg_user_messages: 3.5,
            avg_bot_messages: 4.2,
            new_users: 20,
            active_users: 75,
            retention_d1: RetentionStats { base: 50, returned: 15, rate: 30.0 },
            retention_d7: RetentionStats { base: 40, returned: 8, rate: 20.0 },
            daily: vec![],
        };
        
        assert_eq!(metrics.sessions, 100);
        assert_eq!(metrics.engaged, 80);
        assert_eq!(metrics.converted, 10);
    }

    #[test]
    fn test_bot_metrics_serialize() {
        let metrics = BotMetrics {
            sessions: 50,
            engaged: 40,
            multi_turn: 25,
            converted: 5,
            conversion_rate: 10.0,
            engaged_rate: 80.0,
            multi_rate: 62.5,
            avg_user_messages: 2.0,
            avg_bot_messages: 3.0,
            new_users: 10,
            active_users: 30,
            retention_d1: RetentionStats { base: 20, returned: 5, rate: 25.0 },
            retention_d7: RetentionStats { base: 15, returned: 2, rate: 13.3 },
            daily: vec![],
        };
        
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"sessions\":50"));
        assert!(json.contains("\"engaged\":40"));
    }

    #[test]
    fn test_bot_metrics_clone() {
        let metrics = BotMetrics {
            sessions: 100,
            engaged: 80,
            multi_turn: 50,
            converted: 10,
            conversion_rate: 10.0,
            engaged_rate: 80.0,
            multi_rate: 62.5,
            avg_user_messages: 3.5,
            avg_bot_messages: 4.2,
            new_users: 20,
            active_users: 75,
            retention_d1: RetentionStats { base: 50, returned: 15, rate: 30.0 },
            retention_d7: RetentionStats { base: 40, returned: 8, rate: 20.0 },
            daily: vec![],
        };
        
        let cloned = metrics.clone();
        assert_eq!(metrics.sessions, cloned.sessions);
        assert_eq!(metrics.conversion_rate, cloned.conversion_rate);
    }

    #[test]
    fn test_session_with_end_time() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);
        let session = SessionStats::new(1, 100, "bot".to_string(), start, Some(end));
        
        assert!(session.end.is_some());
        assert_eq!(session.end.unwrap(), end);
    }

    #[test]
    fn test_session_stats_debug() {
        let session = SessionStats::new(1, 100, "bot".to_string(), Utc::now(), None);
        let debug_str = format!("{:?}", session);
        
        assert!(debug_str.contains("SessionStats"));
        assert!(debug_str.contains("id: 1"));
    }

    #[test]
    fn test_retention_stats_debug() {
        let stats = RetentionStats {
            base: 100,
            returned: 25,
            rate: 25.0,
        };
        
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("RetentionStats"));
    }

    #[test]
    fn test_daily_stats_debug() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let stats = DailyStats {
            date,
            sessions: 10,
            conversions: 1,
        };
        
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("DailyStats"));
    }

    #[test]
    fn test_bot_metrics_debug() {
        let metrics = BotMetrics {
            sessions: 10,
            engaged: 8,
            multi_turn: 5,
            converted: 1,
            conversion_rate: 10.0,
            engaged_rate: 80.0,
            multi_rate: 62.5,
            avg_user_messages: 2.0,
            avg_bot_messages: 3.0,
            new_users: 5,
            active_users: 8,
            retention_d1: RetentionStats { base: 5, returned: 1, rate: 20.0 },
            retention_d7: RetentionStats { base: 3, returned: 0, rate: 0.0 },
            daily: vec![],
        };
        
        let debug_str = format!("{:?}", metrics);
        assert!(debug_str.contains("BotMetrics"));
    }

    #[test]
    fn test_bot_metrics_with_daily_stats() {
        let date1 = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date2 = chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        
        let metrics = BotMetrics {
            sessions: 100,
            engaged: 80,
            multi_turn: 50,
            converted: 10,
            conversion_rate: 10.0,
            engaged_rate: 80.0,
            multi_rate: 62.5,
            avg_user_messages: 3.5,
            avg_bot_messages: 4.2,
            new_users: 20,
            active_users: 75,
            retention_d1: RetentionStats { base: 50, returned: 15, rate: 30.0 },
            retention_d7: RetentionStats { base: 40, returned: 8, rate: 20.0 },
            daily: vec![
                DailyStats { date: date1, sessions: 50, conversions: 5 },
                DailyStats { date: date2, sessions: 50, conversions: 5 },
            ],
        };
        
        assert_eq!(metrics.daily.len(), 2);
        assert_eq!(metrics.daily[0].sessions, 50);
    }

    #[test]
    fn test_grace_seconds_constant() {
        assert_eq!(GRACE_SECONDS, 30);
    }
}

