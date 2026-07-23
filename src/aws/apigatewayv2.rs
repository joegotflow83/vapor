#[cfg(feature = "apigatewayv2")]
use aws_config::SdkConfig;

#[cfg(feature = "apigatewayv2")]
use crate::error::VaporError;

#[cfg(feature = "apigatewayv2")]
pub struct ApiGatewayV2Client {
    inner: aws_sdk_apigatewayv2::Client,
}

impl ApiGatewayV2Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_apigatewayv2::Client::new(config),
        }
    }

    /// Lists APIs, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `GetApis` has no SDK paginator
    /// (confirmed: no `paginator.rs` under `aws-sdk-apigatewayv2`'s generated
    /// `operation/get_apis/`), but `max_results` is a string-typed field (not
    /// `i32` like kinesis/mq), so the remaining budget is capped and
    /// stringified on each request, matching `pinpoint.rs`'s loop shape.
    pub async fn get_apis(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigatewayv2::types::Api>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_apis();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results((l - items.len() as i32).to_string());
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items().iter().cloned());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists stages for `api_id`, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetStages` has no
    /// SDK paginator, same as `get_apis`.
    pub async fn get_stages(
        &self,
        api_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigatewayv2::types::Stage>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_stages().api_id(api_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results((l - items.len() as i32).to_string());
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items().iter().cloned());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists routes for `api_id`, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetRoutes` has no
    /// SDK paginator, same as `get_apis`.
    pub async fn get_routes(
        &self,
        api_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigatewayv2::types::Route>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_routes().api_id(api_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results((l - items.len() as i32).to_string());
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items().iter().cloned());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists custom domain names, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetDomainNames`
    /// has no SDK paginator, same as `get_apis`.
    pub async fn get_domain_names(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigatewayv2::types::DomainName>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_domain_names();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results((l - items.len() as i32).to_string());
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items().iter().cloned());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists VPC links, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `GetVpcLinks` has no SDK
    /// paginator, same as `get_apis`.
    pub async fn get_vpc_links(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_apigatewayv2::types::VpcLink>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_vpc_links();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results((l - items.len() as i32).to_string());
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.items().iter().cloned());
            token = output.next_token().map(|t| t.to_string());

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

    const APIS: &str = "https://apigateway.us-east-1.amazonaws.com/v2/apis";

    #[tokio::test]
    async fn get_apis_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APIS, ""),
            json_response(
                200,
                r#"{"items":[{"apiId":"api1","name":"one"},{"apiId":"api2","name":"two"}]}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.get_apis(None, None).await.unwrap();

        assert_eq!(apis.len(), 2);
        assert_eq!(apis[0].api_id(), Some("api1"));
        assert_eq!(apis[1].name(), Some("two"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apis_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?nextToken=cursor-a"), ""),
            json_response(200, r#"{"items":[{"apiId":"api3","name":"three"}]}"#),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (apis, token) = client
            .get_apis(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(apis.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apis_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?maxResults=2"), ""),
            json_response(
                200,
                r#"{"items":[{"apiId":"a1","name":"one"},{"apiId":"a2","name":"two"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.get_apis(Some(2), None).await.unwrap();

        assert_eq!(apis.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apis_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{APIS}?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"items":[{"apiId":"a1","name":"one"},{"apiId":"a2","name":"two"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{APIS}?maxResults=8&nextToken=p2"), ""),
                json_response(200, r#"{"items":[{"apiId":"a3","name":"three"}]}"#),
            ),
        ]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.get_apis(Some(10), None).await.unwrap();

        assert_eq!(apis.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_apis_propagates_errors() {
        // `BadRequestException` (rather than `TooManyRequestsException`) since
        // the latter is on the AWS SDK's built-in throttling-retry list, which
        // would consume a *second* replay event this single-event client
        // doesn't have (see apigateway.rs's precedent for this pitfall).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APIS, ""),
            json_error_response("BadRequestException", "invalid request"),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let err = client.get_apis(None, None).await.unwrap_err();

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
    async fn get_stages_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/v2/apis/api1/stages",
                "",
            ),
            json_response(
                200,
                r#"{"items":[{"stageName":"prod"},{"stageName":"dev"}]}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (stages, token) = client.get_stages("api1", None, None).await.unwrap();

        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].stage_name(), Some("prod"));
        assert_eq!(stages[1].stage_name(), Some("dev"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_stages_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/v2/apis/api1/stages?maxResults=1",
                "",
            ),
            json_response(
                200,
                r#"{"items":[{"stageName":"prod"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (stages, token) = client.get_stages("api1", Some(1), None).await.unwrap();

        assert_eq!(stages.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_routes_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/v2/apis/api1/routes",
                "",
            ),
            json_response(
                200,
                r#"{"items":[{"routeId":"r1","routeKey":"GET /"},{"routeId":"r2","routeKey":"POST /items"}]}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (routes, token) = client.get_routes("api1", None, None).await.unwrap();

        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].route_id(), Some("r1"));
        assert_eq!(routes[1].route_key(), Some("POST /items"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_routes_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/v2/apis/api1/routes?maxResults=1",
                "",
            ),
            json_response(
                200,
                r#"{"items":[{"routeId":"r1","routeKey":"GET /"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (routes, token) = client.get_routes("api1", Some(1), None).await.unwrap();

        assert_eq!(routes.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_domain_names_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/v2/domainnames",
                "",
            ),
            json_response(200, r#"{"items":[{"domainName":"example.com"}]}"#),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (domains, token) = client.get_domain_names(None, None).await.unwrap();

        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].domain_name(), Some("example.com"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_domain_names_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/v2/domainnames?maxResults=1",
                "",
            ),
            json_response(
                200,
                r#"{"items":[{"domainName":"example.com"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (domains, token) = client.get_domain_names(Some(1), None).await.unwrap();

        assert_eq!(domains.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_vpc_links_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request("https://apigateway.us-east-1.amazonaws.com/v2/vpclinks", ""),
            json_response(200, r#"{"items":[{"vpcLinkId":"vpc1","name":"link1"}]}"#),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (links, token) = client.get_vpc_links(None, None).await.unwrap();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].vpc_link_id(), Some("vpc1"));
        assert_eq!(links[0].name(), Some("link1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_vpc_links_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://apigateway.us-east-1.amazonaws.com/v2/vpclinks?maxResults=1",
                "",
            ),
            json_response(
                200,
                r#"{"items":[{"vpcLinkId":"vpc1","name":"link1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = ApiGatewayV2Client::new(&sdk_config(http_client.clone()));

        let (links, token) = client.get_vpc_links(Some(1), None).await.unwrap();

        assert_eq!(links.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }
}
