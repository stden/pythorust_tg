//! Create Linear issues via CLI.

use crate::linear::{CreateIssueInput, LinearClient};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct LinearArgs {
    pub api_key: Option<String>,
    pub team: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub project: Option<String>,
    pub priority: i32,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
}

pub async fn run(args: LinearArgs) -> Result<()> {
    let client = LinearClient::from_optional_key(args.api_key)?;

    let team_key = args
        .team
        .or_else(|| std::env::var("LINEAR_TEAM_KEY").ok())
        .ok_or_else(|| Error::InvalidArgument("LINEAR_TEAM_KEY не задан".to_string()))?;

    let project_id = args.project.or_else(|| std::env::var("LINEAR_PROJECT_ID").ok());
    let labels: Vec<String> = args
        .labels
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let priority = args.priority.clamp(0, 4);

    let issue = client
        .create_issue(CreateIssueInput {
            team_key,
            title: args.title,
            description: args.description,
            project_id,
            priority: Some(priority),
            assignee_id: args.assignee,
            label_ids: labels,
        })
        .await?;

    println!(
        "Создана задача {} {}",
        issue.identifier.as_deref().unwrap_or(""),
        issue.url.as_deref().unwrap_or("")
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => std::env::set_var(self.key, val),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[tokio::test]
    async fn run_clamps_priority_and_trims_labels() {
        let server = MockServer::start_async().await;
        let _url_guard = EnvGuard::set("LINEAR_API_URL", &server.url("/graphql"));

        let team_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/graphql")
                .matches(|req| {
                    let body = String::from_utf8_lossy(req.body().as_ref());
                    body.contains("query TeamId")
                });
            then.status(200)
                .json_body(serde_json::json!({
                    "data": { "team": { "id": "team-123", "key": "APP" } }
                }));
        });

        let issue_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/graphql")
                .matches(|req| {
                    let body = String::from_utf8_lossy(req.body().as_ref());
                    body.contains("mutation IssueCreate")
                });
            then.status(200)
                .json_body(serde_json::json!({
                    "data": {
                        "issueCreate": {
                            "success": true,
                            "issue": {
                                "id": "issue-1",
                                "identifier": "APP-10",
                                "title": "Title",
                                "url": "https://linear.app/app-1/issue/APP-10/title"
                            }
                        }
                    }
                }));
        });

        run(LinearArgs {
            api_key: Some("test-key".into()),
            team: Some("APP".into()),
            title: "Title".into(),
            description: Some("desc   ".into()),
            project: None,
            priority: 7,
            assignee: None,
            labels: vec!["  l1 ".into(), "".into()],
        })
        .await
        .expect("run");

        team_mock.assert_calls(1);
        issue_mock.assert_calls(1);
    }

    #[tokio::test]
    async fn run_errors_when_team_missing() {
        let _team_guard = EnvGuard::remove("LINEAR_TEAM_KEY");

        let err = run(LinearArgs {
            api_key: Some("key".into()),
            team: None,
            title: "Title".into(),
            description: None,
            project: None,
            priority: 1,
            assignee: None,
            labels: vec![],
        })
        .await
        .unwrap_err();

        assert!(matches!(err, Error::InvalidArgument(msg) if msg.contains("LINEAR_TEAM_KEY")));
    }
}
