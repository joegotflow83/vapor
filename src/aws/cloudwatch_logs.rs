#[cfg(feature = "cloudwatchlogs")]
use aws_config::SdkConfig;
#[cfg(feature = "cloudwatchlogs")]
use aws_sdk_cloudwatchlogs::types::OrderBy;

#[cfg(feature = "cloudwatchlogs")]
use crate::error::VaporError;

#[cfg(feature = "cloudwatchlogs")]
pub struct CloudWatchLogsClient {
    inner: aws_sdk_cloudwatchlogs::Client,
}

impl CloudWatchLogsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudwatchlogs::Client::new(config),
        }
    }

    /// List CloudWatch Logs log groups, optionally filtered by name prefix.
    /// `limit` caps the total number of groups returned across all pages
    /// (default unlimited) and resumes from `next_token`. `DescribeLogGroups`
    /// has both `limit` and `next_token` (verified against pinned
    /// `aws-sdk-cloudwatchlogs` 1.139.0's `operation/describe_log_groups/
    /// _describe_log_groups_input.rs`), so `limit` is capped to the
    /// remaining budget on the request itself, matching `mq.rs`'s
    /// `list_brokers` pattern.
    pub async fn describe_log_groups(
        &self,
        prefix: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_cloudwatchlogs::types::LogGroup>, Option<String>), VaporError> {
        let mut groups = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self.inner.describe_log_groups();
            if let Some(ref p) = prefix {
                request = request.log_group_name_prefix(p);
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.limit(l - groups.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            groups.extend(output.log_groups().to_vec());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if groups.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((groups, token))
    }

    /// List log streams within a log group, optionally filtered by name
    /// prefix and ordered. `limit` caps the total number of streams
    /// returned across all pages (default unlimited) and resumes from
    /// `next_token`. `DescribeLogStreams` has both `limit` and `next_token`
    /// (verified against pinned `aws-sdk-cloudwatchlogs` 1.139.0's
    /// `operation/describe_log_streams/_describe_log_streams_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself.
    pub async fn describe_log_streams(
        &self,
        log_group_name: String,
        prefix: Option<String>,
        order_by: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_cloudwatchlogs::types::LogStream>,
            Option<String>,
        ),
        VaporError,
    > {
        let sdk_order_by = order_by.as_deref().map(|s| match s {
            "LastEventTime" => OrderBy::LastEventTime,
            _ => OrderBy::LogStreamName,
        });

        let mut streams = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self
                .inner
                .describe_log_streams()
                .log_group_name(&log_group_name);
            if let Some(ref p) = prefix {
                request = request.log_stream_name_prefix(p);
            }
            if let Some(ob) = sdk_order_by.clone() {
                request = request.order_by(ob);
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.limit(l - streams.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            streams.extend(output.log_streams().to_vec());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if streams.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((streams, token))
    }

    /// List metric filters, optionally scoped to a single log group. `limit`
    /// caps the total number of filters returned across all pages (default
    /// unlimited) and resumes from `next_token`. `DescribeMetricFilters` has
    /// both `limit` and `next_token` (verified against pinned
    /// `aws-sdk-cloudwatchlogs` 1.139.0's `operation/describe_metric_filters/
    /// _describe_metric_filters_input.rs`), so `limit` is capped to the
    /// remaining budget on the request itself.
    pub async fn describe_metric_filters(
        &self,
        log_group_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_cloudwatchlogs::types::MetricFilter>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut filters = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self.inner.describe_metric_filters();
            if let Some(ref name) = log_group_name {
                request = request.log_group_name(name);
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.limit(l - filters.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            filters.extend(output.metric_filters().to_vec());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if filters.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((filters, token))
    }

    /// Search log events in a log group (optionally scoped to one stream and
    /// a filter pattern/time range). `limit` caps the total number of events
    /// returned across all pages (default unlimited) and resumes from
    /// `next_token`. `FilterLogEvents` has both `limit` and `next_token`
    /// (verified against pinned `aws-sdk-cloudwatchlogs` 1.139.0's
    /// `operation/filter_log_events/_filter_log_events_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself.
    pub async fn filter_log_events(
        &self,
        log_group_name: String,
        log_stream_name: Option<String>,
        filter_pattern: Option<String>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_cloudwatchlogs::types::FilteredLogEvent>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut events = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self
                .inner
                .filter_log_events()
                .log_group_name(&log_group_name);
            if let Some(ref stream_name) = log_stream_name {
                request = request.set_log_stream_names(Some(vec![stream_name.clone()]));
            }
            if let Some(ref pattern) = filter_pattern {
                request = request.filter_pattern(pattern);
            }
            if let Some(st) = start_time {
                request = request.start_time(st);
            }
            if let Some(et) = end_time {
                request = request.end_time(et);
            }
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.limit(l - events.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            events.extend(output.events().to_vec());
            token = output.next_token().map(|t| t.to_string());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if events.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((events, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://logs.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_log_groups_returns_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"logGroups":[{"logGroupName":"/aws/lambda/foo","arn":"arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/foo:*"},{"logGroupName":"/aws/lambda/bar"}]}"#,
            ),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client.describe_log_groups(None, None, None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].log_group_name(), Some("/aws/lambda/foo"));
        assert_eq!(
            groups[0].arn(),
            Some("arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/foo:*")
        );
        assert_eq!(groups[1].log_group_name(), Some("/aws/lambda/bar"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_log_groups_passes_prefix_and_resumes_from_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"logGroupNamePrefix":"/aws/lambda","nextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"logGroups":[{"logGroupName":"/aws/lambda/baz"}]}"#),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .describe_log_groups(
                Some("/aws/lambda".to_string()),
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].log_group_name(), Some("/aws/lambda/baz"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_log_groups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"limit":2}"#),
            json_response(
                200,
                r#"{"logGroups":[{"logGroupName":"g1"},{"logGroupName":"g2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .describe_log_groups(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_log_groups_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"limit":10}"#),
                json_response(
                    200,
                    r#"{"logGroups":[{"logGroupName":"g1"},{"logGroupName":"g2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"p2","limit":8}"#),
                json_response(200, r#"{"logGroups":[{"logGroupName":"g3"}]}"#),
            ),
        ]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .describe_log_groups(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_log_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("ServiceUnavailableException", "service is unavailable"),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_log_groups(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ServiceUnavailableException".to_string()));
                assert_eq!(message, "service is unavailable");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_log_streams_returns_all_streams_ordered_by_last_event_time() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"logGroupName":"my-group","orderBy":"LastEventTime"}"#,
            ),
            json_response(
                200,
                r#"{"logStreams":[{"logStreamName":"stream-1","lastEventTimestamp":1700000000000}]}"#,
            ),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (streams, token) = client
            .describe_log_streams(
                "my-group".to_string(),
                None,
                Some("LastEventTime".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].log_stream_name(), Some("stream-1"));
        assert_eq!(streams[0].last_event_timestamp(), Some(1_700_000_000_000));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_log_streams_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"logGroupName":"my-group","logStreamNamePrefix":"prod-","limit":1}"#,
            ),
            json_response(
                200,
                r#"{"logStreams":[{"logStreamName":"prod-1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (streams, token) = client
            .describe_log_streams(
                "my-group".to_string(),
                Some("prod-".to_string()),
                None,
                Some(1),
                None,
            )
            .await
            .unwrap();

        assert_eq!(streams.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_log_streams_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"logGroupName":"missing-group"}"#),
            json_error_response("ResourceNotFoundException", "log group does not exist"),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_log_streams("missing-group".to_string(), None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "log group does not exist");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_metric_filters_returns_all_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"logGroupName":"/aws/cloudtrail"}"#),
            json_response(
                200,
                r#"{"metricFilters":[{"filterName":"root-usage","filterPattern":"{ $.userIdentity.type = \"Root\" }","logGroupName":"/aws/cloudtrail"}]}"#,
            ),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (filters, token) = client
            .describe_metric_filters(Some("/aws/cloudtrail".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].filter_name(), Some("root-usage"));
        assert_eq!(filters[0].log_group_name(), Some("/aws/cloudtrail"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_metric_filters_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"nextToken":"cursor-a","limit":1}"#),
            json_response(
                200,
                r#"{"metricFilters":[{"filterName":"f1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (filters, token) = client
            .describe_metric_filters(None, Some(1), Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(filters.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_metric_filters_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidParameterException", "invalid parameter"),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_metric_filters(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "invalid parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn filter_log_events_returns_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"logGroupName":"my-group"}"#),
            json_response(
                200,
                r#"{"events":[{"logStreamName":"stream-1","timestamp":1700000000000,"message":"hello"}]}"#,
            ),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (events, token) = client
            .filter_log_events("my-group".to_string(), None, None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].log_stream_name(), Some("stream-1"));
        assert_eq!(events[0].message(), Some("hello"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn filter_log_events_passes_stream_pattern_and_time_range() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"logGroupName":"my-group","logStreamNames":["stream-1"],"startTime":1700000000000,"endTime":1700003600000,"filterPattern":"ERROR"}"#,
            ),
            json_response(200, r#"{"events":[{"eventId":"e1"}]}"#),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (events, token) = client
            .filter_log_events(
                "my-group".to_string(),
                Some("stream-1".to_string()),
                Some("ERROR".to_string()),
                Some(1_700_000_000_000),
                Some(1_700_003_600_000),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id(), Some("e1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn filter_log_events_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"logGroupName":"my-group","limit":1}"#),
            json_response(200, r#"{"events":[{"eventId":"e1"}],"nextToken":"page2"}"#),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let (events, token) = client
            .filter_log_events(
                "my-group".to_string(),
                None,
                None,
                None,
                None,
                Some(1),
                None,
            )
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn filter_log_events_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"logGroupName":"missing-group"}"#),
            json_error_response("ResourceNotFoundException", "log group does not exist"),
        )]);
        let client = CloudWatchLogsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .filter_log_events(
                "missing-group".to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "log group does not exist");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
