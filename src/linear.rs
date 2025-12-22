//! Minimal Linear GraphQL client (issue creation).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

#[derive(Debug, Clone)]
pub struct LinearClient {
    http: Client,
    api_key: String,
    base_url: String,
    team_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl LinearClient {
    /// Create client from explicit or environment API key.
    pub fn from_optional_key(api_key: Option<String>) -> Result<Self> {
        let key = api_key
            .or_else(|| std::env::var("LINEAR_API_KEY").ok())
            .ok_or_else(|| Error::InvalidArgument("LINEAR_API_KEY не задан".to_string()))?;

        Self::new(key)
    }

    /// Create client with provided API key.
    pub fn new<S: Into<String>>(api_key: S) -> Result<Self> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(Error::InvalidArgument("LINEAR_API_KEY пустой".to_string()));
        }

        let http = Client::builder()
            .user_agent(format!("telegram_reader/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| Error::LinearError(format!("Не удалось построить HTTP-клиент: {}", e)))?;

        Ok(Self {
            http,
            api_key,
            base_url: std::env::var("LINEAR_API_URL")
                .unwrap_or_else(|_| LINEAR_API_URL.to_string()),
            team_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create client with custom base url (primarily for tests).
    pub fn with_base_url<S1: Into<String>, S2: Into<String>>(
        api_key: S1,
        base_url: S2,
    ) -> Result<Self> {
        let mut client = Self::new(api_key)?;
        client.base_url = base_url.into();
        Ok(client)
    }

    async fn post<V: Serialize, D: for<'de> Deserialize<'de>>(
        &self,
        query: &'static str,
        variables: V,
    ) -> Result<D> {
        let response = self
            .http
            .post(&self.base_url)
            .header("Authorization", &self.api_key)
            .json(&GraphQlRequest { query, variables })
            .send()
            .await
            .map_err(|e| Error::LinearError(format!("Не удалось обратиться к Linear: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Error::LinearError(format!("Не удалось прочитать ответ Linear: {}", e)))?;

        if status != StatusCode::OK {
            return Err(Error::LinearError(format!(
                "Linear вернул HTTP {}: {}",
                status.as_u16(),
                text
            )));
        }

        let envelope: GraphQlResponse<D> = serde_json::from_str(&text).map_err(|e| {
            Error::LinearError(format!("Linear вернул не-JSON ответ: {} ({})", text, e))
        })?;

        if let Some(errors) = envelope.errors {
            let message = errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Error::LinearError(message));
        }

        envelope
            .data
            .ok_or_else(|| Error::LinearError("Пустой ответ от Linear".to_string()))
    }

    /// Resolve team ID by key, cached in-memory.
    pub async fn get_team_id(&self, team_key: &str) -> Result<String> {
        if let Some(cached) = self
            .team_cache
            .lock()
            .map_err(|_| {
                Error::LinearError("Не удалось получить доступ к кешу команд".to_string())
            })?
            .get(team_key)
            .cloned()
        {
            return Ok(cached);
        }

        let data: TeamResponse = self
            .post(TEAM_QUERY, TeamVariables { key: team_key })
            .await?;
        let team = data.team.ok_or_else(|| {
            Error::LinearError(format!("Команда '{}' не найдена в Linear", team_key))
        })?;

        self.team_cache
            .lock()
            .map_err(|_| Error::LinearError("Не удалось обновить кеш команд".to_string()))?
            .insert(team_key.to_string(), team.id.clone());

        Ok(team.id)
    }

    /// Create a Linear issue.
    pub async fn create_issue(&self, input: CreateIssueInput) -> Result<CreatedIssue> {
        if input.title.trim().is_empty() {
            return Err(Error::InvalidArgument(
                "Заголовок задачи пустой".to_string(),
            ));
        }

        let team_id = self.get_team_id(&input.team_key).await?;

        let payload = IssueCreateVariables {
            input: IssueCreateInputPayload {
                team_id,
                title: input.title.trim().to_string(),
                description: input
                    .description
                    .as_ref()
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty()),
                project_id: input.project_id,
                priority: input.priority,
                assignee_id: input.assignee_id,
                label_ids: if input.label_ids.is_empty() {
                    None
                } else {
                    Some(input.label_ids)
                },
            },
        };

        let data: IssueCreateResponse = self.post(ISSUE_CREATE_MUTATION, payload).await?;
        let issue = data
            .issue_create
            .and_then(|ic| if ic.success { ic.issue } else { None })
            .ok_or_else(|| {
                Error::LinearError("Linear не подтвердил создание задачи".to_string())
            })?;

        Ok(issue)
    }
}

#[derive(Debug, Clone)]
pub struct CreateIssueInput {
    pub team_key: String,
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub priority: Option<i32>,
    pub assignee_id: Option<String>,
    pub label_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatedIssue {
    pub id: String,
    pub identifier: Option<String>,
    pub title: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
struct GraphQlRequest<V> {
    query: &'static str,
    variables: V,
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse<D> {
    data: Option<D>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Serialize)]
struct TeamVariables<'a> {
    key: &'a str,
}

#[derive(Debug, Deserialize)]
struct TeamResponse {
    team: Option<LinearTeam>,
}

#[derive(Debug, Deserialize)]
struct LinearTeam {
    id: String,
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    key: String,
}

#[derive(Debug, Serialize)]
struct IssueCreateVariables {
    input: IssueCreateInputPayload,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IssueCreateInputPayload {
    team_id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct IssueCreateResponse {
    #[serde(rename = "issueCreate")]
    issue_create: Option<IssueCreateResult>,
}

#[derive(Debug, Deserialize)]
struct IssueCreateResult {
    success: bool,
    issue: Option<CreatedIssue>,
}

const TEAM_QUERY: &str = r#"
query TeamId($key: String!) {
  team: teamForKey(key: $key) {
    id
    name
    key
  }
}
"#;

const ISSUE_CREATE_MUTATION: &str = r#"
mutation IssueCreate($input: IssueCreateInput!) {
  issueCreate(input: $input) {
    success
    issue {
      id
      identifier
      title
      url
    }
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use std::env;

    fn setup_client(server: &MockServer) -> LinearClient {
        LinearClient::with_base_url("test-key", server.url("/graphql")).expect("client")
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = env::var(key).ok();
            env::set_var(key, value);
            Self { key, original }
        }

        fn clear(key: &'static str) -> Self {
            let original = env::var(key).ok();
            env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => env::set_var(self.key, val),
                None => env::remove_var(self.key),
            }
        }
    }

    #[tokio::test]
    async fn get_team_id_cached_and_fetched_once() {
        let server = MockServer::start_async().await;

        let team_mock = server.mock(|when, then| {
            when.method(POST).path("/graphql").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("query TeamId")
            });
            then.status(200).json_body(serde_json::json!({
                "data": {
                    "team": {
                        "id": "team-123",
                        "name": "App",
                        "key": "APP"
                    }
                }
            }));
        });

        let client = setup_client(&server);

        let first = client.get_team_id("APP").await.expect("first team");
        let second = client.get_team_id("APP").await.expect("cached team");

        assert_eq!(first, "team-123");
        assert_eq!(second, "team-123");
        team_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn get_team_id_surfaces_http_error_status() {
        let server = MockServer::start_async().await;

        let team_mock = server.mock(|when, then| {
            when.method(POST).path("/graphql");
            then.status(500).body("boom");
        });

        let client = setup_client(&server);
        let err = client.get_team_id("APP").await.unwrap_err();

        let msg = format!("{err}");
        assert!(msg.contains("HTTP 500"));
        assert!(msg.contains("boom"));
        team_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn get_team_id_rejects_non_json_body() {
        let server = MockServer::start_async().await;

        let team_mock = server.mock(|when, then| {
            when.method(POST).path("/graphql");
            then.status(200).body("not-json");
        });

        let client = setup_client(&server);
        let err = client.get_team_id("APP").await.unwrap_err();

        assert!(format!("{err}").contains("не-JSON ответ"));
        team_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn create_issue_success() {
        let server = MockServer::start_async().await;

        let team_mock = server.mock(|when, then| {
            when.method(POST).path("/graphql").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("query TeamId")
            });
            then.status(200).json_body(serde_json::json!({
                "data": { "team": { "id": "team-123", "key": "APP" } }
            }));
        });

        let issue_mock = server.mock(|when, then| {
            when.method(POST).path("/graphql").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("mutation IssueCreate")
            });
            then.status(200).json_body(serde_json::json!({
                "data": {
                    "issueCreate": {
                        "success": true,
                        "issue": {
                            "id": "issue-1",
                            "identifier": "APP-101",
                            "title": "Title",
                            "url": "https://linear.app/app-1/issue/APP-101/title"
                        }
                    }
                }
            }));
        });

        let client = setup_client(&server);
        let issue = client
            .create_issue(CreateIssueInput {
                team_key: "APP".into(),
                title: "Title".into(),
                description: Some("Desc".into()),
                project_id: Some("proj".into()),
                priority: Some(3),
                assignee_id: Some("assignee".into()),
                label_ids: vec!["l1".into(), "l2".into()],
            })
            .await
            .expect("issue");

        assert_eq!(issue.identifier.as_deref(), Some("APP-101"));
        assert_eq!(
            issue.url.as_deref(),
            Some("https://linear.app/app-1/issue/APP-101/title")
        );
        team_mock.assert_calls(1);
        issue_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn create_issue_propagates_linear_error() {
        let server = MockServer::start_async().await;

        server.mock(|when, then| {
            when.method(POST).path("/graphql").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("query TeamId")
            });
            then.status(200).json_body(
                serde_json::json!({"data": { "team": { "id": "team-123", "key": "APP" } }}),
            );
        });

        let issue_mock = server.mock(|when, then| {
            when.method(POST).path("/graphql").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("mutation IssueCreate")
            });
            then.status(200).json_body(serde_json::json!({
                "errors": [ { "message": "project not found" } ]
            }));
        });

        let client = setup_client(&server);
        let err = client
            .create_issue(CreateIssueInput {
                team_key: "APP".into(),
                title: "Broken".into(),
                description: None,
                project_id: None,
                priority: None,
                assignee_id: None,
                label_ids: vec![],
            })
            .await
            .unwrap_err();

        let msg = format!("{err}");
        assert!(msg.contains("project not found"));
        issue_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn create_issue_omits_empty_fields_and_trims_title() {
        let server = MockServer::start_async().await;

        let team_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/graphql")
                .json_body(serde_json::json!({
                    "query": TEAM_QUERY,
                    "variables": { "key": "APP" }
                }));
            then.status(200).json_body(serde_json::json!({
                "data": {
                    "team": {
                        "id": "team-123",
                        "name": "App",
                        "key": "APP"
                    }
                }
            }));
        });

        let issue_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/graphql")
                .json_body(serde_json::json!({
                    "query": ISSUE_CREATE_MUTATION,
                    "variables": {
                        "input": {
                            "teamId": "team-123",
                            "title": "Trimmed"
                        }
                    }
                }));
            then.status(200).json_body(serde_json::json!({
                "data": {
                    "issueCreate": {
                        "success": true,
                        "issue": {
                            "id": "issue-9",
                            "identifier": "APP-9",
                            "title": "Trimmed",
                            "url": null
                        }
                    }
                }
            }));
        });

        let client = setup_client(&server);
        let issue = client
            .create_issue(CreateIssueInput {
                team_key: "APP".into(),
                title: "  Trimmed  ".into(),
                description: Some("   ".into()),
                project_id: None,
                priority: None,
                assignee_id: None,
                label_ids: vec![],
            })
            .await
            .expect("issue");

        assert_eq!(issue.title, "Trimmed");
        team_mock.assert_calls(1);
        issue_mock.assert_calls(1);
    }

    #[test]
    fn from_optional_key_uses_env_when_missing_argument() {
        let _guard = EnvGuard::set("LINEAR_API_KEY", "env-key");

        let client = LinearClient::from_optional_key(None).expect("client from env var");

        assert_eq!(client.api_key, "env-key");
        assert_eq!(client.base_url, LINEAR_API_URL);
    }

    #[test]
    fn from_optional_key_fails_without_argument_or_env() {
        let _guard = EnvGuard::clear("LINEAR_API_KEY");

        let err = LinearClient::from_optional_key(None).unwrap_err();

        assert!(format!("{err}").contains("LINEAR_API_KEY не задан"));
    }

    #[test]
    fn new_rejects_empty_key() {
        let err = LinearClient::new("   ").unwrap_err();
        assert!(format!("{err}").contains("LINEAR_API_KEY пустой"));
    }

    #[tokio::test]
    async fn create_issue_rejects_empty_title() {
        let client = LinearClient::new("key").expect("client");
        let err = client
            .create_issue(CreateIssueInput {
                team_key: "APP".into(),
                title: "   ".into(),
                description: None,
                project_id: None,
                priority: None,
                assignee_id: None,
                label_ids: vec![],
            })
            .await
            .unwrap_err();

        assert!(format!("{err}").contains("Заголовок задачи пустой"));
    }

    #[tokio::test]
    async fn get_team_id_returns_error_when_team_missing() {
        let server = MockServer::start_async().await;

        let team_mock = server.mock(|when, then| {
            when.method(POST).path("/graphql").is_true(|req| {
                let body = String::from_utf8_lossy(req.body().as_ref());
                body.contains("query TeamId")
            });
            then.status(200).json_body(serde_json::json!({
                "data": {
                    "team": null
                }
            }));
        });

        let client = setup_client(&server);
        let err = client.get_team_id("MISS").await.unwrap_err();

        assert!(format!("{err}").contains("не найдена"));
        team_mock.assert_calls(1);
    }

    #[test]
    fn from_optional_key_prefers_argument_over_env() {
        let _guard = EnvGuard::set("LINEAR_API_KEY", "env-key");

        let client = LinearClient::from_optional_key(Some("arg-key".into())).expect("client");

        assert_eq!(client.api_key, "arg-key");
    }
}
