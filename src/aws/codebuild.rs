use aws_config::SdkConfig;
use aws_sdk_codebuild::types::{Build, Project};

use crate::error::VaporError;
use crate::aws::pagination::apply_limit;

pub struct CodeBuildClient {
    inner: aws_sdk_codebuild::Client,
}

impl CodeBuildClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codebuild::Client::new(config),
        }
    }

    /// Lists project names, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListProjectsInput` has no
    /// `max_results`-equivalent field at all (verified:
    /// `_list_projects_input.rs` only exposes `sort_by`/`sort_order`/
    /// `next_token` — each page returns up to 100 names with no way to
    /// request fewer) — same caveat class as `polly.rs::describe_voices`/
    /// `list_lexicons`: `limit` can only be enforced via client-side
    /// `apply_limit` truncation, so when that trips mid-page the returned
    /// `next_token` is still AWS's *next*-page token, permanently skipping
    /// whatever was truncated off the current page.
    pub async fn list_projects(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_projects();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.projects.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut items, limit) {
                break;
            }
            if token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn batch_get_projects(&self, names: Vec<String>) -> Result<Vec<Project>, VaporError> {
        let mut all = Vec::new();
        for chunk in names.chunks(100) {
            let output = self
                .inner
                .batch_get_projects()
                .set_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            all.extend(output.projects().to_vec());
        }
        Ok(all)
    }

    /// Lists build IDs for a project, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListBuildsForProjectInput` has no `max_results`-equivalent field
    /// either (only `project_name`/`sort_order`/`next_token`) — same
    /// no-size-hint caveat as `list_projects` above.
    pub async fn list_builds_for_project(
        &self,
        project_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_builds_for_project().project_name(project_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.ids.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut items, limit) {
                break;
            }
            if token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn batch_get_builds(&self, ids: Vec<String>) -> Result<Vec<Build>, VaporError> {
        let mut all = Vec::new();
        for chunk in ids.chunks(100) {
            let output = self
                .inner
                .batch_get_builds()
                .set_ids(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            all.extend(output.builds().to_vec());
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://codebuild.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_projects_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"projects":["proj-1","proj-2"]}"#),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let (projects, token) = client.list_projects(None, None).await.unwrap();

        assert_eq!(projects, vec!["proj-1".to_string(), "proj-2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_projects_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"projects":["proj-3"]}"#),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let (projects, token) = client
            .list_projects(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(projects, vec!["proj-3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_projects_stops_at_limit_and_returns_resume_token() {
        // `ListProjectsInput` has no `maxResults`-equivalent field, so
        // `limit` is enforced entirely via client-side `apply_limit`
        // truncation after the full page is fetched — the request body
        // never carries the limit.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"projects":["proj-1","proj-2","proj-3"],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let (projects, token) = client.list_projects(Some(2), None).await.unwrap();

        assert_eq!(projects.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_projects_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"projects":["proj-1"],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"p2"}"#),
                json_response(200, r#"{"projects":["proj-2"]}"#),
            ),
        ]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let (projects, token) = client.list_projects(Some(10), None).await.unwrap();

        assert_eq!(projects.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_projects_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "bad sort order"),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let err = client.list_projects(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "bad sort order");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_projects_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"names":["proj-1","proj-2"]}"#),
            json_response(
                200,
                r#"{"projects":[{"name":"proj-1","arn":"arn:aws:codebuild:us-east-1:111122223333:project/proj-1"},{"name":"proj-2"}],"projectsNotFound":[]}"#,
            ),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let projects = client
            .batch_get_projects(vec!["proj-1".to_string(), "proj-2".to_string()])
            .await
            .unwrap();

        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name(), Some("proj-1"));
        assert_eq!(
            projects[0].arn(),
            Some("arn:aws:codebuild:us-east-1:111122223333:project/proj-1")
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_projects_chunks_names_over_100() {
        let first_chunk: Vec<String> = (0..100).map(|i| format!("proj-{i}")).collect();
        let second_chunk: Vec<String> = (100..150).map(|i| format!("proj-{i}")).collect();
        let mut all_names = first_chunk.clone();
        all_names.extend(second_chunk.clone());

        let first_body = format!(
            r#"{{"names":[{}]}}"#,
            first_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let second_body = format!(
            r#"{{"names":[{}]}}"#,
            second_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, first_body),
                json_response(200, r#"{"projects":[{"name":"proj-0"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, second_body),
                json_response(200, r#"{"projects":[{"name":"proj-100"}]}"#),
            ),
        ]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let projects = client.batch_get_projects(all_names).await.unwrap();

        assert_eq!(projects.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_projects_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"names":["proj-1"]}"#),
            json_error_response("InvalidInputException", "too many names"),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let err = client
            .batch_get_projects(vec!["proj-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "too many names");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_builds_for_project_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"projectName":"my-project"}"#),
            json_response(200, r#"{"ids":["my-project:build-1","my-project:build-2"]}"#),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_builds_for_project("my-project", None, None)
            .await
            .unwrap();

        assert_eq!(
            ids,
            vec!["my-project:build-1".to_string(), "my-project:build-2".to_string()]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_builds_for_project_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"projectName":"my-project"}"#),
            json_response(
                200,
                r#"{"ids":["my-project:build-1","my-project:build-2"],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_builds_for_project("my-project", Some(1), None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["my-project:build-1".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_builds_for_project_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"projectName":"my-project","nextToken":"cursor-a"}"#),
            json_response(200, r#"{"ids":["my-project:build-3"]}"#),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_builds_for_project("my-project", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(ids, vec!["my-project:build-3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_builds_for_project_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"projectName":"my-project"}"#),
            json_error_response("InvalidInputException", "project not found"),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_builds_for_project("my-project", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "project not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_builds_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ids":["my-project:build-1"]}"#),
            json_response(
                200,
                r#"{"builds":[{"id":"my-project:build-1","projectName":"my-project","buildStatus":"SUCCEEDED"}],"buildsNotFound":[]}"#,
            ),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let builds = client
            .batch_get_builds(vec!["my-project:build-1".to_string()])
            .await
            .unwrap();

        assert_eq!(builds.len(), 1);
        assert_eq!(builds[0].id(), Some("my-project:build-1"));
        assert_eq!(builds[0].project_name(), Some("my-project"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_builds_chunks_ids_over_100() {
        let first_chunk: Vec<String> = (0..100).map(|i| format!("build-{i}")).collect();
        let second_chunk: Vec<String> = (100..150).map(|i| format!("build-{i}")).collect();
        let mut all_ids = first_chunk.clone();
        all_ids.extend(second_chunk.clone());

        let first_body = format!(
            r#"{{"ids":[{}]}}"#,
            first_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let second_body = format!(
            r#"{{"ids":[{}]}}"#,
            second_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, first_body),
                json_response(200, r#"{"builds":[{"id":"build-0"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, second_body),
                json_response(200, r#"{"builds":[{"id":"build-100"}]}"#),
            ),
        ]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let builds = client.batch_get_builds(all_ids).await.unwrap();

        assert_eq!(builds.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_builds_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ids":["my-project:build-1"]}"#),
            json_error_response("InvalidInputException", "too many ids"),
        )]);
        let client = CodeBuildClient::new(&sdk_config(http_client.clone()));

        let err = client
            .batch_get_builds(vec!["my-project:build-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidInputException".to_string()));
                assert_eq!(message, "too many ids");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

