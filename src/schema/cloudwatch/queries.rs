use async_graphql::{Context, Object, Result};
use aws_sdk_cloudwatch::primitives::DateTime as AwsDateTime;

use crate::aws::cloudwatch::CloudWatchClient;
use crate::aws::cloudwatch_logs::CloudWatchLogsClient;
use crate::schema::cloudwatch::types::{
    resolve_time_range, Alarm, AlarmState, DimensionFilter, LogEvent, LogGroup, LogStream, Metric,
    MetricDataQuery, MetricFilter, MetricResult, TimeRange,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CloudWatchQuery;

#[Object]
impl CloudWatchQuery {
    async fn metrics(
        &self,
        ctx: &Context<'_>,
        namespace: Option<String>,
        metric_name: Option<String>,
        dimensions: Option<Vec<DimensionFilter>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Metric>> {
        let cw = ctx.data::<CloudWatchClient>()?;
        let sdk_dims = dimensions.map(|ds| ds.iter().map(|d| d.to_sdk()).collect());
        let (results, next_token) = cw
            .list_metrics(namespace, metric_name, sdk_dims, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(Metric::from).collect(),
            next_token,
        })
    }

    async fn metric_data(
        &self,
        ctx: &Context<'_>,
        queries: Vec<MetricDataQuery>,
        time_range: TimeRange,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MetricResult>> {
        let cw = ctx.data::<CloudWatchClient>()?;
        let (start, end) = resolve_time_range(&time_range)?;
        let aws_start = AwsDateTime::from_secs(start.timestamp());
        let aws_end = AwsDateTime::from_secs(end.timestamp());
        // Build a map from query id → unit so we can annotate each datapoint.
        let unit_map: std::collections::HashMap<String, Option<String>> = queries
            .iter()
            .map(|q| (q.id.clone(), q.unit.clone()))
            .collect();
        let sdk_queries: Vec<_> = queries.iter().map(|q| q.to_sdk()).collect();
        let (results, next_token) = cw
            .get_metric_data(sdk_queries, aws_start, aws_end, limit, next_token)
            .await?;
        Ok(Page {
            items: results
                .into_iter()
                .map(|r| {
                    let unit = r
                        .id()
                        .and_then(|id| unit_map.get(id))
                        .and_then(|u| u.clone());
                    MetricResult::from_sdk(r, unit)
                })
                .collect(),
            next_token,
        })
    }

    async fn alarms(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        name_prefix: Option<String>,
        state: Option<AlarmState>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Alarm>> {
        let cw = ctx.data::<CloudWatchClient>()?;
        let sdk_state = state.map(|s| s.to_sdk());
        let (results, next_token) = cw
            .describe_alarms(names, name_prefix, sdk_state, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(Alarm::from).collect(),
            next_token,
        })
    }

    async fn log_groups(
        &self,
        ctx: &Context<'_>,
        prefix: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LogGroup>> {
        let cwl = ctx.data::<CloudWatchLogsClient>()?;
        let (results, next_token) = cwl.describe_log_groups(prefix, limit, next_token).await?;
        Ok(Page {
            items: results.into_iter().map(LogGroup::from).collect(),
            next_token,
        })
    }

    async fn log_streams(
        &self,
        ctx: &Context<'_>,
        log_group_name: String,
        prefix: Option<String>,
        order_by: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LogStream>> {
        let cwl = ctx.data::<CloudWatchLogsClient>()?;
        let (results, next_token) = cwl
            .describe_log_streams(log_group_name, prefix, order_by, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(LogStream::from).collect(),
            next_token,
        })
    }

    /// List CloudWatch Logs metric filters, optionally scoped to a specific log group.
    ///
    /// Critical for CIS AWS Benchmark 3.x compliance: each required control (e.g. CIS 3.1
    /// unauthorized API calls, CIS 3.3 root account usage) needs a metric filter on the
    /// CloudTrail log group that publishes to a CloudWatch metric, which in turn triggers an
    /// alarm. Without this query you cannot determine which filters exist.
    ///
    /// Workflow: `logGroups(prefix: "/aws/cloudtrail")` → `metricFilters(logGroupName)` →
    /// `alarms(namePrefix: <metric-name>)` to audit end-to-end CIS 3.x coverage.
    async fn metric_filters(
        &self,
        ctx: &Context<'_>,
        log_group_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MetricFilter>> {
        let cwl = ctx.data::<CloudWatchLogsClient>()?;
        let (results, next_token) = cwl
            .describe_metric_filters(log_group_name, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(MetricFilter::from).collect(),
            next_token,
        })
    }

    async fn log_events(
        &self,
        ctx: &Context<'_>,
        log_group_name: String,
        log_stream_name: Option<String>,
        filter_pattern: Option<String>,
        time_range: Option<TimeRange>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LogEvent>> {
        let cwl = ctx.data::<CloudWatchLogsClient>()?;
        let (start_ms, end_ms) = if let Some(tr) = time_range {
            let (start, end) = resolve_time_range(&tr)?;
            (
                Some(start.timestamp_millis()),
                Some(end.timestamp_millis()),
            )
        } else {
            (None, None)
        };
        let (results, next_token) = cwl
            .filter_log_events(
                log_group_name,
                log_stream_name,
                filter_pattern,
                start_ms,
                end_ms,
                limit,
                next_token,
            )
            .await?;
        Ok(Page {
            items: results.into_iter().map(LogEvent::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        cbor_request, cbor_response, json_response, request, sdk_config, ReplayEvent,
        StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;
    use aws_smithy_cbor::Encoder;

    // CloudWatchClient (metrics/metricData/alarms) uses Smithy RPC v2 CBOR;
    // CloudWatchLogsClient (logGroups/logStreams/metricFilters/logEvents)
    // uses plain JSON — see src/aws/cloudwatch.rs's and
    // src/aws/cloudwatch_logs.rs's own test modules for the confirmed
    // per-service protocol/endpoint details this reuses verbatim.
    const CW_ENDPOINT: &str = "https://monitoring.us-east-1.amazonaws.com";
    const LOGS_ENDPOINT: &str = "https://logs.us-east-1.amazonaws.com/";

    fn cw_uri(op: &str) -> String {
        format!("{CW_ENDPOINT}/service/GraniteServiceVersion20100801/operation/{op}")
    }

    #[tokio::test]
    async fn metrics_resolver_forwards_filters_and_maps_items() {
        let mut req = Encoder::new(Vec::new());
        req.begin_map()
            .str("Namespace")
            .str("AWS/EC2")
            .str("MetricName")
            .str("CPUUtilization")
            .end();

        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("Metrics").array(1);
        resp.begin_map()
            .str("Namespace")
            .str("AWS/EC2")
            .str("MetricName")
            .str("CPUUtilization")
            .end();
        resp.end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&cw_uri("ListMetrics"), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ metrics(namespace: "AWS/EC2", metricName: "CPUUtilization") { items { namespace metricName } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["metrics"]["items"];
        assert_eq!(items[0]["namespace"], "AWS/EC2");
        assert_eq!(items[0]["metricName"], "CPUUtilization");
        assert!(json["metrics"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn metric_data_resolver_resolves_absolute_time_range_and_maps_items() {
        let mut req = Encoder::new(Vec::new());
        req.begin_map().str("MetricDataQueries").array(1);
        req.begin_map().str("Id").str("q1").end();
        req.str("StartTime")
            .timestamp(&aws_sdk_cloudwatch::primitives::DateTime::from_secs(1_704_067_200));
        req.str("EndTime")
            .timestamp(&aws_sdk_cloudwatch::primitives::DateTime::from_secs(1_704_070_800));
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
            cbor_request(&cw_uri("GetMetricData"), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ metricData(
                    queries: [{ id: "q1" }],
                    timeRange: { startTime: "2024-01-01T00:00:00Z", endTime: "2024-01-01T01:00:00Z" }
                ) { items { id label statusCode } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["metricData"]["items"];
        assert_eq!(items[0]["id"], "q1");
        assert_eq!(items[0]["label"], "CPU Avg");
        assert_eq!(items[0]["statusCode"], "Complete");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn metric_data_resolver_propagates_invalid_time_range_as_graphql_error() {
        let http_client = StaticReplayClient::new(vec![]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchClient::new(&sdk_config(http_client)))
            .finish();

        let res = schema
            .execute(
                r#"{ metricData(queries: [{ id: "q1" }], timeRange: {}) { items { id } nextToken } }"#,
            )
            .await;

        assert!(!res.errors.is_empty(), "expected a GraphQL error for an empty TimeRange");
    }

    #[tokio::test]
    async fn alarms_resolver_forwards_filters_and_maps_items() {
        let mut req = Encoder::new(Vec::new());
        req.begin_map()
            .str("AlarmNamePrefix")
            .str("cpu-")
            .str("StateValue")
            .str("ALARM")
            .end();

        let mut resp = Encoder::new(Vec::new());
        resp.begin_map().str("MetricAlarms").array(1);
        resp.begin_map()
            .str("AlarmName")
            .str("cpu-high")
            .str("StateValue")
            .str("ALARM")
            .end();
        resp.end();

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            cbor_request(&cw_uri("DescribeAlarms"), req.into_writer()),
            cbor_response(200, resp.into_writer()),
        )]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ alarms(namePrefix: "cpu-", state: ALARM) { items { name state } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["alarms"]["items"];
        assert_eq!(items[0]["name"], "cpu-high");
        assert_eq!(items[0]["state"], "ALARM");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn log_groups_resolver_forwards_prefix_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGS_ENDPOINT, r#"{"logGroupNamePrefix":"/aws/lambda"}"#),
            json_response(
                200,
                r#"{"logGroups":[{"logGroupName":"/aws/lambda/foo","arn":"arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/foo:*"}]}"#,
            ),
        )]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchLogsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ logGroups(prefix: "/aws/lambda") { items { name arn } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["logGroups"]["items"];
        assert_eq!(items[0]["name"], "/aws/lambda/foo");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/foo:*"
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn log_streams_resolver_forwards_group_and_order_by() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                LOGS_ENDPOINT,
                r#"{"logGroupName":"my-group","orderBy":"LastEventTime"}"#,
            ),
            json_response(200, r#"{"logStreams":[{"logStreamName":"stream-1"}]}"#),
        )]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchLogsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ logStreams(logGroupName: "my-group", orderBy: "LastEventTime") { items { name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["logStreams"]["items"][0]["name"], "stream-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn metric_filters_resolver_forwards_log_group_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGS_ENDPOINT, r#"{"logGroupName":"/aws/cloudtrail"}"#),
            json_response(
                200,
                r#"{"metricFilters":[{"filterName":"root-usage","filterPattern":"{ $.userIdentity.type = \"Root\" }","logGroupName":"/aws/cloudtrail"}]}"#,
            ),
        )]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchLogsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ metricFilters(logGroupName: "/aws/cloudtrail") { items { filterName logGroupName } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["metricFilters"]["items"];
        assert_eq!(items[0]["filterName"], "root-usage");
        assert_eq!(items[0]["logGroupName"], "/aws/cloudtrail");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn log_events_resolver_forwards_log_group_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(LOGS_ENDPOINT, r#"{"logGroupName":"my-group"}"#),
            json_response(
                200,
                r#"{"events":[{"logStreamName":"stream-1","timestamp":1700000000000,"message":"hello"}]}"#,
            ),
        )]);
        let schema = build_query_schema(CloudWatchQuery)
            .data(CloudWatchLogsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ logEvents(logGroupName: "my-group") { items { message } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["logEvents"]["items"][0]["message"], "hello");
        http_client.relaxed_requests_match();
    }
}
