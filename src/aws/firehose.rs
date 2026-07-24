use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct FirehoseClient {
    inner: aws_sdk_firehose::Client,
}

impl FirehoseClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_firehose::Client::new(config),
        }
    }

    /// Lists Firehose delivery stream names, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListDeliveryStreams` has no opaque continuation token — it uses
    /// `ExclusiveStartDeliveryStreamName` (the alphabetically-last name seen)
    /// plus a `HasMoreDeliveryStreams` boolean instead (verified against
    /// pinned `aws-sdk-firehose` 1.111.0's
    /// `operation/list_delivery_streams/_list_delivery_streams_input.rs`), so
    /// the last name in a page is reused directly as `next_token`, matching
    /// `kinesis.rs`'s `list_streams` loop shape otherwise. `limit` is capped
    /// to the remaining budget via the request's own `limit` field so a
    /// capped page boundary always lands exactly on the returned token.
    pub async fn list_delivery_streams(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_delivery_streams();
            if let Some(ref t) = token {
                req = req.exclusive_start_delivery_stream_name(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            let names = output.delivery_stream_names();
            if names.is_empty() {
                token = None;
                break;
            }
            let last = names.last().map(|s| s.to_string());
            items.extend(names.iter().map(|s| s.to_string()));
            token = if output.has_more_delivery_streams() {
                last
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn describe_delivery_stream(
        &self,
        name: &str,
    ) -> Result<aws_sdk_firehose::types::DeliveryStreamDescription, VaporError> {
        let output = self
            .inner
            .describe_delivery_stream()
            .delivery_stream_name(name)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        output
            .delivery_stream_description()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk {
                code: None,
                message: "No delivery stream description".to_string(),
            })
    }

    pub async fn list_tags_for_delivery_stream(
        &self,
        name: &str,
    ) -> Result<Vec<aws_sdk_firehose::types::Tag>, VaporError> {
        let mut tags = Vec::new();
        let mut exclusive_start_key: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_tags_for_delivery_stream()
                .delivery_stream_name(name);
            if let Some(ref key) = exclusive_start_key {
                req = req.exclusive_start_tag_key(key);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            let page_tags = output.tags();
            if page_tags.is_empty() {
                break;
            }
            let last_key = page_tags.last().map(|t| t.key().to_string());
            tags.extend(page_tags.to_vec());

            if output.has_more_tags() {
                exclusive_start_key = last_key;
            } else {
                break;
            }
        }

        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use aws_sdk_firehose::types::{DeliveryStreamStatus, DeliveryStreamType};

    const ENDPOINT: &str = "https://firehose.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_delivery_streams_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"DeliveryStreamNames":["stream-a","stream-b"],"HasMoreDeliveryStreams":false}"#,
            ),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_delivery_streams(None, None).await.unwrap();

        assert_eq!(names, vec!["stream-a".to_string(), "stream-b".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_delivery_streams_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"ExclusiveStartDeliveryStreamName":"stream-a"}"#,
            ),
            json_response(
                200,
                r#"{"DeliveryStreamNames":["stream-b"],"HasMoreDeliveryStreams":false}"#,
            ),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client
            .list_delivery_streams(None, Some("stream-a".to_string()))
            .await
            .unwrap();

        assert_eq!(names, vec!["stream-b".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_delivery_streams_stops_at_limit_and_returns_last_name_as_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":2}"#),
            json_response(
                200,
                r#"{"DeliveryStreamNames":["stream-a","stream-b"],"HasMoreDeliveryStreams":true}"#,
            ),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_delivery_streams(Some(2), None).await.unwrap();

        assert_eq!(names, vec!["stream-a".to_string(), "stream-b".to_string()]);
        assert_eq!(token, Some("stream-b".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_delivery_streams_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(
                    200,
                    r#"{"DeliveryStreamNames":["stream-a","stream-b"],"HasMoreDeliveryStreams":true}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"Limit":8,"ExclusiveStartDeliveryStreamName":"stream-b"}"#,
                ),
                json_response(
                    200,
                    r#"{"DeliveryStreamNames":["stream-c"],"HasMoreDeliveryStreams":false}"#,
                ),
            ),
        ]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let (names, token) = client.list_delivery_streams(Some(10), None).await.unwrap();

        assert_eq!(
            names,
            vec![
                "stream-a".to_string(),
                "stream-b".to_string(),
                "stream-c".to_string()
            ]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_delivery_streams_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidArgumentException", "bad request"),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let err = client.list_delivery_streams(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidArgumentException"));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_delivery_stream_returns_description() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
            json_response(
                200,
                r#"{"DeliveryStreamDescription":{"DeliveryStreamName":"my-stream","DeliveryStreamARN":"arn:aws:firehose:us-east-1:1:deliverystream/my-stream","DeliveryStreamStatus":"ACTIVE","DeliveryStreamType":"DirectPut","VersionId":"1","Destinations":[],"HasMoreDestinations":false}}"#,
            ),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let description = client.describe_delivery_stream("my-stream").await.unwrap();

        assert_eq!(description.delivery_stream_name(), "my-stream");
        assert_eq!(
            description.delivery_stream_arn(),
            "arn:aws:firehose:us-east-1:1:deliverystream/my-stream"
        );
        assert_eq!(
            description.delivery_stream_status(),
            &DeliveryStreamStatus::Active
        );
        assert_eq!(
            description.delivery_stream_type(),
            &DeliveryStreamType::DirectPut
        );
        assert_eq!(description.version_id(), "1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_delivery_stream_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DeliveryStreamName":"missing-stream"}"#),
            json_error_response("ResourceNotFoundException", "delivery stream not found"),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_delivery_stream("missing-stream")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ResourceNotFoundException"));
                assert_eq!(message, "delivery stream not found");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_delivery_stream_lists_all_tags_on_one_page() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
            json_response(
                200,
                r#"{"Tags":[{"Key":"Env","Value":"prod"},{"Key":"Owner"}],"HasMoreTags":false}"#,
            ),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let tags = client
            .list_tags_for_delivery_stream("my-stream")
            .await
            .unwrap();

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].key(), "Env");
        assert_eq!(tags[0].value(), Some("prod"));
        assert_eq!(tags[1].key(), "Owner");
        assert_eq!(tags[1].value(), None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_delivery_stream_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
                json_response(200, r#"{"Tags":[{"Key":"a"}],"HasMoreTags":true}"#),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"DeliveryStreamName":"my-stream","ExclusiveStartTagKey":"a"}"#,
                ),
                json_response(200, r#"{"Tags":[{"Key":"b"}],"HasMoreTags":false}"#),
            ),
        ]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let tags = client
            .list_tags_for_delivery_stream("my-stream")
            .await
            .unwrap();

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].key(), "a");
        assert_eq!(tags[1].key(), "b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_delivery_stream_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
            json_error_response("InvalidArgumentException", "invalid tag key"),
        )]);
        let client = FirehoseClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_tags_for_delivery_stream("my-stream")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidArgumentException"));
                assert_eq!(message, "invalid tag key");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
