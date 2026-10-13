//! Dialog evaluation using AI.
//!
//! Evaluates bot dialogues using OpenAI (acting as ChatGPT 5.1 QA).

use mysql_async::{prelude::*, Pool};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::integrations::OpenAIClient;
use crate::{Error, Result};

const EVALUATION_PROMPT: &str = r#"Ты — ChatGPT 5.1, передовой ИИ для контроля качества работы операторов и ботов.
Твоя задача — проанализировать диалог между Кредитным Экспертом (бот) и Клиентом.

КРИТЕРИИ ОЦЕНКИ (по 10-балльной шкале):
1. Эмпатия (насколько бот был вежлив и поддерживал клиента).
2. Следование скрипту (выявление долга, просрочек, кредиторов).
3. Работа с возражениями (успокоил ли клиента, объяснил ли законность).
4. Закрытие (попытался ли взять номер телефона или записать на консультацию).

ФОРМАТ ОТВЕТА:
Общая оценка: X/10
Плюсы: ...
Минусы: ...
Рекомендации: ..."#;

/// Message direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

impl From<&str> for MessageDirection {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "outgoing" => MessageDirection::Outgoing,
            _ => MessageDirection::Incoming,
        }
    }
}

/// Dialog message for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogMessage {
    pub direction: MessageDirection,
    pub message_text: String,
}

/// Evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub session_id: i64,
    pub message_count: usize,
    pub evaluation: String,
}

/// Dialog evaluator using OpenAI.
pub struct DialogEvaluator {
    pool: Pool,
    ai_client: OpenAIClient,
    model: String,
}

impl DialogEvaluator {
    /// Create new dialog evaluator.
    pub fn new(pool: Pool) -> Result<Self> {
        let ai_client = OpenAIClient::from_env()?;
        Ok(Self {
            pool,
            ai_client,
            model: "gpt-4o".to_string(),
        })
    }

    /// Create evaluator with custom model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Fetch conversation history for a user.
    pub async fn get_conversation_history(
        &self,
        user_id: i64,
        limit: u32,
    ) -> Result<Vec<DialogMessage>> {
        let mut conn = self.pool.get_conn().await?;

        let rows: Vec<(String, String)> = conn
            .exec(
                r#"
                SELECT direction, message_text
                FROM bot_messages
                WHERE user_id = ?
                ORDER BY created_at ASC
                LIMIT ?
                "#,
                (user_id, limit),
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|(direction, message_text)| DialogMessage {
                direction: MessageDirection::from(direction.as_str()),
                message_text,
            })
            .collect())
    }

    /// Build transcript from messages.
    fn build_transcript(messages: &[DialogMessage]) -> String {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.direction {
                    MessageDirection::Outgoing => "Бот",
                    MessageDirection::Incoming => "Клиент",
                };
                format!("{}: {}", role, msg.message_text)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Evaluate a single session.
    pub async fn evaluate_session(
        &self,
        session_id: i64,
        messages: &[DialogMessage],
    ) -> Result<EvaluationResult> {
        if messages.is_empty() {
            warn!(session_id = session_id, "Session has no messages");
            return Err(Error::InvalidArgument(format!(
                "Session {} has no messages",
                session_id
            )));
        }

        let transcript = Self::build_transcript(messages);

        info!(
            session_id = session_id,
            message_count = messages.len(),
            "Evaluating session"
        );

        let ai_messages = vec![
            crate::integrations::openai::ChatMessage {
                role: "system".to_string(),
                content: Some(EVALUATION_PROMPT.to_string()),
            },
            crate::integrations::openai::ChatMessage {
                role: "user".to_string(),
                content: Some(format!("Вот диалог для анализа:\n\n{}", transcript)),
            },
        ];

        let evaluation = self
            .ai_client
            .chat_completion(ai_messages, &self.model, 0.3, 2000)
            .await?;

        Ok(EvaluationResult {
            session_id,
            message_count: messages.len(),
            evaluation,
        })
    }

    /// Evaluate recent sessions.
    pub async fn evaluate_recent_sessions(&self, limit: u32) -> Result<Vec<EvaluationResult>> {
        let mut conn = self.pool.get_conn().await?;

        // Fetch recent users with activity
        let users: Vec<i64> = conn
            .query(format!(
                r#"
                    SELECT user_id
                    FROM bot_messages
                    GROUP BY user_id
                    ORDER BY MAX(created_at) DESC
                    LIMIT {}
                    "#,
                limit
            ))
            .await?;

        let mut results = Vec::new();

        for user_id in users {
            let history = self.get_conversation_history(user_id, 50).await?;

            if history.is_empty() {
                continue;
            }

            match self.evaluate_session(user_id, &history).await {
                Ok(result) => {
                    println!("\n--- ОТЧЕТ ПО СЕССИИ {} ---\n", result.session_id);
                    println!("{}", result.evaluation);
                    println!("\n-----------------------------------\n");
                    results.push(result);
                }
                Err(e) => {
                    warn!(user_id = user_id, error = %e, "Failed to evaluate session");
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_transcript() {
        let messages = vec![
            DialogMessage {
                direction: MessageDirection::Outgoing,
                message_text: "Привет!".to_string(),
            },
            DialogMessage {
                direction: MessageDirection::Incoming,
                message_text: "Здравствуйте".to_string(),
            },
        ];

        let transcript = DialogEvaluator::build_transcript(&messages);
        assert!(transcript.contains("Бот: Привет!"));
        assert!(transcript.contains("Клиент: Здравствуйте"));
    }

    #[test]
    fn test_message_direction_from_outgoing() {
        let dir = MessageDirection::from("outgoing");
        assert!(matches!(dir, MessageDirection::Outgoing));
    }

    #[test]
    fn test_message_direction_from_outgoing_uppercase() {
        let dir = MessageDirection::from("OUTGOING");
        assert!(matches!(dir, MessageDirection::Outgoing));
    }

    #[test]
    fn test_message_direction_from_incoming() {
        let dir = MessageDirection::from("incoming");
        assert!(matches!(dir, MessageDirection::Incoming));
    }

    #[test]
    fn test_message_direction_from_unknown() {
        let dir = MessageDirection::from("unknown");
        assert!(matches!(dir, MessageDirection::Incoming)); // defaults to Incoming
    }

    #[test]
    fn test_message_direction_from_empty() {
        let dir = MessageDirection::from("");
        assert!(matches!(dir, MessageDirection::Incoming));
    }

    #[test]
    fn test_dialog_message_creation() {
        let msg = DialogMessage {
            direction: MessageDirection::Outgoing,
            message_text: "Test message".to_string(),
        };

        assert_eq!(msg.message_text, "Test message");
        assert!(matches!(msg.direction, MessageDirection::Outgoing));
    }

    #[test]
    fn test_dialog_message_clone() {
        let msg = DialogMessage {
            direction: MessageDirection::Incoming,
            message_text: "Original".to_string(),
        };

        let cloned = msg.clone();
        assert_eq!(cloned.message_text, "Original");
    }

    #[test]
    fn test_evaluation_result_creation() {
        let result = EvaluationResult {
            session_id: 12345,
            message_count: 10,
            evaluation: "Good performance".to_string(),
        };

        assert_eq!(result.session_id, 12345);
        assert_eq!(result.message_count, 10);
        assert_eq!(result.evaluation, "Good performance");
    }

    #[test]
    fn test_evaluation_result_clone() {
        let result = EvaluationResult {
            session_id: 100,
            message_count: 5,
            evaluation: "Test".to_string(),
        };

        let cloned = result.clone();
        assert_eq!(result.session_id, cloned.session_id);
        assert_eq!(result.evaluation, cloned.evaluation);
    }

    #[test]
    fn test_build_transcript_empty() {
        let messages: Vec<DialogMessage> = vec![];
        let transcript = DialogEvaluator::build_transcript(&messages);
        assert!(transcript.is_empty());
    }

    #[test]
    fn test_build_transcript_single_message() {
        let messages = vec![DialogMessage {
            direction: MessageDirection::Incoming,
            message_text: "Hello".to_string(),
        }];

        let transcript = DialogEvaluator::build_transcript(&messages);
        assert_eq!(transcript, "Клиент: Hello");
    }

    #[test]
    fn test_dialog_message_serialize() {
        let msg = DialogMessage {
            direction: MessageDirection::Outgoing,
            message_text: "Test".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"message_text\":\"Test\""));
    }

    #[test]
    fn test_dialog_message_deserialize() {
        let json = r#"{"direction":"Outgoing","message_text":"Hello"}"#;
        let msg: DialogMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.message_text, "Hello");
        assert!(matches!(msg.direction, MessageDirection::Outgoing));
    }

    #[test]
    fn test_evaluation_result_serialize() {
        let result = EvaluationResult {
            session_id: 42,
            message_count: 3,
            evaluation: "Good".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"session_id\":42"));
        assert!(json.contains("\"message_count\":3"));
    }

    #[test]
    fn test_evaluation_result_deserialize() {
        let json = r#"{"session_id":100,"message_count":5,"evaluation":"Excellent"}"#;
        let result: EvaluationResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.session_id, 100);
        assert_eq!(result.message_count, 5);
        assert_eq!(result.evaluation, "Excellent");
    }

    #[test]
    fn test_message_direction_serialize_incoming() {
        let dir = MessageDirection::Incoming;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"Incoming\"");
    }

    #[test]
    fn test_message_direction_serialize_outgoing() {
        let dir = MessageDirection::Outgoing;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"Outgoing\"");
    }

    #[test]
    fn test_build_transcript_multiline_message() {
        let messages = vec![DialogMessage {
            direction: MessageDirection::Outgoing,
            message_text: "Line 1\nLine 2".to_string(),
        }];

        let transcript = DialogEvaluator::build_transcript(&messages);
        assert!(transcript.contains("Line 1\nLine 2"));
    }
}
