//! Chat moderation - profanity filter, spam detection, and content moderation
//!
//! Monitors chat for profanity, spam, and inappropriate content

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

/// Spam/advertising patterns
const SPAM_PATTERNS: &[&str] = &[
    // Crypto scams
    r"(?i)зарегистрируй\w*\s+аккаунт",
    r"(?i)реферал\w*\s+программ",
    r"(?i)получ\w+\s+бонус\w*\s+до\s+\d+",
    r"(?i)bybit|binance|okx|mexc",
    r"(?i)\d+\s*usdt",
    r"(?i)крипт\w*\s+(?:биржа|кошел|заработ)",
    // MLM / Ponzi
    r"(?i)пассивн\w+\s+доход",
    r"(?i)сетев\w+\s+маркетинг",
    r"(?i)финансов\w+\s+пирамид",
    // Get rich quick
    r"(?i)заработ\w+\s+(?:от|до)\s+\d+\s*(?:₽|руб|usd|\$|долл)",
    r"(?i)бесплатн\w+\s+деньг",
    r"(?i)лёгк\w+\s+заработ",
    // Adult spam
    r"(?i)(?:секс|интим)\w*\s+(?:услуг|знаком)",
    r"(?i)эскорт\w*",
    // Telegram spam
    r"t\.me/[a-zA-Z0-9_]+(?:\?start=|/referral)",
    r"(?i)подпис\w+\s+на\s+канал",
    r"(?i)присоедин\w+\s+к\s+(?:нам|группе)",
    // Generic spam
    r"(?i)(?:жми|переходи|нажми)\s+(?:на\s+)?ссылк",
    r"(?i)(?:срочно|только\s+сегодня|акция)\s+\d+%",
];

/// Suspicious URL patterns (not outright spam but worth flagging)
const SUSPICIOUS_URL_PATTERNS: &[&str] = &[
    r"bit\.ly/",
    r"tinyurl\.com/",
    r"goo\.gl/",
    r"shorturl\.at/",
    r"clck\.ru/",
    // Suspicious TLDs
    r"https?://[^\s]+\.(?:xyz|top|club|work|click|link|online|site|website)/",
];

/// Moderation configuration
pub struct ModerateConfig {
    /// Delete messages with profanity
    pub delete_profanity: bool,
    /// Delete spam messages
    pub delete_spam: bool,
    /// Send warning to user
    pub send_warning: bool,
    /// Custom replacement word
    pub replacement: String,
    /// Additional banned words
    pub banned_words: HashSet<String>,
    /// Enable spam detection
    pub detect_spam: bool,
    /// Flag suspicious URLs (don't delete, just log)
    pub flag_suspicious_urls: bool,
}

impl Default for ModerateConfig {
    fn default() -> Self {
        Self {
            delete_profanity: false,
            delete_spam: false,
            send_warning: true,
            replacement: REPLACEMENT.to_string(),
            banned_words: HashSet::new(),
            detect_spam: true,
            flag_suspicious_urls: true,
        }
    }
}

/// Spam filter
pub struct SpamFilter {
    spam_patterns: Vec<Regex>,
    suspicious_patterns: Vec<Regex>,
}

impl SpamFilter {
    pub fn new() -> Self {
        let spam_patterns: Vec<Regex> = SPAM_PATTERNS
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        let suspicious_patterns: Vec<Regex> = SUSPICIOUS_URL_PATTERNS
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self {
            spam_patterns,
            suspicious_patterns,
        }
    }

    /// Check if text is spam
    pub fn is_spam(&self, text: &str) -> bool {
        self.spam_patterns.iter().any(|p| p.is_match(text))
    }

    /// Check if text contains suspicious URLs
    pub fn has_suspicious_urls(&self, text: &str) -> bool {
        self.suspicious_patterns.iter().any(|p| p.is_match(text))
    }

    /// Get spam score (0-100, higher = more likely spam)
    pub fn spam_score(&self, text: &str) -> u32 {
        let mut score = 0u32;

        // Count spam pattern matches
        for pattern in &self.spam_patterns {
            if pattern.is_match(text) {
                score += 30;
            }
        }

        // Count suspicious URL matches
        for pattern in &self.suspicious_patterns {
            if pattern.is_match(text) {
                score += 15;
            }
        }

        // Additional heuristics
        let text_lower = text.to_lowercase();

        // Too many emojis (spam often has many emojis)
        let emoji_count = text.chars().filter(|c| c.is_emoji()).count();
        if emoji_count > 5 {
            score += 10;
        }

        // All caps sections
        let caps_ratio = text
            .chars()
            .filter(|c| c.is_alphabetic())
            .filter(|c| c.is_uppercase())
            .count() as f32
            / text.chars().filter(|c| c.is_alphabetic()).count().max(1) as f32;
        if caps_ratio > 0.5 && text.len() > 20 {
            score += 15;
        }

        // Multiple URLs
        let url_count = text.matches("http").count();
        if url_count > 2 {
            score += 20;
        }

        // Money mentions
        if text_lower.contains('$')
            || text_lower.contains('₽')
            || text_lower.contains("руб")
            || text_lower.contains("usd")
        {
            score += 10;
        }

        score.min(100)
    }

    /// Get list of detected spam patterns
    pub fn find_spam_patterns(&self, text: &str) -> Vec<String> {
        let mut found = Vec::new();
        for pattern in &self.spam_patterns {
            for mat in pattern.find_iter(text) {
                found.push(mat.as_str().to_string());
            }
        }
        found
    }
}

impl Default for SpamFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait to check if char is emoji
trait IsEmoji {
    fn is_emoji(&self) -> bool;
}

impl IsEmoji for char {
    fn is_emoji(&self) -> bool {
        let c = *self as u32;
        // Basic emoji ranges
        (0x1F600..=0x1F64F).contains(&c)  // Emoticons
            || (0x1F300..=0x1F5FF).contains(&c)  // Misc Symbols and Pictographs
            || (0x1F680..=0x1F6FF).contains(&c)  // Transport and Map
            || (0x1F1E0..=0x1F1FF).contains(&c)  // Flags
            || (0x2600..=0x26FF).contains(&c)    // Misc symbols
            || (0x2700..=0x27BF).contains(&c)    // Dingbats
            || (0xFE00..=0xFE0F).contains(&c)    // Variation Selectors
            || (0x1F900..=0x1F9FF).contains(&c)  // Supplemental Symbols
            || (0x1FA00..=0x1FA6F).contains(&c)  // Chess Symbols
            || (0x1FA70..=0x1FAFF).contains(&c) // Symbols and Pictographs Extended-A
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
                        grammers_client::types::Peer::User(u) => u
                            .username()
                            .map(|s| format!("@{}", s))
                            .unwrap_or_else(|| u.full_name()),
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

    #[test]
    fn test_spam_detection_crypto() {
        let filter = SpamFilter::new();

        assert!(filter.is_spam("Зарегистрируйте аккаунт на Bybit"));
        assert!(filter.is_spam("Получите бонус до 6000 USDT"));
        assert!(filter.is_spam("Реферальная программа - заработай на крипте"));
    }

    #[test]
    fn test_spam_detection_mlm() {
        let filter = SpamFilter::new();

        assert!(filter.is_spam("Пассивный доход без вложений"));
        assert!(filter.is_spam("Сетевой маркетинг - путь к успеху"));
    }

    #[test]
    fn test_spam_detection_telegram() {
        let filter = SpamFilter::new();

        assert!(filter.is_spam("Переходи по ссылке t.me/channel?start=ref123"));
        assert!(filter.is_spam("Жми на ссылку прямо сейчас!"));
    }

    #[test]
    fn test_not_spam() {
        let filter = SpamFilter::new();

        assert!(!filter.is_spam("Привет, как дела?"));
        assert!(!filter.is_spam("Отличная погода сегодня"));
        assert!(!filter.is_spam("Спасибо за помощь!"));
    }

    #[test]
    fn test_suspicious_urls() {
        let filter = SpamFilter::new();

        assert!(filter.has_suspicious_urls("Смотри здесь bit.ly/abc123"));
        assert!(filter.has_suspicious_urls("Ссылка https://example.xyz/promo"));
        assert!(!filter.has_suspicious_urls("https://github.com/project"));
    }

    #[test]
    fn test_spam_score() {
        let filter = SpamFilter::new();

        // Normal message - low score
        let score1 = filter.spam_score("Привет, как дела?");
        assert!(
            score1 < 30,
            "Normal message should have low score, got {}",
            score1
        );

        // Spam message - high score
        let score2 =
            filter.spam_score("Зарегистрируйте аккаунт на Bybit! Получите бонус до 5000 USDT!");
        assert!(
            score2 >= 30,
            "Spam message should have high score, got {}",
            score2
        );

        // Another spam message with different indicators
        let score3 = filter
            .spam_score("ЗАРАБОТАЙ $1000 В ДЕНЬ!!! Переходи по ссылке bit.ly/spam 💰💰💰💰💰💰 https://scam.xyz/promo");
        assert!(
            score3 >= 30,
            "Multiple spam indicators should have high score, got {}",
            score3
        );
    }

    #[test]
    fn test_emoji_detection() {
        assert!('😀'.is_emoji());
        assert!('🔥'.is_emoji());
        assert!('❤'.is_emoji());
        assert!(!'a'.is_emoji());
        assert!(!'1'.is_emoji());
        assert!(!'я'.is_emoji());
    }

    #[test]
    fn test_find_spam_patterns() {
        let filter = SpamFilter::new();
        let patterns =
            filter.find_spam_patterns("Bybit - лучшая криптобиржа! Получите бонус до 1000 USDT");
        assert!(!patterns.is_empty());
    }
}
