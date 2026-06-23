use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct CloudWatchClient {
    inner: aws_sdk_cloudwatch::Client,
}

impl CloudWatchClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudwatch::Client::new(config),
        }
    }

    pub async fn list_metrics(
        &self,
        namespace: Option<String>,
        metric_name: Option<String>,
        dimensions: Option<Vec<aws_sdk_cloudwatch::types::DimensionFilter>>,
    ) -> Result<Vec<aws_sdk_cloudwatch::types::Metric>, VaporError> {
        let mut all_metrics: Vec<aws_sdk_cloudwatch::types::Metric> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.list_metrics();

            if let Some(ref ns) = namespace {
                request = request.namespace(ns);
            }
            if let Some(ref name) = metric_name {
                request = request.metric_name(name);
            }
            if let Some(ref dims) = dimensions {
                request = request.set_dimensions(Some(dims.clone()));
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_metrics.extend(output.metrics().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_metrics)
    }

    pub async fn get_metric_data(
        &self,
        metric_data_queries: Vec<aws_sdk_cloudwatch::types::MetricDataQuery>,
        start_time: aws_sdk_cloudwatch::primitives::DateTime,
        end_time: aws_sdk_cloudwatch::primitives::DateTime,
    ) -> Result<Vec<aws_sdk_cloudwatch::types::MetricDataResult>, VaporError> {
        let mut all_results: Vec<aws_sdk_cloudwatch::types::MetricDataResult> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .inner
                .get_metric_data()
                .set_metric_data_queries(Some(metric_data_queries.clone()))
                .start_time(start_time)
                .end_time(end_time);

            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_results.extend(output.metric_data_results().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_results)
    }

    pub async fn describe_alarms(
        &self,
        names: Option<Vec<String>>,
        name_prefix: Option<String>,
        state: Option<aws_sdk_cloudwatch::types::StateValue>,
    ) -> Result<Vec<aws_sdk_cloudwatch::types::MetricAlarm>, VaporError> {
        let mut all_alarms: Vec<aws_sdk_cloudwatch::types::MetricAlarm> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.inner.describe_alarms();

            if let Some(ref alarm_names) = names {
                request = request.set_alarm_names(Some(alarm_names.clone()));
            }
            if let Some(ref prefix) = name_prefix {
                request = request.alarm_name_prefix(prefix);
            }
            if let Some(ref s) = state {
                request = request.state_value(s.clone());
            }
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let output = request.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_alarms.extend(output.metric_alarms().iter().cloned());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(all_alarms)
    }
}
