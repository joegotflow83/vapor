use aws_config::SdkConfig;
use aws_sdk_kinesis::types::Shard;

use crate::error::VaporError;

pub struct KinesisClient {
    inner: aws_sdk_kinesis::Client,
}

impl KinesisClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_kinesis::Client::new(config),
        }
    }

    /// Lists Kinesis stream names, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `ListStreamsInput::limit` (this operation's `max_results`-equivalent) so a
    /// capped page boundary lands exactly on the returned token, matching
    /// `specs/plan-2-schema-v2-pagination-timestamps.md`'s client-layer pattern.
    pub async fn list_streams(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_streams();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.stream_names);
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn describe_stream_summary(
        &self,
        name: &str,
    ) -> Result<aws_sdk_kinesis::types::StreamDescriptionSummary, VaporError> {
        let output = self
            .inner
            .describe_stream_summary()
            .stream_name(name)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        output
            .stream_description_summary()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk {
                code: None,
                message: "No stream description summary".to_string(),
            })
    }

    /// Lists shards for a stream, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListShards` requires that
    /// `stream_name` is set only on the first page — once a `next_token` is in
    /// play (either passed in by the caller to resume, or returned from a prior
    /// page within this loop), the API rejects a request carrying both.
    /// `max_results` is capped to the remaining budget so a `limit`-truncated
    /// page boundary always lines up with the token AWS returns (verified: no
    /// generated paginator exists for `ListShards` in `aws-sdk-kinesis`
    /// 1.109.0 — `operation/list_shards/` has no `paginator.rs` — so this loop
    /// is required, not just a style choice).
    pub async fn list_shards(
        &self,
        name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Shard>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_shards();
            if let Some(ref t) = token {
                req = req.next_token(t);
            } else {
                req = req.stream_name(name);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.shards().to_vec());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn list_tags_for_stream(
        &self,
        name: &str,
    ) -> Result<Vec<aws_sdk_kinesis::types::Tag>, VaporError> {
        let mut tags = Vec::new();
        let mut exclusive_start_key: Option<String> = None;

        loop {
            let mut req = self.inner.list_tags_for_stream().stream_name(name);
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

    const ENDPOINT: &str = "https://kinesis.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_streams_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"StreamNames":["stream-a","stream-b"],"HasMoreStreams":false}"#,
            ),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_streams(None, None).await.unwrap();

        assert_eq!(items, vec!["stream-a".to_string(), "stream-b".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_streams_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(
                200,
                r#"{"StreamNames":["stream-c"],"HasMoreStreams":false}"#,
            ),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_streams(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items, vec!["stream-c".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_streams_stops_at_limit_and_returns_resume_token() {
        // ListStreams forwards `Limit` straight to AWS with no client-side
        // truncation, so the canned response must return exactly the
        // requested count, not more (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":2}"#),
            json_response(
                200,
                r#"{"StreamNames":["stream-a","stream-b"],"HasMoreStreams":true,"NextToken":"page2"}"#,
            ),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_streams(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_streams_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(
                    200,
                    r#"{"StreamNames":["stream-a","stream-b"],"HasMoreStreams":true,"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","Limit":8}"#),
                json_response(
                    200,
                    r#"{"StreamNames":["stream-c"],"HasMoreStreams":false}"#,
                ),
            ),
        ]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_streams(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_streams_propagates_errors() {
        // `InvalidArgumentException`, not a throttling-classified code
        // (durable gotcha 1: those get retried and exhaust the single
        // replay event).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidArgumentException", "bad argument"),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let err = client.list_streams(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidArgumentException".to_string()));
                assert_eq!(message, "bad argument");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_stream_summary_returns_summary() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
            json_response(
                200,
                r#"{"StreamDescriptionSummary":{"StreamName":"my-stream","StreamARN":"arn:aws:kinesis:us-east-1:123456789012:stream/my-stream","StreamStatus":"ACTIVE","RetentionPeriodHours":24,"StreamCreationTimestamp":1700000000,"EnhancedMonitoring":[],"OpenShardCount":4}}"#,
            ),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let summary = client.describe_stream_summary("my-stream").await.unwrap();

        assert_eq!(summary.stream_name(), "my-stream");
        assert_eq!(
            summary.stream_arn(),
            "arn:aws:kinesis:us-east-1:123456789012:stream/my-stream"
        );
        assert_eq!(
            summary.stream_status(),
            &aws_sdk_kinesis::types::StreamStatus::Active
        );
        assert_eq!(summary.retention_period_hours(), 24);
        assert_eq!(summary.open_shard_count(), 4);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_stream_summary_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"missing-stream"}"#),
            json_error_response("ResourceNotFoundException", "stream not found"),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_stream_summary("missing-stream")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "stream not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_shards_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
            json_response(200, r#"{"Shards":[{"ShardId":"shardId-000000000000"}]}"#),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_shards("my-stream", None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].shard_id(), "shardId-000000000000");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_shards_resumes_from_provided_next_token_without_stream_name() {
        // `ListShards` rejects a request carrying both `StreamName` and
        // `NextToken` — once a token is in play the wrapper must send only
        // the token, not the stream name (see the method's doc comment).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Shards":[{"ShardId":"shardId-000000000001"}]}"#),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_shards("my-stream", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].shard_id(), "shardId-000000000001");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_shards_stops_at_limit_and_returns_resume_token() {
        // ListShards forwards `MaxResults` straight to AWS with no
        // client-side truncation (durable gotcha 13), so the canned
        // response must return exactly the requested count.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream","MaxResults":2}"#),
            json_response(
                200,
                r#"{"Shards":[{"ShardId":"s1"},{"ShardId":"s2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_shards("my-stream", Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_shards_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"StreamName":"my-stream","MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Shards":[{"ShardId":"s1"},{"ShardId":"s2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Shards":[{"ShardId":"s3"}]}"#),
            ),
        ]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_shards("my-stream", Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_shards_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
            json_error_response("ResourceNotFoundException", "stream not found"),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_shards("my-stream", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "stream not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_stream_returns_all_tags_single_page() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
            json_response(
                200,
                r#"{"Tags":[{"Key":"env","Value":"prod"}],"HasMoreTags":false}"#,
            ),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let tags = client.list_tags_for_stream("my-stream").await.unwrap();

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), "env");
        assert_eq!(tags[0].value(), Some("prod"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_stream_returns_empty_when_no_tags() {
        // An empty `Tags` page breaks the loop immediately via the
        // `page_tags.is_empty()` check, regardless of what `HasMoreTags`
        // claims.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
            json_response(200, r#"{"Tags":[],"HasMoreTags":true}"#),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let tags = client.list_tags_for_stream("my-stream").await.unwrap();

        assert!(tags.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_stream_pages_through_until_has_more_tags_false() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
                json_response(
                    200,
                    r#"{"Tags":[{"Key":"a","Value":"1"},{"Key":"b","Value":"2"}],"HasMoreTags":true}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"StreamName":"my-stream","ExclusiveStartTagKey":"b"}"#,
                ),
                json_response(
                    200,
                    r#"{"Tags":[{"Key":"c","Value":"3"}],"HasMoreTags":false}"#,
                ),
            ),
        ]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let tags = client.list_tags_for_stream("my-stream").await.unwrap();

        assert_eq!(tags.len(), 3);
        assert_eq!(tags[2].key(), "c");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_stream_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
            json_error_response("InvalidArgumentException", "bad argument"),
        )]);
        let client = KinesisClient::new(&sdk_config(http_client.clone()));

        let err = client.list_tags_for_stream("my-stream").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidArgumentException".to_string()));
                assert_eq!(message, "bad argument");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
