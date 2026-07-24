use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct AppSyncClient {
    inner: aws_sdk_appsync::Client,
}

impl AppSyncClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_appsync::Client::new(config),
        }
    }

    /// Lists AppSync GraphQL APIs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `ListGraphqlApisInput::max_results` so a capped page boundary lands
    /// exactly on the returned token.
    pub async fn list_graphql_apis(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_appsync::types::GraphqlApi>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_graphql_apis();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.graphql_apis.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists AppSync data sources for an API, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`. `limit` is
    /// handed to AWS via `ListDataSourcesInput::max_results` so a capped page
    /// boundary lands exactly on the returned token.
    pub async fn list_data_sources(
        &self,
        api_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_appsync::types::DataSource>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_data_sources().api_id(api_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.data_sources.unwrap_or_default());
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

    const APIS: &str = "https://appsync.us-east-1.amazonaws.com/v1/apis";

    #[tokio::test]
    async fn list_graphql_apis_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APIS, ""),
            json_response(
                200,
                r#"{"graphqlApis":[{"apiId":"api1","name":"one"},{"apiId":"api2","name":"two"}]}"#,
            ),
        )]);
        let client = AppSyncClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.list_graphql_apis(None, None).await.unwrap();

        assert_eq!(apis.len(), 2);
        assert_eq!(apis[0].api_id(), Some("api1"));
        assert_eq!(apis[1].name(), Some("two"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphql_apis_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?nextToken=cursor-a"), ""),
            json_response(200, r#"{"graphqlApis":[{"apiId":"api3","name":"three"}]}"#),
        )]);
        let client = AppSyncClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client
            .list_graphql_apis(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(apis.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphql_apis_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{APIS}?maxResults=2"), ""),
            json_response(
                200,
                r#"{"graphqlApis":[{"apiId":"a1","name":"one"},{"apiId":"a2","name":"two"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = AppSyncClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.list_graphql_apis(Some(2), None).await.unwrap();

        assert_eq!(apis.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphql_apis_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{APIS}?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"graphqlApis":[{"apiId":"a1","name":"one"},{"apiId":"a2","name":"two"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{APIS}?nextToken=p2&maxResults=8"), ""),
                json_response(200, r#"{"graphqlApis":[{"apiId":"a3","name":"three"}]}"#),
            ),
        ]);
        let client = AppSyncClient::new(&sdk_config(http_client.clone()));

        let (apis, token) = client.list_graphql_apis(Some(10), None).await.unwrap();

        assert_eq!(apis.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_graphql_apis_propagates_errors() {
        // `BadRequestException` (not a throttling exception — see
        // apigateway.rs's precedent for why that would consume a second
        // replay event via the SDK's default retry strategy).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(APIS, ""),
            json_error_response("BadRequestException", "invalid request"),
        )]);
        let client = AppSyncClient::new(&sdk_config(http_client.clone()));

        let err = client.list_graphql_apis(None, None).await.unwrap_err();

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
    async fn list_data_sources_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://appsync.us-east-1.amazonaws.com/v1/apis/api1/datasources",
                "",
            ),
            json_response(200, r#"{"dataSources":[{"name":"ds1"},{"name":"ds2"}]}"#),
        )]);
        let client = AppSyncClient::new(&sdk_config(http_client.clone()));

        let (sources, token) = client.list_data_sources("api1", None, None).await.unwrap();

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].name(), Some("ds1"));
        assert_eq!(sources[1].name(), Some("ds2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_data_sources_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://appsync.us-east-1.amazonaws.com/v1/apis/api1/datasources?maxResults=1",
                "",
            ),
            json_response(
                200,
                r#"{"dataSources":[{"name":"ds1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = AppSyncClient::new(&sdk_config(http_client.clone()));

        let (sources, token) = client
            .list_data_sources("api1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }
}
