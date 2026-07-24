use aws_config::SdkConfig;

use crate::aws::pagination::apply_limit;
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

    /// List CloudWatch metrics matching the given filters, optionally capped
    /// at `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListMetrics` has no `max_results`-equivalent input field, so `limit`
    /// is enforced purely client-side via `apply_limit` truncation (same
    /// no-server-side-cap shape as `cloudformation::list_exports`).
    pub async fn list_metrics(
        &self,
        namespace: Option<String>,
        metric_name: Option<String>,
        dimensions: Option<Vec<aws_sdk_cloudwatch::types::DimensionFilter>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_cloudwatch::types::Metric>, Option<String>), VaporError> {
        let mut all_metrics: Vec<aws_sdk_cloudwatch::types::Metric> = Vec::new();
        let mut token = next_token;

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
            if let Some(ref t) = token {
                request = request.next_token(t);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_metrics.extend(output.metrics.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut all_metrics, limit) || token.is_none() {
                break;
            }
        }

        Ok((all_metrics, token))
    }

    /// Fetch metric data points for the given queries, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `GetMetricData` has no `max_results`-equivalent input field (its
    /// `MaxDatapoints` governs per-query datapoint count, not result-page
    /// size), so `limit` is enforced purely client-side.
    pub async fn get_metric_data(
        &self,
        metric_data_queries: Vec<aws_sdk_cloudwatch::types::MetricDataQuery>,
        start_time: aws_sdk_cloudwatch::primitives::DateTime,
        end_time: aws_sdk_cloudwatch::primitives::DateTime,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_cloudwatch::types::MetricDataResult>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut all_results: Vec<aws_sdk_cloudwatch::types::MetricDataResult> = Vec::new();
        let mut token = next_token;

        loop {
            let mut request = self
                .inner
                .get_metric_data()
                .set_metric_data_queries(Some(metric_data_queries.clone()))
                .start_time(start_time)
                .end_time(end_time);
            if let Some(ref t) = token {
                request = request.next_token(t);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_results.extend(output.metric_data_results.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut all_results, limit) || token.is_none() {
                break;
            }
        }

        Ok((all_results, token))
    }

    /// Describe CloudWatch alarms matching the given filters, optionally
    /// capped at `limit` results (default unlimited) and resumed from
    /// `next_token`. `DescribeAlarms` has both `max_records`/`next_token`
    /// (verified against pinned `aws-sdk-cloudwatch` 1.116.0), so `limit` is
    /// capped to the remaining budget on the request itself.
    pub async fn describe_alarms(
        &self,
        names: Option<Vec<String>>,
        name_prefix: Option<String>,
        state: Option<aws_sdk_cloudwatch::types::StateValue>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_cloudwatch::types::MetricAlarm>, Option<String>), VaporError> {
        let mut all_alarms: Vec<aws_sdk_cloudwatch::types::MetricAlarm> = Vec::new();
        let mut token = next_token;

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
            if let Some(ref t) = token {
                request = request.next_token(t);
            }
            if let Some(l) = limit {
                request = request.max_records(l - all_alarms.len() as i32);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_alarms.extend(output.metric_alarms.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut all_alarms, limit) || token.is_none() {
                break;
            }
        }

        Ok((all_alarms, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        cbor_error_response, cbor_request, cbor_response, sdk_config, ReplayEvent,
        StaticReplayClient,
    };
    use aws_sdk_cloudwatch::primitives::DateTime as AwsDateTime;
    use aws_sdk_cloudwatch::types::{DimensionFilter, MetricDataQuery, StateValue};
    use aws_smithy_cbor::Encoder;

    // `aws-sdk-cloudwatch` 1.116.0 uses Smithy RPC v2 CBOR (`rpc-v2-cbor`,
    // verified against pinned `operation/*.rs`'s `ListMetricsRequestSerializer`
    // — `Content-Type: application/cbor`, `smithy-protocol: rpc-v2-cbor`, POST
    // to `/service/GraniteServiceVersion20100801/operation/<Op>`), not
    // restJson1/awsJson1.1/ec2Query like every other file in this sweep so
    // far. Bodies are binary CBOR, not JSON text — `StaticReplayClient` falls
    // back to exact byte-for-byte comparison for non-JSON content types, so
    // every expected request body below is built with the same
    // `aws_smithy_cbor::Encoder` the SDK itself uses, mirroring each op's
    // pinned `ser_*_input` codegen field order exactly (confirmed
    // field-order-only-matters-for-requests; response map field order is
    // irrelevant since the decoder matches by key name). Error bodies use the
    // same `__type`/`message` map shape as awsJson1.1, just CBOR-encoded
    // (`cbor_errors::parse_error_metadata`, shared with the
    // awsQueryCompatible `x-amzn-query-error` header path this file doesn't
    // exercise). Endpoint is the global-looking-but-regional host
    // `https://monitoring.us-east-1.amazonaws.com` (verified against pinned
    // `config/endpoint.rs`'s us-east-1 fixture — `monitoring` not
    // `cloudwatch`, a legacy service-name mismatch).
    const ENDPOINT: &str = "https://monitoring.us-east-1.amazonaws.com";

    fn list_metrics_uri() -> String {
        format!("{ENDPOINT}/service/GraniteServiceVersion20100801/operation/ListMetrics")
    }
    fn get_metric_data_uri() -> String {
        format!("{ENDPOINT}/service/GraniteServiceVersion20100801/operation/GetMetricData")
    }
    fn describe_alarms_uri() -> String {
        format!("{ENDPOINT}/service/GraniteServiceVersion20100801/operation/DescribeAlarms")
    }

    fn empty_map() -> Vec<u8> {
        let mut enc = Encoder::new(Vec::new());
        enc.begin_map().end();
        enc.into_writer()
    }

    #[tokio::test]
    async fn list_metrics_lists_all_when_no_limit() {
        // Exercises the Namespace/MetricName/Dimensions field mapping in the
        // same case as the happy path (ListMetrics has no MaxResults-
        // equivalent field, so this is the only request-shape case besides
        // NextToken passthrough).
        let mut req = Encoder::new(Vec::new());
        req.begin_map()
            .str("Namespace")
            .str("AWS/EC2")
            .str("MetricName")
            .str("CPUUtilization")
            .str("Dimensions")
            .array(1);
        req.begin_map()
            .str("Name")
            .str("InstanceId")
            .str("Value")
            .str("i-123")
            .end();
        req.end();

        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("Metrics").array(1);
        resp.begin_map()
            .str("Namespace")
            .str("AWS/EC2")
            .str("MetricName")
            .str("CPUUtilization")
            .str("Dimensions")
            .array(1);
        resp.begin_map()
            .str("Name")
            .str("InstanceId")
            .str("Value")
            .str("i-123")
            .end();
        resp.end();
        resp.end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&list_metrics_uri(), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let dims = vec![DimensionFilter::builder()
            .name("InstanceId")
            .value("i-123")
            .build()];
        let (metrics, token) = client
            .list_metrics(
                Some("AWS/EC2".to_string()),
                Some("CPUUtilization".to_string()),
                Some(dims),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].namespace(), Some("AWS/EC2"));
        assert_eq!(metrics[0].metric_name(), Some("CPUUtilization"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_metrics_resumes_from_provided_next_token() {
        let mut req = Encoder::new(Vec::new());
        req.begin_map().str("NextToken").str("cursor-a").end();

        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("Metrics").array(0).end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&list_metrics_uri(), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let (metrics, token) = client
            .list_metrics(None, None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(metrics.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_metrics_stops_at_limit_and_returns_resume_token() {
        // ListMetrics has no MaxResults-equivalent field, so `limit` never
        // shows up in the request body — only one page is fetched since
        // `apply_limit` trips immediately on a 2-item first page.
        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("Metrics").array(2);
        resp.begin_map().str("MetricName").str("m1").end();
        resp.begin_map().str("MetricName").str("m2").end();
        resp.str("NextToken").str("page2-token");
        resp.end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&list_metrics_uri(), empty_map()),
            cbor_response(200, resp.into_writer()),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let (metrics, token) = client
            .list_metrics(None, None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(metrics.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_metrics_pages_through_until_exhausted_when_limit_not_reached() {
        let mut resp1 = Encoder::new(Vec::new());
        resp1.begin_map().str("Metrics").array(1);
        resp1.begin_map().str("MetricName").str("m1").end();
        resp1.str("NextToken").str("p2");
        resp1.end();

        let mut req2 = Encoder::new(Vec::new());
        req2.begin_map().str("NextToken").str("p2").end();

        let mut resp2 = Encoder::new(Vec::new());
        resp2.begin_map().str("Metrics").array(1);
        resp2.begin_map().str("MetricName").str("m2").end();
        resp2.end();

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                cbor_request(&list_metrics_uri(), empty_map()),
                cbor_response(200, resp1.into_writer()),
            ),
            ReplayEvent::new(
                cbor_request(&list_metrics_uri(), req2.into_writer()),
                cbor_response(200, resp2.into_writer()),
            ),
        ]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let (metrics, token) = client
            .list_metrics(None, None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(metrics.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_metrics_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&list_metrics_uri(), empty_map()),
            cbor_error_response("InvalidParameterValueException", "bad namespace"),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let result = client.list_metrics(None, None, None, None, None).await;

        match result {
            Ok(_) => panic!("expected an error, got Ok"),
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad namespace");
            }
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_metric_data_lists_all_when_no_limit() {
        let query = MetricDataQuery::builder()
            .id("q1")
            .expression("SELECT AVG(CPUUtilization)")
            .build();

        let mut req = Encoder::new(Vec::new());
        req.begin_map().str("MetricDataQueries").array(1);
        req.begin_map()
            .str("Id")
            .str("q1")
            .str("Expression")
            .str("SELECT AVG(CPUUtilization)")
            .end();
        req.str("StartTime")
            .timestamp(&AwsDateTime::from_secs(1_700_000_000));
        req.str("EndTime")
            .timestamp(&AwsDateTime::from_secs(1_700_003_600));
        req.end();

        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("MetricDataResults").array(1);
        resp.begin_map()
            .str("Id")
            .str("q1")
            .str("Label")
            .str("CPU Avg")
            .str("StatusCode")
            .str("Complete")
            .end();
        resp.end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&get_metric_data_uri(), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .get_metric_data(
                vec![query],
                AwsDateTime::from_secs(1_700_000_000),
                AwsDateTime::from_secs(1_700_003_600),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id(), Some("q1"));
        assert_eq!(results[0].label(), Some("CPU Avg"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_metric_data_stops_at_limit_and_returns_resume_token() {
        // Same no-MaxResults-equivalent-field shape as list_metrics — `limit`
        // never appears in the request body, only one page is fetched.
        let query = MetricDataQuery::builder()
            .id("q1")
            .expression("SELECT SUM(Errors)")
            .build();

        let mut req = Encoder::new(Vec::new());
        req.begin_map().str("MetricDataQueries").array(1);
        req.begin_map()
            .str("Id")
            .str("q1")
            .str("Expression")
            .str("SELECT SUM(Errors)")
            .end();
        req.str("StartTime")
            .timestamp(&AwsDateTime::from_secs(1_700_000_000));
        req.str("EndTime")
            .timestamp(&AwsDateTime::from_secs(1_700_003_600));
        req.end();

        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("MetricDataResults").array(2);
        resp.begin_map().str("Id").str("q1").end();
        resp.begin_map().str("Id").str("q2").end();
        resp.str("NextToken").str("page2-token");
        resp.end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&get_metric_data_uri(), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .get_metric_data(
                vec![query],
                AwsDateTime::from_secs(1_700_000_000),
                AwsDateTime::from_secs(1_700_003_600),
                Some(2),
                None,
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_alarms_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&describe_alarms_uri(), empty_map()),
            {
                let mut resp = Encoder::new(Vec::new());
                resp.begin_map().str("MetricAlarms").array(1);
                resp.begin_map()
                    .str("AlarmName")
                    .str("cpu-high")
                    .str("StateValue")
                    .str("ALARM")
                    .str("MetricName")
                    .str("CPUUtilization")
                    .str("Namespace")
                    .str("AWS/EC2")
                    .str("ComparisonOperator")
                    .str("GreaterThanThreshold")
                    .str("Threshold")
                    .double(80.0)
                    .end();
                resp.end();
                cbor_response(200, resp.into_writer())
            },
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let (alarms, token) = client
            .describe_alarms(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(alarms.len(), 1);
        assert_eq!(alarms[0].alarm_name(), Some("cpu-high"));
        assert_eq!(alarms[0].state_value(), Some(&StateValue::Alarm));
        assert_eq!(alarms[0].threshold, Some(80.0));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_alarms_stops_at_limit_and_returns_resume_token() {
        // Unlike list_metrics/get_metric_data, DescribeAlarms has a real
        // MaxRecords field (verified against pinned
        // `operation/describe_alarms/_describe_alarms_input.rs`) — the
        // wrapper sets it to the remaining budget on every request.
        let mut req = Encoder::new(Vec::new());
        req.begin_map()
            .str("AlarmNamePrefix")
            .str("cpu-")
            .str("StateValue")
            .str("ALARM")
            .str("MaxRecords")
            .integer(2)
            .end();

        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("MetricAlarms").array(2);
        resp.begin_map().str("AlarmName").str("cpu-high").end();
        resp.begin_map().str("AlarmName").str("cpu-low").end();
        resp.str("NextToken").str("page2-token");
        resp.end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&describe_alarms_uri(), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let (alarms, token) = client
            .describe_alarms(
                None,
                Some("cpu-".to_string()),
                Some(StateValue::Alarm),
                Some(2),
                None,
            )
            .await
            .unwrap();

        assert_eq!(alarms.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_alarms_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&describe_alarms_uri(), empty_map()),
            cbor_error_response("InvalidNextToken", "token expired"),
        )]);
        let client = CloudWatchClient::new(&sdk_config(http_client.clone()));

        let result = client.describe_alarms(None, None, None, None, None).await;

        match result {
            Ok(_) => panic!("expected an error, got Ok"),
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidNextToken".to_string()));
                assert_eq!(message, "token expired");
            }
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
