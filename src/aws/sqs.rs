#[cfg(feature = "sqs")]
use aws_config::SdkConfig;
#[cfg(feature = "sqs")]
use aws_sdk_sqs::types::QueueAttributeName;
#[cfg(feature = "sqs")]
use std::collections::HashMap;

#[cfg(feature = "sqs")]
use crate::error::VaporError;

#[cfg(feature = "sqs")]
pub struct SqsClient {
    inner: aws_sdk_sqs::Client,
}

impl SqsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sqs::Client::new(config),
        }
    }

    /// Lists queue URLs, optionally filtered by name prefix, capped at
    /// `limit` results (default unlimited), and resumed from `next_token` if
    /// given. Returns the page's items plus a token to resume from, if more
    /// results remain.
    pub async fn list_queues(
        &self,
        prefix: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut urls = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_queues();
            if let Some(p) = prefix {
                req = req.queue_name_prefix(p);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - urls.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            urls.extend(output.queue_urls.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if urls.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((urls, token))
    }

    pub async fn get_queue_attributes(
        &self,
        queue_url: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(QueueAttributeName::All)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output
            .attributes()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.clone()))
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn list_queue_tags(
        &self,
        queue_url: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .list_queue_tags()
            .queue_url(queue_url)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output.tags().cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://sqs.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_queues_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"QueueUrls":["https://sqs.us-east-1.amazonaws.com/111111111111/q1","https://sqs.us-east-1.amazonaws.com/111111111111/q2"]}"#,
            ),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let (urls, token) = client.list_queues(None, None, None).await.unwrap();

        assert_eq!(urls.len(), 2);
        assert_eq!(
            urls[0],
            "https://sqs.us-east-1.amazonaws.com/111111111111/q1"
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_passes_through_prefix() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"QueueNamePrefix":"prod-"}"#),
            json_response(200, r#"{"QueueUrls":[]}"#),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let (urls, token) = client.list_queues(Some("prod-"), None, None).await.unwrap();

        assert_eq!(urls.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"QueueUrls":[]}"#),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let (urls, token) = client
            .list_queues(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(urls.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(200, r#"{"QueueUrls":["url-1"],"NextToken":"page2-token"}"#),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let (urls, token) = client.list_queues(None, Some(1), None).await.unwrap();

        assert_eq!(urls, vec!["url-1".to_string()]);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(200, r#"{"QueueUrls":["url-1","url-2"],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"QueueUrls":["url-3"]}"#),
            ),
        ]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let (urls, token) = client.list_queues(None, Some(10), None).await.unwrap();

        assert_eq!(urls.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queues_propagates_errors() {
        // `InvalidAddress`, not a throttling-classified code (see memory
        // gotcha: those get retried and exhaust the single replay event,
        // surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InvalidAddress", "bad address"),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_queues(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidAddress".to_string()));
                assert_eq!(message, "bad address");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_queue_attributes_returns_attribute_map() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"QueueUrl":"https://sqs.us-east-1.amazonaws.com/111111111111/q1","AttributeNames":["All"]}"#,
            ),
            json_response(
                200,
                r#"{"Attributes":{"QueueArn":"arn:aws:sqs:us-east-1:111111111111:q1","VisibilityTimeout":"30"}}"#,
            ),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let attrs = client
            .get_queue_attributes("https://sqs.us-east-1.amazonaws.com/111111111111/q1")
            .await
            .unwrap();

        assert_eq!(
            attrs.get("QueueArn"),
            Some(&"arn:aws:sqs:us-east-1:111111111111:q1".to_string())
        );
        assert_eq!(attrs.get("VisibilityTimeout"), Some(&"30".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_queue_attributes_returns_empty_map_when_attributes_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"QueueUrl":"q-url","AttributeNames":["All"]}"#),
            json_response(200, "{}"),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let attrs = client.get_queue_attributes("q-url").await.unwrap();

        assert!(attrs.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_queue_attributes_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"QueueUrl":"q-url","AttributeNames":["All"]}"#),
            json_error_response("InvalidAttributeName", "unknown attribute"),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let err = client.get_queue_attributes("q-url").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidAttributeName".to_string()));
                assert_eq!(message, "unknown attribute");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queue_tags_returns_tag_map() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"QueueUrl":"q-url"}"#),
            json_response(200, r#"{"Tags":{"env":"prod","team":"platform"}}"#),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let tags = client.list_queue_tags("q-url").await.unwrap();

        assert_eq!(tags.get("env"), Some(&"prod".to_string()));
        assert_eq!(tags.get("team"), Some(&"platform".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queue_tags_returns_empty_map_when_tags_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"QueueUrl":"q-url"}"#),
            json_response(200, "{}"),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let tags = client.list_queue_tags("q-url").await.unwrap();

        assert!(tags.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_queue_tags_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"QueueUrl":"q-url"}"#),
            json_error_response(
                "AWS.SimpleQueueService.NonExistentQueue",
                "queue does not exist",
            ),
        )]);
        let client = SqsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_queue_tags("q-url").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(
                    code,
                    Some("AWS.SimpleQueueService.NonExistentQueue".to_string())
                );
                assert_eq!(message, "queue does not exist");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
