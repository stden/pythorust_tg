//! AI-powered chat analyzer (Rust port of `chat_analyzer.py`).
//!
//! Features:
//! - Fetch recent messages from a chat with basic filtering
//! - Format data for LLM analysis (OpenAI/Claude/Gemini)
//! - Parse JSON response and save as JSON + Markdown reports

use crate::chat::find_chat;
use crate::integrations::{ClaudeClient, GeminiClient, OpenAIClient};
use crate::reactions::count_reactions;
use crate::session::{get_client, SessionLock};
use crate::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use grammers_client::types::peer::Peer;
use grammers_client::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

const SYSTEM_MESSAGE: &str =
    "You are an expert Telegram chat analyzer. Always respond with valid JSON that matches the requested schema.";

// Fallback prompt if prompts/chat_categorizer.md is missing.
const FALLBACK_PROMPT: &str = r#"You are a chat analyzer. Analyze the provided Telegram chat messages and provide a comprehensive analysis.

Your analysis must be in JSON format with the following structure:

{
  "category": "primary category (e.g., IT, Business, Community, Education)",
  "subcategories": ["subcategory1", "subcategory2"],
  "sentiment": "overall sentiment (positive/negative/neutral/mixed)",
  "activity_level": "activity level (high/medium/low)",
  "professionalism": "professionalism level (professional/casual/mixed)",
  "topics": [
    {
      "name": "topic name",
      "mentions": 10,
      "sentiment": "positive/negative/neutral",
      "key_message_ids": [123, 456]
    }
  ],
  "discussions": [
    {
      "title": "discussion title",
      "date": "2025-11-24",
      "participants": ["User1", "User2"],
      "messages_count": 15,
      "summary": "brief summary of discussion"
    }
  ],
  "key_participants": [
    {
      "name": "User Name",
      "message_count": 50,
      "engagement_score": 8.5
    }
  ],
  "summary": "Overall summary of the chat (2-3 sentences)",
  "insights": ["insight 1", "insight 2", "insight 3"],
  "recommendations": ["recommendation 1", "recommendation 2"]
}

Analyze the messages and provide your response in this exact JSON format."#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Markdown,
    Both,
}

impl OutputFormat {
    pub fn from_str(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "markdown" | "md" => OutputFormat::Markdown,
            _ => OutputFormat::Both,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    OpenAI,
    Claude,
    Gemini,
}

impl LlmProvider {
    pub fn from_str(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "claude" => LlmProvider::Claude,
            "gemini" => LlmProvider::Gemini,
            _ => LlmProvider::OpenAI,
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            LlmProvider::OpenAI => "gpt-4o-mini",
            LlmProvider::Claude => "claude-sonnet-4-5-20250929",
            LlmProvider::Gemini => "gemini-2.0-flash",
        }
    }
}

/// Analyzer configuration.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub message_limit: usize,
    pub days_back: i64,
    pub llm_provider: LlmProvider,
    pub model: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub min_message_length: usize,
    pub include_media: bool,
    pub exclude_bots: bool,
    pub output_format: OutputFormat,
    pub output_dir: PathBuf,
    pub prompt_path: Option<PathBuf>,
    pub verbose: bool,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        // Prefer environment overrides for compatibility with the Python tool.
        let provider = std::env::var("CHAT_ANALYZER_LLM_PROVIDER")
            .ok()
            .map(|p| LlmProvider::from_str(&p))
            .unwrap_or(LlmProvider::OpenAI);

        let output_dir = std::env::var("ANALYSIS_RESULTS_DIR")
            .or_else(|_| std::env::var("CHAT_ANALYZER_OUTPUT_DIR"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("analysis_results"));

        Self {
            message_limit: 1000,
            days_back: 30,
            llm_provider: provider,
            model: std::env::var("CHAT_ANALYZER_MODEL").ok(),
            temperature: 0.3,
            max_tokens: 2000,
            min_message_length: 10,
            include_media: false,
            exclude_bots: true,
            output_format: OutputFormat::Both,
            output_dir,
            prompt_path: None,
            verbose: true,
        }
    }
}

impl AnalyzerConfig {
    pub fn resolved_model(&self) -> String {
        self.model
            .clone()
            .unwrap_or_else(|| self.llm_provider.default_model().to_string())
    }
}

#[derive(Debug, Clone)]
struct FormattedMessage {
    date: DateTime<Utc>,
    sender_name: String,
    text: String,
    message_id: i32,
    reactions_count: i32,
    has_media: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Topic {
    pub name: String,
    pub mentions: i64,
    pub sentiment: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub key_message_ids: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Discussion {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<String>,
    pub messages_count: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct KeyParticipant {
    pub name: String,
    pub message_count: i64,
    pub engagement_score: f32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ActivityMetrics {
    pub total_messages: usize,
    pub active_users: usize,
    pub messages_per_day: f32,
    pub avg_message_length: f32,
    pub media_percentage: f32,
    pub reactions_count: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatAnalysisResult {
    pub chat_name: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub analyzed_at: DateTime<Utc>,
    pub category: String,
    pub subcategories: Vec<String>,
    pub sentiment: String,
    pub activity_level: String,
    pub professionalism: String,
    pub topics: Vec<Topic>,
    pub discussions: Vec<Discussion>,
    pub key_participants: Vec<KeyParticipant>,
    pub activity_metrics: ActivityMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range_end: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub summary: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub insights: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recommendations: Vec<String>,
}

impl ChatAnalysisResult {
    pub fn save_json(&self, path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| Error::InvalidArgument(format!("Failed to serialize JSON: {}", e)))?;
        std::fs::write(path, data)
            .map_err(|e| Error::InvalidArgument(format!("Failed to write JSON: {}", e)))?;
        Ok(())
    }

    pub fn save_markdown(&self, path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        std::fs::write(path, self.to_markdown())
            .map_err(|e| Error::InvalidArgument(format!("Failed to write Markdown: {}", e)))?;
        Ok(())
    }

    fn to_markdown(&self) -> String {
        let mut lines = Vec::new();

        lines.push("# Chat Analysis Report".to_string());
        lines.push(String::new());
        lines.push(format!("**Chat:** {}", self.chat_name));
        lines.push(format!(
            "**Analyzed:** {}",
            self.analyzed_at.format("%Y-%m-%d %H:%M:%S")
        ));
        lines.push(String::new());

        lines.push("## 📂 Categorization".to_string());
        lines.push(String::new());
        lines.push(format!("- **Category:** {}", self.category));
        lines.push(format!(
            "- **Subcategories:** {}",
            if self.subcategories.is_empty() {
                "—".to_string()
            } else {
                self.subcategories.join(", ")
            }
        ));
        lines.push(format!("- **Sentiment:** {}", self.sentiment));
        lines.push(format!("- **Activity Level:** {}", self.activity_level));
        lines.push(format!("- **Professionalism:** {}", self.professionalism));
        lines.push(String::new());

        if !self.summary.is_empty() {
            lines.push("## 📋 Summary".to_string());
            lines.push(String::new());
            lines.push(self.summary.clone());
            lines.push(String::new());
        }

        if let (Some(start), Some(end)) = (&self.date_range_start, &self.date_range_end) {
            lines.push(format!("**Period:** {} → {}", start, end));
            lines.push(String::new());
        }

        let m = &self.activity_metrics;
        lines.push("## 📊 Activity Metrics".to_string());
        lines.push(String::new());
        lines.push(format!("- **Total Messages:** {}", m.total_messages));
        lines.push(format!("- **Active Users:** {}", m.active_users));
        lines.push(format!("- **Messages/Day:** {:.1}", m.messages_per_day));
        lines.push(format!(
            "- **Avg Message Length:** {:.1} characters",
            m.avg_message_length
        ));
        lines.push(format!(
            "- **Media Percentage:** {:.1}%",
            m.media_percentage
        ));
        lines.push(format!("- **Total Reactions:** {}", m.reactions_count));
        lines.push(String::new());

        if !self.topics.is_empty() {
            lines.push("## 💬 Topics".to_string());
            lines.push(String::new());
            for (idx, topic) in self.topics.iter().enumerate() {
                lines.push(format!("### {}. {}", idx + 1, topic.name));
                lines.push(String::new());
                lines.push(format!("- **Mentions:** {}", topic.mentions));
                lines.push(format!("- **Sentiment:** {}", topic.sentiment));
                if !topic.key_message_ids.is_empty() {
                    let ids: Vec<String> = topic
                        .key_message_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect();
                    lines.push(format!("- **Key Messages:** {}", ids.join(", ")));
                }
                lines.push(String::new());
            }
        }

        if !self.discussions.is_empty() {
            lines.push("## 🗣️ Key Discussions".to_string());
            lines.push(String::new());
            for (idx, disc) in self.discussions.iter().enumerate() {
                lines.push(format!("### {}. {}", idx + 1, disc.title));
                lines.push(String::new());
                if let Some(date) = &disc.date {
                    lines.push(format!("- **Date:** {}", date));
                }
                if !disc.participants.is_empty() {
                    let display = if disc.participants.len() > 10 {
                        let mut list = disc.participants[..10].join(", ");
                        list.push_str(&format!(" (+{} more)", disc.participants.len() - 10));
                        list
                    } else {
                        disc.participants.join(", ")
                    };
                    lines.push(format!("- **Participants:** {}", display));
                }
                lines.push(format!("- **Messages:** {}", disc.messages_count));
                if !disc.summary.is_empty() {
                    lines.push(String::new());
                    lines.push(disc.summary.clone());
                }
                lines.push(String::new());
            }
        }

        if !self.key_participants.is_empty() {
            lines.push("## 👥 Key Participants".to_string());
            lines.push(String::new());
            for participant in &self.key_participants {
                lines.push(format!(
                    "- **{}** — {} messages (engagement: {:.1}/10)",
                    participant.name, participant.message_count, participant.engagement_score
                ));
            }
            lines.push(String::new());
        }

        if !self.insights.is_empty() {
            lines.push("## 💡 Insights".to_string());
            lines.push(String::new());
            for insight in &self.insights {
                lines.push(format!("- {}", insight));
            }
            lines.push(String::new());
        }

        if !self.recommendations.is_empty() {
            lines.push("## 🎯 Recommendations".to_string());
            lines.push(String::new());
            for rec in &self.recommendations {
                lines.push(format!("- {}", rec));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

/// Analyze chat and write results to disk.
pub async fn run(chat: &str, config: AnalyzerConfig) -> Result<ChatAnalysisResult> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;
    analyze_with_client(&client, chat, config).await
}

async fn analyze_with_client(
    client: &Client,
    chat: &str,
    config: AnalyzerConfig,
) -> Result<ChatAnalysisResult> {
    if config.verbose {
        info!(
            "Analyzing chat '{}' (provider: {:?}, limit: {}, days: {})",
            chat, config.llm_provider, config.message_limit, config.days_back
        );
    }

    let collected = collect_messages(client, chat, &config).await?;
    if collected.messages.is_empty() {
        return Err(Error::InvalidArgument(format!(
            "No messages found in chat '{}'",
            chat
        )));
    }

    let messages_text = format_messages_for_llm(&collected.messages);
    let metadata = build_metadata(&collected.stats);
    let prompt_template = load_prompt(config.prompt_path.as_deref());
    let prompt = build_prompt(&prompt_template, &messages_text, &metadata, chat);

    let llm_raw = call_llm(
        config.llm_provider,
        &config.resolved_model(),
        &prompt,
        config.temperature,
        config.max_tokens,
    )
    .await?;

    let result = build_result(chat, &llm_raw, &collected.stats, &collected.sender_counts);

    write_outputs(&result, &config)?;

    if config.verbose {
        info!("Analysis complete");
    }

    Ok(result)
}

struct MessageStats {
    total_messages: usize,
    unique_senders: usize,
    total_reactions: i32,
    has_media: bool,
    avg_length: f32,
    media_percentage: f32,
    messages_per_day: f32,
    date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

struct CollectedMessages {
    messages: Vec<FormattedMessage>,
    sender_counts: HashMap<String, usize>,
    stats: MessageStats,
}

async fn collect_messages(
    client: &Client,
    chat: &str,
    config: &AnalyzerConfig,
) -> Result<CollectedMessages> {
    let peer = find_chat(client, chat).await?;

    let cutoff = if config.days_back > 0 {
        Some(Utc::now() - Duration::days(config.days_back))
    } else {
        None
    };

    let mut messages = Vec::new();
    let mut sender_counts: HashMap<String, usize> = HashMap::new();
    let mut unique_senders: HashSet<String> = HashSet::new();
    let mut total_reactions = 0;
    let mut total_length: usize = 0;
    let mut media_count = 0;
    let mut earliest: Option<DateTime<Utc>> = None;
    let mut latest: Option<DateTime<Utc>> = None;

    let mut iter = client.iter_messages(&peer);
    while let Some(msg) = iter.next().await.transpose() {
        let msg = msg.map_err(|e| Error::TelegramError(e.to_string()))?;

        if messages.len() >= config.message_limit {
            break;
        }

        if let Some(cutoff) = cutoff {
            if msg.date() < cutoff {
                break;
            }
        }

        let text = msg.text();
        if text.is_empty() || text.chars().count() < config.min_message_length {
            continue;
        }

        if config.exclude_bots && is_bot(&msg) {
            continue;
        }

        let has_media = msg.media().is_some();
        if !config.include_media && has_media && text.chars().count() < config.min_message_length {
            continue;
        }

        let sender_name = sender_name(&msg);
        let reactions_count = count_reactions(&msg);

        messages.push(FormattedMessage {
            date: msg.date(),
            sender_name: sender_name.clone(),
            text: text.to_string(),
            message_id: msg.id(),
            reactions_count,
            has_media,
        });

        *sender_counts.entry(sender_name.clone()).or_insert(0) += 1;
        unique_senders.insert(sender_name);
        total_reactions += reactions_count;
        total_length += text.chars().count();
        if has_media {
            media_count += 1;
        }

        earliest = Some(earliest.map_or(msg.date(), |d| d.min(msg.date())));
        latest = Some(latest.map_or(msg.date(), |d| d.max(msg.date())));
    }

    // Reverse to chronological order for better LLM context.
    messages.reverse();

    let total_messages = messages.len();
    let avg_length = if total_messages > 0 {
        total_length as f32 / total_messages as f32
    } else {
        0.0
    };
    let media_percentage = if total_messages > 0 {
        (media_count as f32 / total_messages as f32) * 100.0
    } else {
        0.0
    };
    let messages_per_day = if let Some((start, end)) = earliest.zip(latest) {
        let days = (end - start).num_days().max(0) + 1;
        if days > 0 {
            total_messages as f32 / days as f32
        } else {
            total_messages as f32
        }
    } else {
        0.0
    };

    let stats = MessageStats {
        total_messages,
        unique_senders: unique_senders.len(),
        total_reactions,
        has_media: media_count > 0,
        avg_length,
        media_percentage,
        messages_per_day,
        date_range: earliest.zip(latest),
    };

    Ok(CollectedMessages {
        messages,
        sender_counts,
        stats,
    })
}

fn format_messages_for_llm(messages: &[FormattedMessage]) -> String {
    let mut lines = Vec::new();
    for msg in messages {
        let reactions = if msg.reactions_count > 0 {
            format!(" [{} reactions]", msg.reactions_count)
        } else {
            String::new()
        };
        let media_marker = if msg.has_media { " [media]" } else { "" };
        lines.push(format!(
            "[{}] {}: {}{}{}",
            msg.date.format("%d.%m.%Y %H:%M"),
            msg.sender_name,
            msg.text,
            reactions,
            media_marker
        ));
    }
    lines.join("\n")
}

fn build_metadata(stats: &MessageStats) -> Value {
    let date_range = stats.date_range.map(|(start, end)| {
        json!({
            "start": start.to_rfc3339(),
            "end": end.to_rfc3339(),
        })
    });

    json!({
        "total_messages": stats.total_messages,
        "date_range": date_range,
        "unique_senders": stats.unique_senders,
        "has_media": stats.has_media,
        "total_reactions": stats.total_reactions,
    })
}

fn load_prompt(prompt_path: Option<&Path>) -> String {
    if let Some(path) = prompt_path {
        if let Ok(text) = std::fs::read_to_string(path) {
            return text;
        }
    }

    let default_path = prompt_dir().join("chat_categorizer.md");
    if let Ok(text) = std::fs::read_to_string(&default_path) {
        return text;
    }

    FALLBACK_PROMPT.to_string()
}

fn prompt_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PROMPTS_DIR") {
        return PathBuf::from(dir);
    }
    for candidate in [
        PathBuf::from("prompts"),
        PathBuf::from("../prompts"),
        PathBuf::from("../../prompts"),
    ] {
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from("prompts")
}

fn build_prompt(template: &str, messages: &str, metadata: &Value, chat: &str) -> String {
    format!(
        "{template}\n\n## Chat Metadata\n{}\n\n## Chat Name\n{}\n\n## Messages\n{}\n\nProvide your analysis in JSON format as specified above.",
        serde_json::to_string_pretty(metadata).unwrap_or_default(),
        chat,
        messages
    )
}

async fn call_llm(
    provider: LlmProvider,
    model: &str,
    prompt: &str,
    temperature: f32,
    max_tokens: u32,
) -> Result<String> {
    match provider {
        LlmProvider::OpenAI => {
            let client = OpenAIClient::from_env()?;
            let messages = vec![
                crate::integrations::openai::ChatMessage {
                    role: "system".to_string(),
                    content: Some(SYSTEM_MESSAGE.to_string()),
                },
                crate::integrations::openai::ChatMessage {
                    role: "user".to_string(),
                    content: Some(prompt.to_string()),
                },
            ];
            client
                .chat_completion(messages, model, temperature, max_tokens)
                .await
        }
        LlmProvider::Claude => {
            let client = ClaudeClient::from_env()?.with_model(model);
            client.chat_with_system(prompt, Some(SYSTEM_MESSAGE)).await
        }
        LlmProvider::Gemini => {
            let client = GeminiClient::from_env()?.with_model(model);
            client.chat_with_system(prompt, Some(SYSTEM_MESSAGE)).await
        }
    }
}

fn build_result(
    chat: &str,
    llm_raw: &str,
    stats: &MessageStats,
    sender_counts: &HashMap<String, usize>,
) -> ChatAnalysisResult {
    let parsed = parse_llm_json(llm_raw);

    let category = parsed
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    let subcategories = parsed
        .get("subcategories")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    let sentiment = parsed
        .get("sentiment")
        .and_then(Value::as_str)
        .unwrap_or("neutral")
        .to_string();
    let activity_level = parsed
        .get("activity_level")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let professionalism = parsed
        .get("professionalism")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    let topics = parse_topics(&parsed);
    let discussions = parse_discussions(&parsed);
    let mut key_participants = parse_participants(&parsed);
    if key_participants.is_empty() {
        key_participants = fallback_participants(sender_counts);
    }

    let date_range_start = stats.date_range.map(|(start, _)| start.to_rfc3339());
    let date_range_end = stats.date_range.map(|(_, end)| end.to_rfc3339());

    let activity_metrics = ActivityMetrics {
        total_messages: stats.total_messages,
        active_users: stats.unique_senders,
        messages_per_day: stats.messages_per_day,
        avg_message_length: stats.avg_length,
        media_percentage: stats.media_percentage,
        reactions_count: stats.total_reactions,
    };

    let summary = parsed
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let insights = parsed
        .get("insights")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let recommendations = parsed
        .get("recommendations")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    ChatAnalysisResult {
        chat_name: chat.to_string(),
        analyzed_at: Utc::now(),
        category,
        subcategories,
        sentiment,
        activity_level,
        professionalism,
        topics,
        discussions,
        key_participants,
        activity_metrics,
        date_range_start,
        date_range_end,
        summary,
        insights,
        recommendations,
    }
}

fn parse_topics(parsed: &Value) -> Vec<Topic> {
    parsed
        .get("topics")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| Topic {
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("Unknown")
                        .to_string(),
                    mentions: item.get("mentions").and_then(Value::as_i64).unwrap_or(0),
                    sentiment: item
                        .get("sentiment")
                        .and_then(Value::as_str)
                        .unwrap_or("neutral")
                        .to_string(),
                    key_message_ids: item
                        .get("key_message_ids")
                        .and_then(Value::as_array)
                        .map(|ids| {
                            ids.iter()
                                .filter_map(Value::as_i64)
                                .map(|v| v as i32)
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_discussions(parsed: &Value) -> Vec<Discussion> {
    parsed
        .get("discussions")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| Discussion {
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("Unknown Discussion")
                        .to_string(),
                    date: item
                        .get("date")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string()),
                    participants: item
                        .get("participants")
                        .and_then(Value::as_array)
                        .map(|vals| {
                            vals.iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default(),
                    messages_count: item
                        .get("messages_count")
                        .and_then(Value::as_i64)
                        .unwrap_or(0),
                    summary: item
                        .get("summary")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_participants(parsed: &Value) -> Vec<KeyParticipant> {
    parsed
        .get("key_participants")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| KeyParticipant {
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("Unknown")
                        .to_string(),
                    message_count: item
                        .get("message_count")
                        .and_then(Value::as_i64)
                        .unwrap_or(0),
                    engagement_score: item
                        .get("engagement_score")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0) as f32,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn fallback_participants(sender_counts: &HashMap<String, usize>) -> Vec<KeyParticipant> {
    let mut counts: Vec<(&String, &usize)> = sender_counts.iter().collect();
    counts.sort_by(|a, b| b.1.cmp(a.1));

    counts
        .into_iter()
        .take(5)
        .map(|(name, count)| KeyParticipant {
            name: name.clone(),
            message_count: *count as i64,
            engagement_score: 5.0,
        })
        .collect()
}

fn parse_llm_json(raw: &str) -> Value {
    let cleaned = strip_code_fences(raw.trim());
    serde_json::from_str(&cleaned).unwrap_or_else(|e| {
        warn!("Failed to parse LLM JSON: {}", e);
        json!({})
    })
}

fn strip_code_fences(text: &str) -> String {
    let mut trimmed = text.trim().to_string();
    if trimmed.starts_with("```json") {
        trimmed = trimmed.trim_start_matches("```json").to_string();
    } else if trimmed.starts_with("```") {
        trimmed = trimmed.trim_start_matches("```").to_string();
    }
    if trimmed.ends_with("```") {
        trimmed.truncate(trimmed.len().saturating_sub(3));
    }
    trimmed.trim().to_string()
}

fn write_outputs(result: &ChatAnalysisResult, config: &AnalyzerConfig) -> Result<()> {
    ensure_dir(&config.output_dir)?;
    let safe_chat = sanitize_filename(&result.chat_name);
    let timestamp = result.analyzed_at.format("%Y%m%d_%H%M%S");
    let base = format!("{}_{}", safe_chat, timestamp);

    match config.output_format {
        OutputFormat::Json => {
            let path = config.output_dir.join(format!("{base}.json"));
            result.save_json(&path)?;
            if config.verbose {
                info!("Saved JSON: {}", path.display());
            }
        }
        OutputFormat::Markdown => {
            let path = config.output_dir.join(format!("{base}.md"));
            result.save_markdown(&path)?;
            if config.verbose {
                info!("Saved Markdown: {}", path.display());
            }
        }
        OutputFormat::Both => {
            let json_path = config.output_dir.join(format!("{base}.json"));
            let md_path = config.output_dir.join(format!("{base}.md"));
            result.save_json(&json_path)?;
            result.save_markdown(&md_path)?;
            if config.verbose {
                info!("Saved JSON: {}", json_path.display());
                info!("Saved Markdown: {}", md_path.display());
            }
        }
    }

    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::InvalidArgument(format!(
                "Failed to create directories for {}: {}",
                path.display(),
                e
            ))
        })?;
    }
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|e| {
        Error::InvalidArgument(format!("Failed to create dir {}: {}", path.display(), e))
    })
}

fn sender_name(msg: &grammers_client::types::Message) -> String {
    if let Some(sender) = msg.sender() {
        match sender {
            Peer::User(u) => u
                .username()
                .map(|u| format!("@{}", u))
                .unwrap_or_else(|| u.full_name()),
            Peer::Channel(c) => c.title().to_string(),
            Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
        }
    } else {
        "Unknown".to_string()
    }
}

fn is_bot(msg: &grammers_client::types::Message) -> bool {
    if let Some(sender) = msg.sender() {
        if let Peer::User(u) = sender {
            match &u.raw {
                grammers_tl_types::enums::User::User(user) => user.bot,
                grammers_tl_types::enums::User::Empty(_) => false,
            }
        } else {
            false
        }
    } else {
        false
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn serialize_datetime<S>(dt: &DateTime<Utc>, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&dt.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn strips_code_fences() {
        let input = "```json\n{\"a\":1}\n```";
        assert_eq!(strip_code_fences(input), "{\"a\":1}");
    }

    #[test]
    fn parses_llm_json_gracefully() {
        let parsed = parse_llm_json("not json");
        assert!(parsed.is_object());
    }

    #[test]
    fn sanitizes_filename() {
        assert_eq!(sanitize_filename("chat@name"), "chat_name");
        assert_eq!(sanitize_filename("Русский"), "_______");
    }

    #[test]
    fn parses_topics_with_defaults() {
        let data = json!({
            "topics": [
                {"name": "AI", "mentions": 5, "sentiment": "positive", "key_message_ids": [1,2]}
            ]
        });
        let topics = parse_topics(&data);
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].name, "AI");
        assert_eq!(topics[0].mentions, 5);
    }

    #[test]
    fn formats_messages_for_llm_includes_reactions_and_media() {
        let dt = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let messages = vec![FormattedMessage {
            date: dt,
            sender_name: "Alice".to_string(),
            text: "Hello world".to_string(),
            message_id: 1,
            reactions_count: 3,
            has_media: true,
        }];

        let formatted = format_messages_for_llm(&messages);
        assert!(formatted.contains("Alice"));
        assert!(formatted.contains("[3 reactions]"));
        assert!(formatted.contains("[media]"));
        assert!(formatted.contains("01.01.2024 12:00"));
    }

    #[test]
    fn metadata_includes_date_range_and_counts() {
        let start = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = chrono::Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap();
        let stats = MessageStats {
            total_messages: 10,
            unique_senders: 4,
            total_reactions: 7,
            has_media: true,
            avg_length: 12.0,
            media_percentage: 20.0,
            messages_per_day: 5.0,
            date_range: Some((start, end)),
        };

        let meta = build_metadata(&stats);
        assert_eq!(meta["total_messages"], 10);
        assert_eq!(meta["unique_senders"], 4);
        assert_eq!(meta["total_reactions"], 7);
        assert!(meta["date_range"]["start"].as_str().unwrap().contains("2024-01-01"));
    }

    #[test]
    fn fallback_participants_returns_sorted_top5() {
        let mut counts = std::collections::HashMap::new();
        for (name, count) in [
            ("Alice", 10usize),
            ("Bob", 3),
            ("Carol", 7),
            ("Dave", 2),
            ("Eve", 5),
            ("Frank", 1),
        ] {
            counts.insert(name.to_string(), count);
        }

        let participants = fallback_participants(&counts);
        assert_eq!(participants.len(), 5);
        assert_eq!(participants[0].name, "Alice");
        assert_eq!(participants[0].message_count, 10);
        // Ensure sorted descending
        let counts_sorted: Vec<i64> = participants.iter().map(|p| p.message_count).collect();
        assert!(counts_sorted.windows(2).all(|w| w[0] >= w[1]));
    }
}
