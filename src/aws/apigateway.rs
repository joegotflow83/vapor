#[cfg(feature = "apigateway")]
use aws_config::SdkConfig;

#[cfg(feature = "apigateway")]
use crate::error::VaporError;

#[cfg(feature = "apigateway")]
pub struct ApiGatewayClient {
    rest: aws_sdk_apigateway::Client,
}

#[cfg(feature = "apigateway")]
impl ApiGatewayClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            rest: aws_sdk_apigateway::Client::new(config),
        }
    }

    // ── REST API (v1) ─────────────────────────────────────────────────────────

    /// Lists REST APIs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `GetRestApis` has both
    /// `position` and an `i32` `limit` (verified against pinned
    /// `aws-sdk-apigateway` 1.108.0's
    /// `operation/get_rest_apis/_get_rest_apis_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `control_tower.rs`'s `list_landing_zones` pattern.
    pub async fn list_rest_apis(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigateway::types::RestApi>, Option<String>), VaporError> {
        let mut results = Vec::new();
        let mut position = next_token;
        loop {
            let mut req = self.rest.get_rest_apis();
            if let Some(ref pos) = position {
                req = req.position(pos);
            }
            if let Some(l) = limit {
                req = req.limit(l - results.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.items().iter().cloned());
            position = output.position().map(|p| p.to_string());

            match (&position, limit) {
                (None, _) => break,
                (_, Some(l)) if results.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((results, position))
    }

    /// Lists stages for a REST API. `GetStages` has no `position`/`limit`
    /// fields at all (verified against pinned `aws-sdk-apigateway` 1.108.0's
    /// `operation/get_stages/_get_stages_{input,output}.rs`) — it always
    /// returns every stage for the API in one call, so there is no token to
    /// surface (same class as `sts`'s single-op file, but at the
    /// individual-operation level).
    pub async fn list_rest_stages(
        &self,
        api_id: &str,
    ) -> Result<Vec<aws_sdk_apigateway::types::Stage>, VaporError> {
        let output = self
            .rest
            .get_stages()
            .rest_api_id(api_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.item().to_vec())
    }

    /// Lists resources (path nodes) for a REST API, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `GetResources` has both `position` and an `i32` `limit` (verified
    /// against pinned `aws-sdk-apigateway` 1.108.0's
    /// `operation/get_resources/_get_resources_input.rs`), same shape as
    /// `list_rest_apis` above.
    pub async fn list_rest_resources(
        &self,
        api_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigateway::types::Resource>, Option<String>), VaporError> {
        let mut results = Vec::new();
        let mut position = next_token;
        loop {
            let mut req = self.rest.get_resources().rest_api_id(api_id);
            if let Some(ref pos) = position {
                req = req.position(pos);
            }
            if let Some(l) = limit {
                req = req.limit(l - results.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.items().iter().cloned());
            position = output.position().map(|p| p.to_string());

            match (&position, limit) {
                (None, _) => break,
                (_, Some(l)) if results.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((results, position))
    }

    /// Lists deployments for a REST API, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `GetDeployments` has both `position` and an `i32` `limit` (verified
    /// against pinned `aws-sdk-apigateway` 1.108.0's
    /// `operation/get_deployments/_get_deployments_input.rs`), same shape as
    /// `list_rest_apis` above.
    pub async fn list_rest_deployments(
        &self,
        api_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigateway::types::Deployment>, Option<String>), VaporError> {
        let mut results = Vec::new();
        let mut position = next_token;
        loop {
            let mut req = self.rest.get_deployments().rest_api_id(api_id);
            if let Some(ref pos) = position {
                req = req.position(pos);
            }
            if let Some(l) = limit {
                req = req.limit(l - results.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.items().iter().cloned());
            position = output.position().map(|p| p.to_string());

            match (&position, limit) {
                (None, _) => break,
                (_, Some(l)) if results.len() as i32 >= l => break,
                _ => continue,
            }
        }
        Ok((results, position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const APIS: &str = "https://apigateway.us-east-1.amazonaws.com/restapis";

    #[tokio::test]
    async fn list_rest_apis_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APIS, ""),
            json_response(
                200,
                r#"{"item":[{"id":"api1","name":"one"},{"id":"api2","name":"two"}]}"#,
            ),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.list_rest_apis(None, None).await.unwrap();

        assert_eq!(apis.len(), 2);
        assert_eq!(apis[0].id(), Some("api1"));
        assert_eq!(apis[1].name(), Some("two"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_apis_resumes_from_provided_position() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?position=cursor-a"), ""),
            json_response(200, r#"{"item":[{"id":"api3","name":"three"}]}"#),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client
            .list_rest_apis(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(apis.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_apis_stops_at_limit_and_returns_resume_position() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?limit=2"), ""),
            json_response(
                200,
                r#"{"item":[{"id":"a1","name":"one"},{"id":"a2","name":"two"}],"position":"page2"}"#,
            ),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.list_rest_apis(Some(2), None).await.unwrap();

        assert_eq!(apis.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_apis_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{APIS}?limit=10"), ""),
                json_response(
                    200,
                    r#"{"item":[{"id":"a1","name":"one"},{"id":"a2","name":"two"}],"position":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{APIS}?position=p2&limit=8"), ""),
                json_response(200, r#"{"item":[{"id":"a3","name":"three"}]}"#),
            ),
        ]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.list_rest_apis(Some(10), None).await.unwrap();

        assert_eq!(apis.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_apis_propagates_errors() {
        // Uses `UnauthorizedException` rather than the more-obvious
        // `TooManyRequestsException` — the latter is on the AWS SDK's
        // built-in throttling-retry list, so the retry strategy consumes a
        // *second* replay event that a single-event `StaticReplayClient`
        // doesn't have, surfacing as `SdkError::DispatchFailure` (code
        // `None`) instead of exercising this file's error-mapping path.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APIS, ""),
            json_error_response("UnauthorizedException", "not authorized"),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let err = client.list_rest_apis(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("UnauthorizedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_stages_returns_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/restapis/api1/stages",
                "",
            ),
            json_response(
                200,
                r#"{"item":[{"stageName":"prod","deploymentId":"dep1"},{"stageName":"dev","deploymentId":"dep2"}]}"#,
            ),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let stages = client.list_rest_stages("api1").await.unwrap();

        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].stage_name(), Some("prod"));
        assert_eq!(stages[1].stage_name(), Some("dev"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_resources_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/restapis/api1/resources",
                "",
            ),
            json_response(200, r#"{"item":[{"id":"res1","pathPart":"users"}]}"#),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client
            .list_rest_resources("api1", None, None)
            .await
            .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].path_part(), Some("users"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_resources_stops_at_limit_and_returns_resume_position() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/restapis/api1/resources?limit=1",
                "",
            ),
            json_response(
                200,
                r#"{"item":[{"id":"res1","pathPart":"users"}],"position":"page2"}"#,
            ),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client
            .list_rest_resources("api1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_deployments_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/restapis/api1/deployments",
                "",
            ),
            json_response(200, r#"{"item":[{"id":"dep1"},{"id":"dep2"}]}"#),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (deployments, token) = client
            .list_rest_deployments("api1", None, None)
            .await
            .unwrap();

        assert_eq!(deployments.len(), 2);
        assert_eq!(deployments[0].id(), Some("dep1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rest_deployments_stops_at_limit_and_returns_resume_position() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/restapis/api1/deployments?limit=1",
                "",
            ),
            json_response(200, r#"{"item":[{"id":"dep1"}],"position":"page2"}"#),
        )]);
        let client = ApiGatewayClient::new(&sdk_config(http_client.clone()));

        let (deployments, token) = client
            .list_rest_deployments("api1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(deployments.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }
}
