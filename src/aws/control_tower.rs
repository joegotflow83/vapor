use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct ControlTowerClient {
    inner: aws_sdk_controltower::Client,
}

impl ControlTowerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_controltower::Client::new(config),
        }
    }

    /// Lists landing zones, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListLandingZones` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-controltower` 1.110.0's
    /// `operation/list_landing_zones/_list_landing_zones_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern. The N+1
    /// `get_landing_zone` fan-out only covers the ARNs collected across
    /// these pages, not the whole collection.
    pub async fn list_landing_zones(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_controltower::types::LandingZoneDetail>, Option<String>), VaporError>
    {
        let mut arns: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_landing_zones();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - arns.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for summary in output.landing_zones() {
                if let Some(arn) = summary.arn() {
                    arns.push(arn.to_string());
                }
            }
            token = output
                .next_token()
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if arns.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut details = Vec::with_capacity(arns.len());
        for arn in arns {
            let output = self
                .inner
                .get_landing_zone()
                .landing_zone_identifier(&arn)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            if let Some(detail) = output.landing_zone {
                details.push(detail);
            }
        }
        Ok((details, token))
    }

    /// Lists enabled controls, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListEnabledControls` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-controltower` 1.110.0's
    /// `operation/list_enabled_controls/_list_enabled_controls_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself,
    /// matching `kinesis.rs`'s `list_streams` pattern.
    pub async fn list_enabled_controls(
        &self,
        target_identifier: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_controltower::types::EnabledControlSummary>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_enabled_controls();
            if let Some(ref target) = target_identifier {
                req = req.target_identifier(target);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.enabled_controls);
            token = output.next_token.filter(|t| !t.is_empty());

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
    use aws_sdk_controltower::types::LandingZoneStatus;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://controltower.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_landing_zones_lists_all_with_get_landing_zone_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/list-landingzones"), "{}"),
                json_response(200, r#"{"landingZones":[{"arn":"lz-arn-1"}]}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/get-landingzone"),
                    r#"{"landingZoneIdentifier":"lz-arn-1"}"#,
                ),
                json_response(
                    200,
                    r#"{"landingZone":{"arn":"lz-arn-1","version":"3.3","status":"ACTIVE","latestAvailableVersion":"3.3"}}"#,
                ),
            ),
        ]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (details, token) = client.list_landing_zones(None, None).await.unwrap();

        assert_eq!(details.len(), 1);
        assert_eq!(details[0].arn(), Some("lz-arn-1"));
        assert_eq!(details[0].version(), "3.3");
        assert_eq!(details[0].status(), Some(&LandingZoneStatus::Active));
        assert_eq!(details[0].latest_available_version(), Some("3.3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_landing_zones_skips_get_landing_zone_fan_out_when_arn_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/list-landingzones"), "{}"),
            json_response(200, r#"{"landingZones":[{}]}"#),
        )]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (details, token) = client.list_landing_zones(None, None).await.unwrap();

        assert_eq!(details.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_landing_zones_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/list-landingzones"),
                r#"{"nextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"landingZones":[]}"#),
        )]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (details, token) = client
            .list_landing_zones(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(details.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_landing_zones_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/list-landingzones"), r#"{"maxResults":1}"#),
                json_response(
                    200,
                    r#"{"landingZones":[{"arn":"lz-arn-1"}],"nextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/get-landingzone"),
                    r#"{"landingZoneIdentifier":"lz-arn-1"}"#,
                ),
                json_response(200, r#"{"landingZone":{"arn":"lz-arn-1","version":"3.3"}}"#),
            ),
        ]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (details, token) = client.list_landing_zones(Some(1), None).await.unwrap();

        assert_eq!(details.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_landing_zones_propagates_list_errors() {
        // `ValidationException`, not a throttling-classified code (see memory
        // gotcha: those get retried and exhaust the single replay event,
        // surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/list-landingzones"), "{}"),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_landing_zones(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_landing_zones_propagates_get_landing_zone_fan_out_errors() {
        // Unlike `connect.rs`'s describe fan-out (which folds errors through
        // `.ok()`), `list_landing_zones`'s `get_landing_zone` fan-out
        // propagates via `?` just like the top-level list call -- verify
        // that shape explicitly rather than assuming it from the list call.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/list-landingzones"), "{}"),
                json_response(200, r#"{"landingZones":[{"arn":"lz-arn-1"}]}"#),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/get-landingzone"),
                    r#"{"landingZoneIdentifier":"lz-arn-1"}"#,
                ),
                json_error_response("ResourceNotFoundException", "no such landing zone"),
            ),
        ]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_landing_zones(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "no such landing zone");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_enabled_controls_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/list-enabled-controls"), "{}"),
            json_response(
                200,
                r#"{"enabledControls":[{"arn":"ec-arn-1","controlIdentifier":"ctrl-1","targetIdentifier":"ou-1"}]}"#,
            ),
        )]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_enabled_controls(None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].arn(), Some("ec-arn-1"));
        assert_eq!(items[0].control_identifier(), Some("ctrl-1"));
        assert_eq!(items[0].target_identifier(), Some("ou-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_enabled_controls_passes_through_target_identifier_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/list-enabled-controls"),
                r#"{"targetIdentifier":"ou-1"}"#,
            ),
            json_response(200, r#"{"enabledControls":[]}"#),
        )]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_enabled_controls(Some("ou-1".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_enabled_controls_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/list-enabled-controls"), r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"enabledControls":[{"arn":"ec-arn-1"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_enabled_controls(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_enabled_controls_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/list-enabled-controls"), r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"enabledControls":[{"arn":"ec-arn-1"},{"arn":"ec-arn-2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/list-enabled-controls"),
                    r#"{"nextToken":"p2","maxResults":8}"#,
                ),
                json_response(200, r#"{"enabledControls":[{"arn":"ec-arn-3"}]}"#),
            ),
        ]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_enabled_controls(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_enabled_controls_propagates_errors() {
        // `ValidationException`, not a throttling-classified code (see memory
        // gotcha: those get retried and exhaust the single replay event,
        // surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/list-enabled-controls"), "{}"),
            json_error_response("ValidationException", "bad target"),
        )]);
        let client = ControlTowerClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_enabled_controls(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad target");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

