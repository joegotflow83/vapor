use aws_config::SdkConfig;
use aws_sdk_appconfig::types::{Application, ConfigurationProfileSummary, Environment};

use crate::error::VaporError;

pub struct AppConfigClient {
    inner: aws_sdk_appconfig::Client,
}

impl AppConfigClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_appconfig::Client::new(config),
        }
    }

    /// Lists AppConfig applications, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListApplications` has both
    /// `max_results` and `next_token` (verified against pinned `aws-sdk-appconfig`
    /// 1.107.0's `operation/list_applications/_list_applications_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_applications(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Application>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_applications();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists AppConfig environments for an application, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`. `ListEnvironments`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-appconfig` 1.107.0's
    /// `operation/list_environments/_list_environments_input.rs`), same pattern as
    /// `list_applications` above.
    pub async fn list_environments(
        &self,
        application_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Environment>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_environments()
                .application_id(application_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists AppConfig configuration profiles for an application, optionally capped
    /// at `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListConfigurationProfiles` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-appconfig` 1.107.0's
    /// `operation/list_configuration_profiles/_list_configuration_profiles_input.rs`),
    /// same pattern as `list_applications` above.
    pub async fn list_configuration_profiles(
        &self,
        application_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ConfigurationProfileSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_configuration_profiles()
                .application_id(application_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const APPS: &str = "https://appconfig.us-east-1.amazonaws.com/applications";

    #[tokio::test]
    async fn list_applications_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APPS, ""),
            json_response(
                200,
                r#"{"Items":[{"Id":"app1","Name":"one"},{"Id":"app2","Name":"two"}]}"#,
            ),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client.list_applications(None, None).await.unwrap();

        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id(), Some("app1"));
        assert_eq!(apps[1].name(), Some("two"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APPS}?next_token=cursor-a"), ""),
            json_response(200, r#"{"Items":[{"Id":"app3","Name":"three"}]}"#),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client
            .list_applications(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(apps.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APPS}?max_results=2"), ""),
            json_response(
                200,
                r#"{"Items":[{"Id":"a1","Name":"one"},{"Id":"a2","Name":"two"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client.list_applications(Some(2), None).await.unwrap();

        assert_eq!(apps.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{APPS}?max_results=10"), ""),
                json_response(
                    200,
                    r#"{"Items":[{"Id":"a1","Name":"one"},{"Id":"a2","Name":"two"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{APPS}?max_results=8&next_token=p2"), ""),
                json_response(200, r#"{"Items":[{"Id":"a3","Name":"three"}]}"#),
            ),
        ]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (apps, token) = client.list_applications(Some(10), None).await.unwrap();

        assert_eq!(apps.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_applications_propagates_errors() {
        // `BadRequestException` (not `TooManyRequestsException` or another
        // AWS-modeled throttling exception) — the SDK's default retry
        // strategy classifies throttling exceptions as retryable and
        // consumes a second replay event this single-event client doesn't
        // have (see apigateway.rs's precedent for this pitfall).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APPS, ""),
            json_error_response("BadRequestException", "invalid request"),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let err = client.list_applications(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_environments_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://appconfig.us-east-1.amazonaws.com/applications/app1/environments",
                "",
            ),
            json_response(
                200,
                r#"{"Items":[{"Id":"env1","Name":"prod"},{"Id":"env2","Name":"dev"}]}"#,
            ),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client.list_environments("app1", None, None).await.unwrap();

        assert_eq!(envs.len(), 2);
        assert_eq!(envs[0].id(), Some("env1"));
        assert_eq!(envs[1].name(), Some("dev"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_environments_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://appconfig.us-east-1.amazonaws.com/applications/app1/environments?max_results=1",
                "",
            ),
            json_response(
                200,
                r#"{"Items":[{"Id":"env1","Name":"prod"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client
            .list_environments("app1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(envs.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configuration_profiles_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://appconfig.us-east-1.amazonaws.com/applications/app1/configurationprofiles",
                "",
            ),
            json_response(
                200,
                r#"{"Items":[{"Id":"cp1","Name":"profile1"},{"Id":"cp2","Name":"profile2"}]}"#,
            ),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (profiles, token) = client
            .list_configuration_profiles("app1", None, None)
            .await
            .unwrap();

        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].id(), Some("cp1"));
        assert_eq!(profiles[1].name(), Some("profile2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_configuration_profiles_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://appconfig.us-east-1.amazonaws.com/applications/app1/configurationprofiles?max_results=1",
                "",
            ),
            json_response(
                200,
                r#"{"Items":[{"Id":"cp1","Name":"profile1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = AppConfigClient::new(&sdk_config(http_client.clone()));

        let (profiles, token) = client
            .list_configuration_profiles("app1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(profiles.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }
}
