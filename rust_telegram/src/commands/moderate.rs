//! Chat moderation - profanity filter and content moderation
//!
//! Monitors chat for profanity and inappropriate content

use crate::error::Result;
use crate::session::{get_client, SessionLock};
use regex::Regex;
use std::collections::HashSet;
use tokio::signal;

/// Russian profanity patterns (censored for safety)
const PROFANITY_PATTERNS: &[&str] = &[
    r"(?i)\bхуй\w*",
    r"(?i)\bпизд\w*",
    r"(?i)\bбля\w*",
    r"(?i)\bеб\w*(?:ть|ал|ут|ёт)",
    r"(?i)\bсук\w*",
    r"(?i)\bмуд\w*",
    r"(?i)\bдерьм\w*",
    r"(?i)\bжоп\w*",
    r"(?i)\bср[аи]н\w*",
    r"(?i)\bнах\w*(?:уй|ер)",
];

/// Replacement word for profanity
const REPLACEMENT: &str = "хулиган";

/// Moderation configuration
pub struct ModerateConfig {
    /// Delete messages with profanity
    pub delete_profanity: bool,
    /// Send warning to user
    pub send_warning: bool,
    /// Custom replacement word
    pub replacement: String,
    /// Additional banned words
    pub banned_words: HashSet<String>,
}

impl Default for ModerateConfig {
    fn default() -> Self {
        Self {
            delete_profanity: false,
            send_warning: true,
            replacement: REPLACEMENT.to_string(),
            banned_words: HashSet::new(),
        }
    }
}

/// Profanity filter
pub struct ProfanityFilter {
    patterns: Vec<Regex>,
    replacement: String,
}

impl ProfanityFilter {
    pub fn new(replacement: &str) -> Self {
        let patterns: Vec<Regex> = PROFANITY_PATTERNS
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self {
            patterns,
            replacement: replacement.to_string(),
        }
    }

    /// Check if text contains profanity
    pub fn contains_profanity(&self, text: &str) -> bool {
        self.patterns.iter().any(|p| p.is_match(text))
    }

    /// Replace profanity with replacement word
    pub fn censor(&self, text: &str) -> String {
        let mut result = text.to_string();
        for pattern in &self.patterns {
            result = pattern.replace_all(&result, &self.replacement).to_string();
        }
        result
    }

    /// Get list of found profanity words
    pub fn find_profanity(&self, text: &str) -> Vec<String> {
        let mut found = Vec::new();
        for pattern in &self.patterns {
            for mat in pattern.find_iter(text) {
                found.push(mat.as_str().to_string());
            }
        }
        found
    }
}

/// Run moderation bot
pub async fn run(chat_name: &str, config: ModerateConfig) -> Result<()> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let filter = ProfanityFilter::new(&config.replacement);

    println!("🛡️ Модератор запущен для чата '{}'", chat_name);
    println!("Нажмите Ctrl+C для остановки.");

    let chat = crate::chat::find_chat(&client, chat_name).await?;
    let mut last_seen_id: Option<i32> = None;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("\n🛑 Останавливаю модератора...");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                let mut messages = client.iter_messages(&chat);

                if let Some(Ok(msg)) = messages.next().await.transpose() {
                    let msg_id = msg.id();

                    // Skip already seen messages
                    if let Some(last_id) = last_seen_id {
                        if msg_id <= last_id {
                            continue;
                        }
                    }
                    last_seen_id = Some(msg_id);

                    // Skip outgoing messages
                    if msg.outgoing() {
                        continue;
                    }

                    let text = msg.text().trim();
                    if text.is_empty() {
                        continue;
                    }

                    // Check for profanity
                    if filter.contains_profanity(text) {
                        let sender = if let Some(sender) = msg.sender() {
                            match sender {
                                grammers_client::types::Peer::User(u) => {
                                    u.username().map(|s| format!("@{}", s))
                                        .unwrap_or_else(|| u.full_name())
                                }
                                _ => "Unknown".to_string(),
                            }
                        } else {
                            "Unknown".to_string()
                        };

                        let found = filter.find_profanity(text);
                        println!("⚠️ Обнаружен мат от {}: {:?}", sender, found);

                        if config.delete_profanity {
                            // Note: Deleting messages requires admin rights
                            println!("🗑️ Удаление сообщения (требуются права админа)");
                        }

                        if config.send_warning {
                            let censored = filter.censor(text);
                            let warning = format!(
                                "⚠️ {}, пожалуйста, общайтесь культурно!\n\nВаше сообщение:\n{}",
                                sender, censored
                            );
                            if let Err(e) = msg.reply(warning).await {
                                eprintln!("Ошибка отправки предупреждения: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Analyze chat for profanity statistics
pub async fn analyze(chat_name: &str, limit: usize) -> Result<ProfanityStats> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let filter = ProfanityFilter::new(REPLACEMENT);
    let chat = crate::chat::find_chat(&client, chat_name).await?;

    let mut stats = ProfanityStats::default();
    let mut messages = client.iter_messages(&chat);
    let mut count = 0;

    while let Some(msg_result) = messages.next().await.transpose() {
        if count >= limit {
            break;
        }
        count += 1;

        if let Ok(msg) = msg_result {
            let text = msg.text().trim();
            if filter.contains_profanity(text) {
                stats.messages_with_profanity += 1;

                let sender = if let Some(sender) = msg.sender() {
                    match sender {
                        grammers_client::types::Peer::User(u) => {
                            u.username().map(|s| format!("@{}", s))
                                .unwrap_or_else(|| u.full_name())
                        }
                        _ => "Unknown".to_string(),
                    }
                } else {
                    "Unknown".to_string()
                };

                *stats.offenders.entry(sender).or_insert(0) += 1;

                for word in filter.find_profanity(text) {
                    *stats.word_frequency.entry(word.to_lowercase()).or_insert(0) += 1;
                }
            }
        }
        stats.total_messages += 1;
    }

    Ok(stats)
}

/// Profanity statistics
#[derive(Default)]
pub struct ProfanityStats {
    pub total_messages: usize,
    pub messages_with_profanity: usize,
    pub offenders: std::collections::HashMap<String, usize>,
    pub word_frequency: std::collections::HashMap<String, usize>,
}

impl ProfanityStats {
    pub fn profanity_rate(&self) -> f64 {
        if self.total_messages == 0 {
            return 0.0;
        }
        (self.messages_with_profanity as f64 / self.total_messages as f64) * 100.0
    }

    pub fn top_offenders(&self, n: usize) -> Vec<(&String, &usize)> {
        let mut offenders: Vec<_> = self.offenders.iter().collect();
        offenders.sort_by(|a, b| b.1.cmp(a.1));
        offenders.into_iter().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profanity_detection() {
        let filter = ProfanityFilter::new("***");

        assert!(filter.contains_profanity("Какой-то хуйня текст"));
        assert!(filter.contains_profanity("Блядь, что за дела"));
        assert!(!filter.contains_profanity("Нормальный текст"));
        assert!(!filter.contains_profanity("Привет мир"));
    }

    #[test]
    fn test_profanity_censoring() {
        let filter = ProfanityFilter::new("хулиган");

        let censored = filter.censor("Какой-то хуйня текст");
        assert!(!censored.contains("хуй"));
        assert!(censored.contains("хулиган"));
    }

    #[test]
    fn test_find_profanity() {
        let filter = ProfanityFilter::new("***");

        let found = filter.find_profanity("блядь и хуйня");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_case_insensitive() {
        let filter = ProfanityFilter::new("***");

        assert!(filter.contains_profanity("БЛЯДЬ"));
        assert!(filter.contains_profanity("Блядь"));
        assert!(filter.contains_profanity("блядь"));
    }
}
