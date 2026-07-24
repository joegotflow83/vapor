use aws_config::SdkConfig;
use aws_sdk_codedeploy::types::{ApplicationInfo, DeploymentGroupInfo, DeploymentInfo};

use crate::aws::pagination::apply_limit;
use crate::error::VaporError;

pub struct CodeDeployClient {
    inner: aws_sdk_codedeploy::Client,
}

impl CodeDeployClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codedeploy::Client::new(config),
        }
    }

    /// Lists CodeDeploy application names, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListApplications`
    /// has no `max_results`-equivalent input field at all (verified against
    /// pinned `aws-sdk-codedeploy` 1.105.0's `_list_applications_input.rs` —
    /// only `next_token`) — same caveat class as `polly.rs::describe_voices`/
    /// `cost_explorer.rs::get_cost_and_usage`: `limit` can only be enforced via
    /// client-side `apply_limit` truncation, so when that trips mid-page the
    /// returned `next_token` is still AWS's *next*-page token, permanently
    /// skipping whatever was truncated off the current page.
    pub async fn list_applications(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_applications();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.applications.unwrap_or_default());

            token = match output.next_token {
                Some(t) if !t.is_empty() => Some(t),
                _ => None,
            };

            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn batch_get_applications(
        &self,
        names: Vec<String>,
    ) -> Result<Vec<ApplicationInfo>, VaporError> {
        let mut all = Vec::new();
        for chunk in names.chunks(100) {
            let output = self
                .inner
                .batch_get_applications()
                .set_application_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            all.extend(output.applications_info().to_vec());
        }
        Ok(all)
    }

    /// Lists deployment group names for an application, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListDeploymentGroups` has no `max_results`-equivalent input field
    /// (same caveat class as `list_applications` above).
    pub async fn list_deployment_groups(
        &self,
        app_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_deployment_groups()
                .application_name(app_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.deployment_groups.unwrap_or_default());

            token = match output.next_token {
                Some(t) if !t.is_empty() => Some(t),
                _ => None,
            };

            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn batch_get_deployment_groups(
        &self,
        app_name: &str,
        group_names: Vec<String>,
    ) -> Result<Vec<DeploymentGroupInfo>, VaporError> {
        let mut all = Vec::new();
        for chunk in group_names.chunks(100) {
            let output = self
                .inner
                .batch_get_deployment_groups()
                .application_name(app_name)
                .set_deployment_group_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            all.extend(output.deployment_groups_info().to_vec());
        }
        Ok(all)
    }

    /// Lists deployment IDs, optionally filtered by application/deployment group,
    /// capped at `limit` results (default unlimited), and resumed from
    /// `next_token`. `ListDeployments` has no `max_results`-equivalent input
    /// field (same caveat class as `list_applications` above).
    pub async fn list_deployments(
        &self,
        app_name: Option<&str>,
        group_name: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_deployments();
            if let Some(app) = app_name {
                req = req.application_name(app);
            }
            if let Some(group) = group_name {
                req = req.deployment_group_name(group);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.deployments.unwrap_or_default());

            token = match output.next_token {
                Some(t) if !t.is_empty() => Some(t),
                _ => None,
            };

            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn batch_get_deployments(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<DeploymentInfo>, VaporError> {
        let mut all = Vec::new();
        for chunk in ids.chunks(100) {
            let output = self
                .inner
                .batch_get_deployments()
                .set_deployment_ids(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            all.extend(output.deployments_info().to_vec());
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

    const ENDPOINT: &str = "https://codedeploy.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_applications_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"applications":["app-1","app-2"]}"#),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client.list_applications(None, None).await.unwrap();

        assert_eq!(apps, vec!["app-1".to_string(), "app-2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"applications":["app-3"]}"#),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client
            .list_applications(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(apps, vec!["app-3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_stops_at_limit_and_returns_resume_token() {
        // `ListApplicationsInput` has no `maxResults`-equivalent field, so
        // `limit` is enforced entirely via client-side `apply_limit`
        // truncation after the full page is fetched.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"applications":["app-1","app-2","app-3"],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client.list_applications(Some(2), None).await.unwrap();

        assert_eq!(apps.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"applications":["app-1"],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"p2"}"#),
                json_response(200, r#"{"applications":["app-2"]}"#),
            ),
        ]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client.list_applications(Some(10), None).await.unwrap();

        assert_eq!(apps.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidNextTokenException", "bad next token"),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let err = client.list_applications(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidNextTokenException".to_string()));
                assert_eq!(message, "bad next token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_applications_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"applicationNames":["app-1","app-2"]}"#),
            json_response(
                200,
                r#"{"applicationsInfo":[{"applicationName":"app-1","applicationId":"1234abcd"},{"applicationName":"app-2"}]}"#,
            ),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let apps = client
            .batch_get_applications(vec!["app-1".to_string(), "app-2".to_string()])
            .await
            .unwrap();

        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].application_name(), Some("app-1"));
        assert_eq!(apps[0].application_id(), Some("1234abcd"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_applications_chunks_names_over_100() {
        let first_chunk: Vec<String> = (0..100).map(|i| format!("app-{i}")).collect();
        let second_chunk: Vec<String> = (100..150).map(|i| format!("app-{i}")).collect();
        let mut all_names = first_chunk.clone();
        all_names.extend(second_chunk.clone());

        let first_body = format!(
            r#"{{"applicationNames":[{}]}}"#,
            first_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let second_body = format!(
            r#"{{"applicationNames":[{}]}}"#,
            second_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, first_body),
                json_response(200, r#"{"applicationsInfo":[{"applicationName":"app-0"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, second_body),
                json_response(
                    200,
                    r#"{"applicationsInfo":[{"applicationName":"app-100"}]}"#,
                ),
            ),
        ]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let apps = client.batch_get_applications(all_names).await.unwrap();

        assert_eq!(apps.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_applications_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"applicationNames":["app-1"]}"#),
            json_error_response("BatchLimitExceededException", "too many names"),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let err = client
            .batch_get_applications(vec!["app-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BatchLimitExceededException".to_string()));
                assert_eq!(message, "too many names");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployment_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"applicationName":"my-app"}"#),
            json_response(200, r#"{"deploymentGroups":["group-1","group-2"]}"#),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_deployment_groups("my-app", None, None)
            .await
            .unwrap();

        assert_eq!(groups, vec!["group-1".to_string(), "group-2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployment_groups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"applicationName":"my-app"}"#),
            json_response(
                200,
                r#"{"deploymentGroups":["group-1","group-2"],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_deployment_groups("my-app", Some(1), None)
            .await
            .unwrap();

        assert_eq!(groups, vec!["group-1".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployment_groups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"applicationName":"my-app","nextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"deploymentGroups":["group-3"]}"#),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_deployment_groups("my-app", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(groups, vec!["group-3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployment_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"applicationName":"my-app"}"#),
            json_error_response("ApplicationDoesNotExistException", "no such application"),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_deployment_groups("my-app", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ApplicationDoesNotExistException".to_string()));
                assert_eq!(message, "no such application");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_deployment_groups_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"applicationName":"my-app","deploymentGroupNames":["group-1"]}"#,
            ),
            json_response(
                200,
                r#"{"deploymentGroupsInfo":[{"applicationName":"my-app","deploymentGroupName":"group-1","deploymentGroupId":"abcd1234"}]}"#,
            ),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let groups = client
            .batch_get_deployment_groups("my-app", vec!["group-1".to_string()])
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].deployment_group_name(), Some("group-1"));
        assert_eq!(groups[0].deployment_group_id(), Some("abcd1234"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_deployment_groups_chunks_names_over_100() {
        let first_chunk: Vec<String> = (0..100).map(|i| format!("group-{i}")).collect();
        let second_chunk: Vec<String> = (100..150).map(|i| format!("group-{i}")).collect();
        let mut all_names = first_chunk.clone();
        all_names.extend(second_chunk.clone());

        let first_body = format!(
            r#"{{"applicationName":"my-app","deploymentGroupNames":[{}]}}"#,
            first_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let second_body = format!(
            r#"{{"applicationName":"my-app","deploymentGroupNames":[{}]}}"#,
            second_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, first_body),
                json_response(
                    200,
                    r#"{"deploymentGroupsInfo":[{"deploymentGroupName":"group-0"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, second_body),
                json_response(
                    200,
                    r#"{"deploymentGroupsInfo":[{"deploymentGroupName":"group-100"}]}"#,
                ),
            ),
        ]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let groups = client
            .batch_get_deployment_groups("my-app", all_names)
            .await
            .unwrap();

        assert_eq!(groups.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_deployment_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"applicationName":"my-app","deploymentGroupNames":["group-1"]}"#,
            ),
            json_error_response("BatchLimitExceededException", "too many names"),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let err = client
            .batch_get_deployment_groups("my-app", vec!["group-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BatchLimitExceededException".to_string()));
                assert_eq!(message, "too many names");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployments_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"deployments":["d-1","d-2"]}"#),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_deployments(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["d-1".to_string(), "d-2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployments_filters_by_application_and_group() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"applicationName":"my-app","deploymentGroupName":"my-group"}"#,
            ),
            json_response(200, r#"{"deployments":["d-1"]}"#),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_deployments(Some("my-app"), Some("my-group"), None, None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["d-1".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployments_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"deployments":["d-1","d-2"],"nextToken":"page2"}"#),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_deployments(None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["d-1".to_string()]);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployments_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"deployments":["d-1"],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"p2"}"#),
                json_response(200, r#"{"deployments":["d-2"]}"#),
            ),
        ]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_deployments(None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(ids.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_deployments_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidNextTokenException", "bad next token"),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_deployments(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidNextTokenException".to_string()));
                assert_eq!(message, "bad next token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_deployments_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"deploymentIds":["d-1"]}"#),
            json_response(
                200,
                r#"{"deploymentsInfo":[{"deploymentId":"d-1","applicationName":"my-app","deploymentGroupName":"my-group"}]}"#,
            ),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let deployments = client
            .batch_get_deployments(vec!["d-1".to_string()])
            .await
            .unwrap();

        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments[0].deployment_id(), Some("d-1"));
        assert_eq!(deployments[0].application_name(), Some("my-app"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_deployments_chunks_ids_over_100() {
        let first_chunk: Vec<String> = (0..100).map(|i| format!("d-{i}")).collect();
        let second_chunk: Vec<String> = (100..150).map(|i| format!("d-{i}")).collect();
        let mut all_ids = first_chunk.clone();
        all_ids.extend(second_chunk.clone());

        let first_body = format!(
            r#"{{"deploymentIds":[{}]}}"#,
            first_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let second_body = format!(
            r#"{{"deploymentIds":[{}]}}"#,
            second_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, first_body),
                json_response(200, r#"{"deploymentsInfo":[{"deploymentId":"d-0"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, second_body),
                json_response(200, r#"{"deploymentsInfo":[{"deploymentId":"d-100"}]}"#),
            ),
        ]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let deployments = client.batch_get_deployments(all_ids).await.unwrap();

        assert_eq!(deployments.len(), 2);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_deployments_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"deploymentIds":["d-1"]}"#),
            json_error_response("BatchLimitExceededException", "too many ids"),
        )]);
        let client = CodeDeployClient::new(&sdk_config(http_client.clone()));

        let err = client
            .batch_get_deployments(vec!["d-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BatchLimitExceededException".to_string()));
                assert_eq!(message, "too many ids");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
