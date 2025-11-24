//! User hunting/recruitment - Find users matching specific criteria in chats
//!
//! Search for potential candidates based on message content, activity, interests

use crate::error::Result;
use crate::session::{get_client, SessionLock};
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Criteria for hunting/filtering users
#[derive(Debug, Clone, Default)]
pub struct HuntCriteria {
    /// Keywords to search in messages (any match)
    pub keywords: Vec<String>,
    /// Keywords that must ALL be present
    pub required_keywords: Vec<String>,
    /// Keywords to exclude
    pub exclude_keywords: Vec<String>,
    /// Minimum activity (messages in period)
    pub min_messages: usize,
    /// Look at messages from last N days
    pub days_back: i64,
    /// Regex patterns to match
    pub patterns: Vec<String>,
    /// Only users with bio containing keywords
    pub bio_keywords: Vec<String>,
}

/// Information about a found user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuntResult {
    pub user_id: i64,
    pub username: Option<String>,
    pub full_name: String,
    pub message_count: usize,
    pub matching_messages: Vec<String>,
    pub keywords_found: Vec<String>,
    pub last_active: DateTime<Utc>,
    pub score: f64,
}

/// Hunt for users in a chat matching criteria
pub async fn hunt_users(
    chat_name: &str,
    criteria: HuntCriteria,
    max_messages: usize,
) -> Result<Vec<HuntResult>> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    println!("🎯 Hunting users in '{}' with criteria:", chat_name);
    if !criteria.keywords.is_empty() {
        println!("   Keywords: {:?}", criteria.keywords);
    }
    if !criteria.required_keywords.is_empty() {
        println!("   Required: {:?}", criteria.required_keywords);
    }
    if !criteria.exclude_keywords.is_empty() {
        println!("   Exclude: {:?}", criteria.exclude_keywords);
    }

    let chat = crate::chat::find_chat(&client, chat_name).await?;

    // Compile regex patterns
    let patterns: Vec<Regex> = criteria
        .patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    // Calculate time cutoff
    let cutoff = Utc::now() - Duration::days(criteria.days_back.max(1));

    // Collect user data
    let mut user_data: HashMap<i64, UserData> = HashMap::new();
    let mut iter = client.iter_messages(&chat);
    let mut count = 0;

    while let Some(msg_result) = iter.next().await.transpose() {
        if count >= max_messages {
            break;
        }
        count += 1;

        if let Ok(msg) = msg_result {
            let msg_time: DateTime<Utc> = msg.date();
            if msg_time < cutoff {
                break;
            }

            let text = msg.text().trim().to_string();
            if text.is_empty() {
                continue;
            }

            // Get sender info
            let (user_id, username, full_name) = if let Some(sender) = msg.sender() {
                match sender {
                    grammers_client::types::Peer::User(u) => {
                        let user_id = match &u.raw {
                            grammers_tl_types::enums::User::User(user) => user.id,
                            grammers_tl_types::enums::User::Empty(empty) => empty.id,
                        };
                        (user_id, u.username().map(String::from), u.full_name())
                    }
                    _ => continue, // Skip non-user senders
                }
            } else {
                continue;
            };

            // Check if message matches criteria
            let matches = check_message_match(&text, &criteria, &patterns);
            if matches.is_empty() && criteria.keywords.is_empty() && criteria.patterns.is_empty() {
                // If no keywords specified, collect all active users
            } else if matches.is_empty() {
                continue;
            }

            // Check exclusion keywords
            let text_lower = text.to_lowercase();
            if criteria.exclude_keywords.iter().any(|k| text_lower.contains(&k.to_lowercase())) {
                continue;
            }

            // Add to user data
            let entry = user_data.entry(user_id).or_insert_with(|| UserData {
                user_id,
                username: username.clone(),
                full_name: full_name.clone(),
                messages: Vec::new(),
                keywords_found: Vec::new(),
                last_active: msg_time,
            });

            if msg_time > entry.last_active {
                entry.last_active = msg_time;
            }

            // Store matching message (truncated)
            let truncated: String = text.chars().take(200).collect();
            entry.messages.push(truncated);
            entry.keywords_found.extend(matches);
        }
    }

    // Convert to results and filter by min_messages
    let mut results: Vec<HuntResult> = user_data
        .into_values()
        .filter(|u| u.messages.len() >= criteria.min_messages.max(1))
        .filter(|u| {
            // Check required keywords - user must have mentioned ALL of them
            criteria.required_keywords.iter().all(|req| {
                u.keywords_found.iter().any(|k| k.to_lowercase() == req.to_lowercase())
            })
        })
        .map(|u| {
            let score = calculate_score(&u, &criteria);
            HuntResult {
                user_id: u.user_id,
                username: u.username,
                full_name: u.full_name,
                message_count: u.messages.len(),
                matching_messages: u.messages.into_iter().take(5).collect(),
                keywords_found: u.keywords_found.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect(),
                last_active: u.last_active,
                score,
            }
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    println!("✅ Found {} matching users", results.len());
    Ok(results)
}

struct UserData {
    user_id: i64,
    username: Option<String>,
    full_name: String,
    messages: Vec<String>,
    keywords_found: Vec<String>,
    last_active: DateTime<Utc>,
}

fn check_message_match(text: &str, criteria: &HuntCriteria, patterns: &[Regex]) -> Vec<String> {
    let mut matches = Vec::new();
    let text_lower = text.to_lowercase();

    // Check keywords
    for keyword in &criteria.keywords {
        if text_lower.contains(&keyword.to_lowercase()) {
            matches.push(keyword.clone());
        }
    }

    // Check required keywords
    for keyword in &criteria.required_keywords {
        if text_lower.contains(&keyword.to_lowercase()) {
            matches.push(keyword.clone());
        }
    }

    // Check regex patterns
    for pattern in patterns {
        if pattern.is_match(text) {
            if let Some(mat) = pattern.find(text) {
                matches.push(mat.as_str().to_string());
            }
        }
    }

    matches
}

fn calculate_score(user: &UserData, criteria: &HuntCriteria) -> f64 {
    let mut score = 0.0;

    // More messages = higher score
    score += (user.messages.len() as f64).ln() * 10.0;

    // More keywords found = higher score
    score += user.keywords_found.len() as f64 * 5.0;

    // Recent activity bonus
    let days_since_active = (Utc::now() - user.last_active).num_days();
    if days_since_active < 1 {
        score += 20.0;
    } else if days_since_active < 7 {
        score += 10.0;
    }

    // Required keywords bonus
    let required_found = criteria.required_keywords.iter()
        .filter(|req| user.keywords_found.iter().any(|k| k.to_lowercase() == req.to_lowercase()))
        .count();
    score += required_found as f64 * 15.0;

    score
}

/// Print hunt results in a readable format
pub fn print_results(results: &[HuntResult], limit: usize) {
    println!("\n🎯 Hunt Results ({} users found)\n", results.len());

    for (i, user) in results.iter().take(limit).enumerate() {
        let username = user.username.as_ref()
            .map(|u| format!("@{}", u))
            .unwrap_or_else(|| format!("id:{}", user.user_id));

        println!("{}. {} ({}) - Score: {:.1}", i + 1, user.full_name, username, user.score);
        println!("   📊 Messages: {}, Keywords: {:?}", user.message_count, user.keywords_found);
        println!("   🕐 Last active: {}", user.last_active.format("%d.%m.%Y %H:%M"));

        if !user.matching_messages.is_empty() {
            println!("   💬 Sample message:");
            println!("      \"{}...\"", user.matching_messages[0].chars().take(100).collect::<String>());
        }
        println!();
    }
}

/// Export results to CSV
pub fn export_csv(results: &[HuntResult]) -> String {
    let mut csv = String::from("user_id,username,full_name,message_count,score,last_active,keywords\n");

    for result in results {
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",{},{:.1},{},\"{}\"\n",
            result.user_id,
            result.username.as_deref().unwrap_or(""),
            result.full_name.replace('"', "'"),
            result.message_count,
            result.score,
            result.last_active.format("%Y-%m-%d %H:%M"),
            result.keywords_found.join("; ")
        ));
    }

    csv
}

/// Search multiple chats for users matching criteria
pub async fn hunt_multiple_chats(
    chat_names: &[&str],
    criteria: HuntCriteria,
    max_messages_per_chat: usize,
) -> Result<Vec<HuntResult>> {
    let mut all_results: HashMap<i64, HuntResult> = HashMap::new();

    for chat_name in chat_names {
        println!("\n📡 Scanning chat: {}", chat_name);
        match hunt_users(chat_name, criteria.clone(), max_messages_per_chat).await {
            Ok(results) => {
                for result in results {
                    // Merge results for same user
                    let entry = all_results.entry(result.user_id).or_insert(result.clone());
                    if result.score > entry.score {
                        *entry = result;
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️ Error scanning {}: {}", chat_name, e);
            }
        }
    }

    let mut results: Vec<HuntResult> = all_results.into_values().collect();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_message_match_keywords() {
        let criteria = HuntCriteria {
            keywords: vec!["курьер".to_string(), "доставка".to_string()],
            ..Default::default()
        };

        let matches = check_message_match("Ищу работу курьером", &criteria, &[]);
        assert!(matches.contains(&"курьер".to_string()));

        let matches = check_message_match("Привет мир", &criteria, &[]);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_check_message_match_patterns() {
        let criteria = HuntCriteria::default();
        let patterns = vec![Regex::new(r"\d{3,}[₽$€]").unwrap()];

        let matches = check_message_match("Зарплата 50000₽", &criteria, &patterns);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_export_csv() {
        let results = vec![HuntResult {
            user_id: 123,
            username: Some("testuser".to_string()),
            full_name: "Test User".to_string(),
            message_count: 10,
            matching_messages: vec!["Hello".to_string()],
            keywords_found: vec!["keyword".to_string()],
            last_active: Utc::now(),
            score: 50.0,
        }];

        let csv = export_csv(&results);
        assert!(csv.contains("testuser"));
        assert!(csv.contains("Test User"));
    }
}
