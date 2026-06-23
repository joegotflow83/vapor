use async_graphql::{Context, Object, Result};
use aws_sdk_cloudwatch::primitives::DateTime as AwsDateTime;

use crate::aws::cloudwatch::CloudWatchClient;
use crate::aws::cloudwatch_logs::CloudWatchLogsClient;
use crate::schema::cloudwatch::types::{
    resolve_time_range, Alarm, AlarmState, DimensionFilter, LogEvent, LogGroup, LogStream, Metric,
    MetricDataQuery, MetricFilter, MetricResult, TimeRange,
};

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
    ) -> Result<Vec<Metric>> {
        let cw = ctx.data::<CloudWatchClient>()?;
        let sdk_dims = dimensions.map(|ds| ds.iter().map(|d| d.to_sdk()).collect());
        let results = cw.list_metrics(namespace, metric_name, sdk_dims).await?;
        Ok(results.into_iter().map(Metric::from).collect())
    }

    async fn metric_data(
        &self,
        ctx: &Context<'_>,
        queries: Vec<MetricDataQuery>,
        time_range: TimeRange,
    ) -> Result<Vec<MetricResult>> {
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
        let results = cw.get_metric_data(sdk_queries, aws_start, aws_end).await?;
        Ok(results
            .into_iter()
            .map(|r| {
                let unit = r
                    .id()
                    .and_then(|id| unit_map.get(id))
                    .and_then(|u| u.clone());
                MetricResult::from_sdk(r, unit)
            })
            .collect())
    }

    async fn alarms(
        &self,
        ctx: &Context<'_>,
        names: Option<Vec<String>>,
        name_prefix: Option<String>,
        state: Option<AlarmState>,
    ) -> Result<Vec<Alarm>> {
        let cw = ctx.data::<CloudWatchClient>()?;
        let sdk_state = state.map(|s| s.to_sdk());
        let results = cw.describe_alarms(names, name_prefix, sdk_state).await?;
        Ok(results.into_iter().map(Alarm::from).collect())
    }

    async fn log_groups(
        &self,
        ctx: &Context<'_>,
        prefix: Option<String>,
    ) -> Result<Vec<LogGroup>> {
        let cwl = ctx.data::<CloudWatchLogsClient>()?;
        let results = cwl.describe_log_groups(prefix).await?;
        Ok(results.into_iter().map(LogGroup::from).collect())
    }

    async fn log_streams(
        &self,
        ctx: &Context<'_>,
        log_group_name: String,
        prefix: Option<String>,
        order_by: Option<String>,
    ) -> Result<Vec<LogStream>> {
        let cwl = ctx.data::<CloudWatchLogsClient>()?;
        let results = cwl
            .describe_log_streams(log_group_name, prefix, order_by)
            .await?;
        Ok(results.into_iter().map(LogStream::from).collect())
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
    ) -> Result<Vec<MetricFilter>> {
        let cwl = ctx.data::<CloudWatchLogsClient>()?;
        let results = cwl.describe_metric_filters(log_group_name).await?;
        Ok(results.into_iter().map(MetricFilter::from).collect())
    }

    async fn log_events(
        &self,
        ctx: &Context<'_>,
        log_group_name: String,
        log_stream_name: Option<String>,
        filter_pattern: Option<String>,
        time_range: Option<TimeRange>,
        limit: Option<i32>,
    ) -> Result<Vec<LogEvent>> {
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
        let results = cwl
            .filter_log_events(
                log_group_name,
                log_stream_name,
                filter_pattern,
                start_ms,
                end_ms,
                limit,
            )
            .await?;
        Ok(results.into_iter().map(LogEvent::from).collect())
    }
}
