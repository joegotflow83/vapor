use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct ComprehendClient {
    inner: aws_sdk_comprehend::Client,
}

impl ComprehendClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_comprehend::Client::new(config),
        }
    }

    /// Lists entity recognizers, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_entity_recognizers(
        &self,
        status_filter: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_comprehend::types::EntityRecognizerProperties>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_entity_recognizers();
            if let Some(ref s) = status_filter {
                let filter = aws_sdk_comprehend::types::EntityRecognizerFilter::builder()
                    .status(aws_sdk_comprehend::types::ModelStatus::from(s.as_str()))
                    .build();
                req = req.filter(filter);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.entity_recognizer_properties_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists document classifiers, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_document_classifiers(
        &self,
        status_filter: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_comprehend::types::DocumentClassifierProperties>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_document_classifiers();
            if let Some(ref s) = status_filter {
                let filter = aws_sdk_comprehend::types::DocumentClassifierFilter::builder()
                    .status(aws_sdk_comprehend::types::ModelStatus::from(s.as_str()))
                    .build();
                req = req.filter(filter);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(
                output
                    .document_classifier_properties_list
                    .unwrap_or_default(),
            );
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists endpoints, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_endpoints(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_comprehend::types::EndpointProperties>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_endpoints();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.endpoint_properties_list.unwrap_or_default());
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
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://comprehend.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_entity_recognizers_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Filter":{"Status":"TRAINED"}}"#),
            json_response(
                200,
                r#"{"EntityRecognizerPropertiesList":[{"EntityRecognizerArn":"arn1"},{"EntityRecognizerArn":"arn2"}]}"#,
            ),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_entity_recognizers(Some("TRAINED".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].entity_recognizer_arn(), Some("arn1"));
        assert_eq!(items[1].entity_recognizer_arn(), Some("arn2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_entity_recognizers_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"EntityRecognizerPropertiesList":[{"EntityRecognizerArn":"arn3"}]}"#),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_entity_recognizers(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].entity_recognizer_arn(), Some("arn3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_entity_recognizers_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"EntityRecognizerPropertiesList":[{"EntityRecognizerArn":"a1"},{"EntityRecognizerArn":"a2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_entity_recognizers(None, Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_entity_recognizers_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"EntityRecognizerPropertiesList":[{"EntityRecognizerArn":"a1"},{"EntityRecognizerArn":"a2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"EntityRecognizerPropertiesList":[{"EntityRecognizerArn":"a3"}]}"#),
            ),
        ]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_entity_recognizers(None, Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_entity_recognizers_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "bad filter"),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        match client.list_entity_recognizers(None, None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad filter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_document_classifiers_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Filter":{"Status":"TRAINED"}}"#),
            json_response(
                200,
                r#"{"DocumentClassifierPropertiesList":[{"DocumentClassifierArn":"dc1"},{"DocumentClassifierArn":"dc2"}]}"#,
            ),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_document_classifiers(Some("TRAINED".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].document_classifier_arn(), Some("dc1"));
        assert_eq!(items[1].document_classifier_arn(), Some("dc2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_document_classifiers_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"DocumentClassifierPropertiesList":[{"DocumentClassifierArn":"dc1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_document_classifiers(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_document_classifiers_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "bad filter"),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        match client.list_document_classifiers(None, None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad filter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"EndpointPropertiesList":[{"EndpointArn":"ep1"},{"EndpointArn":"ep2"}]}"#,
            ),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_endpoints(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].endpoint_arn(), Some("ep1"));
        assert_eq!(items[1].endpoint_arn(), Some("ep2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"EndpointPropertiesList":[{"EndpointArn":"ep1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_endpoints(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = ComprehendClient::new(&sdk_config(http_client.clone()));

        match client.list_endpoints(None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
