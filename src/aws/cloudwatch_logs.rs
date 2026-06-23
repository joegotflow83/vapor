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

    pub async fn describe_log_groups(
        &self,
        prefix: Option<String>,
    ) -> Result<Vec<aws_sdk_cloudwatchlogs::types::LogGroup>, VaporError> {
        let mut all_groups: Vec<aws_sdk_cloudwatchlogs::types::LogGroup> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_log_groups();

            if let Some(ref p) = prefix {
                request = request.log_group_name_prefix(p);
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_groups.extend(output.log_groups().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_groups)
    }

    pub async fn describe_log_streams(
        &self,
        log_group_name: String,
        prefix: Option<String>,
        order_by: Option<String>,
    ) -> Result<Vec<aws_sdk_cloudwatchlogs::types::LogStream>, VaporError> {
        let mut all_streams: Vec<aws_sdk_cloudwatchlogs::types::LogStream> = Vec::new();
        let mut next_token: Option<String> = None;

        let sdk_order_by = order_by.as_deref().map(|s| match s {
            "LastEventTime" => OrderBy::LastEventTime,
            _ => OrderBy::LogStreamName,
        });

        loop {
            let mut request = self
                .inner
                .describe_log_streams()
                .log_group_name(&log_group_name);

            if let Some(ref p) = prefix {
                request = request.log_stream_name_prefix(p);
            }
            if let Some(ref ob) = sdk_order_by {
                request = request.order_by(ob.clone());
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_streams.extend(output.log_streams().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_streams)
    }

    pub async fn describe_metric_filters(
        &self,
        log_group_name: Option<String>,
    ) -> Result<Vec<aws_sdk_cloudwatchlogs::types::MetricFilter>, VaporError> {
        let mut all_filters: Vec<aws_sdk_cloudwatchlogs::types::MetricFilter> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_metric_filters();

            if let Some(ref name) = log_group_name {
                request = request.log_group_name(name);
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_filters.extend(output.metric_filters().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_filters)
    }

    pub async fn filter_log_events(
        &self,
        log_group_name: String,
        log_stream_name: Option<String>,
        filter_pattern: Option<String>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: Option<i32>,
    ) -> Result<Vec<aws_sdk_cloudwatchlogs::types::FilteredLogEvent>, VaporError> {
        let mut all_events: Vec<aws_sdk_cloudwatchlogs::types::FilteredLogEvent> = Vec::new();
        let mut next_token: Option<String> = None;

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
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_events.extend(output.events().iter().cloned());

            if let Some(lim) = limit {
                if all_events.len() >= lim as usize {
                    all_events.truncate(lim as usize);
                    break;
                }
            }

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_events)
    }
}
