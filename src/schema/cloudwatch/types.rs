use async_graphql::{Enum, InputObject, SimpleObject};
use chrono::{DateTime, Duration, Utc};

use crate::error::VaporError;

// === Helpers ===

/// Convert milliseconds since Unix epoch to an ISO-8601 / RFC-3339 string.
fn ms_to_iso8601(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt: DateTime<Utc>| dt.to_rfc3339())
        .unwrap_or_default()
}

// === Enums ===

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AlarmState {
    Ok,
    Alarm,
    InsufficientData,
}

impl AlarmState {
    pub fn from_sdk(s: &aws_sdk_cloudwatch::types::StateValue) -> Self {
        match s {
            aws_sdk_cloudwatch::types::StateValue::Ok => Self::Ok,
            aws_sdk_cloudwatch::types::StateValue::Alarm => Self::Alarm,
            aws_sdk_cloudwatch::types::StateValue::InsufficientData => Self::InsufficientData,
            _ => Self::InsufficientData,
        }
    }

    pub fn to_sdk(&self) -> aws_sdk_cloudwatch::types::StateValue {
        match self {
            Self::Ok => aws_sdk_cloudwatch::types::StateValue::Ok,
            Self::Alarm => aws_sdk_cloudwatch::types::StateValue::Alarm,
            Self::InsufficientData => aws_sdk_cloudwatch::types::StateValue::InsufficientData,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ComparisonOperator {
    GreaterThanThreshold,
    GreaterThanOrEqualToThreshold,
    LessThanThreshold,
    LessThanOrEqualToThreshold,
    LessThanLowerOrGreaterThanUpperThreshold,
    LessThanLowerThreshold,
    GreaterThanUpperThreshold,
}

impl ComparisonOperator {
    pub fn from_sdk(s: &aws_sdk_cloudwatch::types::ComparisonOperator) -> Self {
        match s {
            aws_sdk_cloudwatch::types::ComparisonOperator::GreaterThanThreshold => {
                Self::GreaterThanThreshold
            }
            aws_sdk_cloudwatch::types::ComparisonOperator::GreaterThanOrEqualToThreshold => {
                Self::GreaterThanOrEqualToThreshold
            }
            aws_sdk_cloudwatch::types::ComparisonOperator::LessThanThreshold => {
                Self::LessThanThreshold
            }
            aws_sdk_cloudwatch::types::ComparisonOperator::LessThanOrEqualToThreshold => {
                Self::LessThanOrEqualToThreshold
            }
            aws_sdk_cloudwatch::types::ComparisonOperator::LessThanLowerOrGreaterThanUpperThreshold => {
                Self::LessThanLowerOrGreaterThanUpperThreshold
            }
            aws_sdk_cloudwatch::types::ComparisonOperator::LessThanLowerThreshold => {
                Self::LessThanLowerThreshold
            }
            aws_sdk_cloudwatch::types::ComparisonOperator::GreaterThanUpperThreshold => {
                Self::GreaterThanUpperThreshold
            }
            _ => Self::GreaterThanThreshold,
        }
    }
}

// === Input types ===

#[derive(InputObject)]
pub struct TimeRange {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub last_minutes: Option<i32>,
    pub last_hours: Option<i32>,
}

#[derive(InputObject)]
pub struct DimensionFilter {
    pub name: String,
    pub value: Option<String>,
}

impl DimensionFilter {
    pub fn to_sdk(&self) -> aws_sdk_cloudwatch::types::DimensionFilter {
        aws_sdk_cloudwatch::types::DimensionFilter::builder()
            .name(&self.name)
            .set_value(self.value.clone())
            .build()
    }
}

#[derive(InputObject)]
pub struct MetricDataQuery {
    pub id: String,
    pub namespace: Option<String>,
    pub metric_name: Option<String>,
    pub dimensions: Option<Vec<DimensionFilter>>,
    pub period: Option<i32>,
    pub stat: Option<String>,
    pub unit: Option<String>,
    pub label: Option<String>,
}

impl MetricDataQuery {
    pub fn to_sdk(&self) -> aws_sdk_cloudwatch::types::MetricDataQuery {
        let metric_stat =
            if let (Some(ns), Some(mn), Some(period), Some(stat)) = (
                &self.namespace,
                &self.metric_name,
                self.period,
                &self.stat,
            ) {
                let dims: Vec<aws_sdk_cloudwatch::types::Dimension> = self
                    .dimensions
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .map(|d| {
                        aws_sdk_cloudwatch::types::Dimension::builder()
                            .name(&d.name)
                            .value(d.value.as_deref().unwrap_or(""))
                            .build()
                    })
                    .collect();

                let metric = aws_sdk_cloudwatch::types::Metric::builder()
                    .namespace(ns)
                    .metric_name(mn)
                    .set_dimensions(Some(dims))
                    .build();

                Some(
                    aws_sdk_cloudwatch::types::MetricStat::builder()
                        .metric(metric)
                        .period(period)
                        .stat(stat)
                        .build(),
                )
            } else {
                None
            };

        aws_sdk_cloudwatch::types::MetricDataQuery::builder()
            .id(&self.id)
            .set_label(self.label.clone())
            .set_metric_stat(metric_stat)
            .build()
    }
}

// === Output types ===

#[derive(SimpleObject, Clone)]
pub struct Dimension {
    pub name: String,
    pub value: String,
}

impl From<aws_sdk_cloudwatch::types::Dimension> for Dimension {
    fn from(d: aws_sdk_cloudwatch::types::Dimension) -> Self {
        Self {
            name: d.name().unwrap_or("").to_string(),
            value: d.value().unwrap_or("").to_string(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Metric {
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: Vec<Dimension>,
}

impl From<aws_sdk_cloudwatch::types::Metric> for Metric {
    fn from(m: aws_sdk_cloudwatch::types::Metric) -> Self {
        Self {
            namespace: m.namespace().unwrap_or("").to_string(),
            metric_name: m.metric_name().unwrap_or("").to_string(),
            dimensions: m
                .dimensions()
                .iter()
                .map(|d| Dimension::from(d.clone()))
                .collect(),
        }
    }
}

/// A single datapoint within a metric result. `unit` is populated from the
/// originating query since `GetMetricData` does not return unit per-datapoint.
#[derive(SimpleObject, Clone)]
pub struct MetricDataPoint {
    pub timestamp: String,
    pub value: f64,
    pub unit: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct MetricResult {
    pub id: String,
    pub label: Option<String>,
    /// Complete | InternalError | PartialData | Forbidden
    pub status_code: Option<String>,
    pub data_points: Vec<MetricDataPoint>,
}

impl MetricResult {
    /// Construct from the AWS SDK type, propagating the optional `unit` from
    /// the originating `MetricDataQuery` to each datapoint.
    pub fn from_sdk(r: aws_sdk_cloudwatch::types::MetricDataResult, unit: Option<String>) -> Self {
        let timestamps = r.timestamps().to_vec();
        let values = r.values().to_vec();
        let data_points = timestamps
            .iter()
            .enumerate()
            .map(|(i, ts)| MetricDataPoint {
                timestamp: ts.to_string(),
                value: values.get(i).copied().unwrap_or(0.0),
                unit: unit.clone(),
            })
            .collect();

        Self {
            id: r.id().unwrap_or("").to_string(),
            label: r.label().map(|s| s.to_string()),
            status_code: r.status_code().map(|s| s.as_str().to_string()),
            data_points,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct AlarmMetric {
    pub namespace: Option<String>,
    pub metric_name: Option<String>,
    pub dimensions: Vec<Dimension>,
    pub period: Option<i32>,
    pub stat: Option<String>,
    pub unit: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct Alarm {
    pub name: String,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub state: AlarmState,
    /// Human-readable reason explaining the current alarm state.
    pub state_reason: Option<String>,
    /// ISO-8601 timestamp of the last state change.
    pub state_updated_timestamp: Option<String>,
    pub metric: AlarmMetric,
    pub comparison_operator: Option<ComparisonOperator>,
    pub threshold: Option<f64>,
    pub evaluation_periods: Option<i32>,
    /// "missing" | "ignore" | "breaching" | "notBreaching"
    pub treat_missing_data: Option<String>,
    /// Whether actions (SNS notifications etc.) are enabled for this alarm.
    pub actions_enabled: bool,
    /// SNS topic ARNs or other action ARNs triggered when the alarm fires.
    pub alarm_actions: Vec<String>,
    /// Actions triggered when the alarm returns to OK state.
    pub ok_actions: Vec<String>,
    /// ISO-8601 timestamp of the last alarm configuration change.
    pub updated_timestamp: Option<String>,
}

impl From<aws_sdk_cloudwatch::types::MetricAlarm> for Alarm {
    fn from(a: aws_sdk_cloudwatch::types::MetricAlarm) -> Self {
        let state = a
            .state_value()
            .map(AlarmState::from_sdk)
            .unwrap_or(AlarmState::InsufficientData);

        let metric = AlarmMetric {
            namespace: a.namespace().map(|s| s.to_string()),
            metric_name: a.metric_name().map(|s| s.to_string()),
            dimensions: a
                .dimensions()
                .iter()
                .map(|d| Dimension::from(d.clone()))
                .collect(),
            period: a.period(),
            stat: a.statistic().map(|s| s.as_str().to_string()),
            unit: a.unit().map(|s| s.as_str().to_string()),
        };

        let comparison_operator = a.comparison_operator().map(ComparisonOperator::from_sdk);

        Self {
            name: a.alarm_name().unwrap_or("").to_string(),
            arn: a.alarm_arn().map(|s| s.to_string()),
            description: a.alarm_description().map(|s| s.to_string()),
            state,
            state_reason: a.state_reason().map(|s| s.to_string()),
            state_updated_timestamp: a.state_updated_timestamp().map(|dt| dt.to_string()),
            metric,
            comparison_operator,
            threshold: a.threshold(),
            evaluation_periods: a.evaluation_periods(),
            treat_missing_data: a.treat_missing_data().map(|s| s.to_string()),
            actions_enabled: a.actions_enabled().unwrap_or(true),
            alarm_actions: a.alarm_actions().iter().map(|s| s.to_string()).collect(),
            ok_actions: a.ok_actions().iter().map(|s| s.to_string()).collect(),
            updated_timestamp: a
                .alarm_configuration_updated_timestamp()
                .map(|dt| dt.to_string()),
        }
    }
}

// === CloudWatch Logs types ===

#[derive(SimpleObject, Clone)]
pub struct LogGroup {
    pub name: String,
    pub arn: Option<String>,
    /// ISO-8601 timestamp of when this log group was created.
    pub creation_time: Option<String>,
    pub retention_in_days: Option<i32>,
    pub stored_bytes: Option<i64>,
    /// KMS key ARN used to encrypt this log group, if any.
    pub kms_key_id: Option<String>,
}

impl From<aws_sdk_cloudwatchlogs::types::LogGroup> for LogGroup {
    fn from(g: aws_sdk_cloudwatchlogs::types::LogGroup) -> Self {
        Self {
            name: g.log_group_name().unwrap_or("").to_string(),
            arn: g.arn().map(|s| s.to_string()),
            creation_time: g.creation_time().map(ms_to_iso8601),
            retention_in_days: g.retention_in_days(),
            stored_bytes: g.stored_bytes(),
            kms_key_id: g.kms_key_id().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LogStream {
    pub name: String,
    pub arn: Option<String>,
    /// ISO-8601 timestamp of when this log stream was created.
    pub creation_time: Option<String>,
    /// ISO-8601 timestamp of the last log event in this stream.
    pub last_event_time: Option<String>,
    /// ISO-8601 timestamp of the last log ingestion into this stream.
    pub last_ingestion_time: Option<String>,
    pub stored_bytes: Option<i64>,
}

impl From<aws_sdk_cloudwatchlogs::types::LogStream> for LogStream {
    fn from(s: aws_sdk_cloudwatchlogs::types::LogStream) -> Self {
        Self {
            name: s.log_stream_name().unwrap_or("").to_string(),
            arn: s.arn().map(|n| n.to_string()),
            creation_time: s.creation_time().map(ms_to_iso8601),
            last_event_time: s.last_event_timestamp().map(ms_to_iso8601),
            last_ingestion_time: s.last_ingestion_time().map(ms_to_iso8601),
            stored_bytes: s.stored_bytes(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LogEvent {
    /// ISO-8601 timestamp of the log event.
    pub timestamp: String,
    pub message: String,
    /// ISO-8601 timestamp of when the event was ingested.
    pub ingestion_time: Option<String>,
}

impl From<aws_sdk_cloudwatchlogs::types::FilteredLogEvent> for LogEvent {
    fn from(e: aws_sdk_cloudwatchlogs::types::FilteredLogEvent) -> Self {
        Self {
            timestamp: e.timestamp().map(ms_to_iso8601).unwrap_or_default(),
            message: e.message().unwrap_or("").to_string(),
            ingestion_time: e.ingestion_time().map(ms_to_iso8601),
        }
    }
}

// === CloudWatch Logs Metric Filter types ===

/// A single metric transformation within a metric filter — maps log events to
/// a CloudWatch metric. Most filters have exactly one transformation.
#[derive(SimpleObject, Clone)]
pub struct MetricTransformation {
    /// The CloudWatch metric name this filter publishes to.
    pub metric_name: String,
    /// The CloudWatch namespace this filter publishes to.
    pub metric_namespace: String,
    /// The value published to the metric when a filter match occurs (often "1").
    pub metric_value: String,
    /// The value emitted when no matching log events are found in the filter period.
    pub default_value: Option<f64>,
    /// The unit of measure for the metric (e.g. "Count").
    pub unit: Option<String>,
}

impl From<aws_sdk_cloudwatchlogs::types::MetricTransformation> for MetricTransformation {
    fn from(t: aws_sdk_cloudwatchlogs::types::MetricTransformation) -> Self {
        Self {
            metric_name: t.metric_name().to_string(),
            metric_namespace: t.metric_namespace().to_string(),
            metric_value: t.metric_value().to_string(),
            default_value: t.default_value(),
            unit: t.unit().map(|u| u.as_str().to_string()),
        }
    }
}

/// A CloudWatch Logs metric filter — watches a log group for a pattern and
/// publishes matching event counts to a CloudWatch metric.
///
/// Critical for CIS AWS Benchmark 3.x: required filters monitor CloudTrail
/// log groups for unauthorized API calls, root account usage, IAM changes, etc.
/// Workflow: `logGroups` → `metricFilters(logGroupName)` → `alarms` (by metric name).
#[derive(SimpleObject, Clone)]
pub struct MetricFilter {
    pub filter_name: String,
    /// The filter pattern used to match log events (CloudWatch Logs filter syntax).
    pub filter_pattern: String,
    pub log_group_name: Option<String>,
    /// ISO-8601 timestamp of when this filter was created.
    pub creation_time: Option<String>,
    /// The CloudWatch metrics this filter writes to (usually one).
    pub metric_transformations: Vec<MetricTransformation>,
}

impl From<aws_sdk_cloudwatchlogs::types::MetricFilter> for MetricFilter {
    fn from(f: aws_sdk_cloudwatchlogs::types::MetricFilter) -> Self {
        Self {
            filter_name: f.filter_name().unwrap_or("").to_string(),
            filter_pattern: f.filter_pattern().unwrap_or("").to_string(),
            log_group_name: f.log_group_name().map(|s| s.to_string()),
            creation_time: f.creation_time().map(ms_to_iso8601),
            metric_transformations: f
                .metric_transformations()
                .iter()
                .map(|t| MetricTransformation::from(t.clone()))
                .collect(),
        }
    }
}

// === Time range resolver ===

pub fn resolve_time_range(
    tr: &TimeRange,
) -> Result<(DateTime<Utc>, DateTime<Utc>), VaporError> {
    if let Some(m) = tr.last_minutes {
        if m <= 0 {
            return Err(VaporError::InvalidInput(
                "lastMinutes must be positive".to_string(),
            ));
        }
        let end = Utc::now();
        let start = end - Duration::minutes(m as i64);
        Ok((start, end))
    } else if let Some(h) = tr.last_hours {
        if h <= 0 {
            return Err(VaporError::InvalidInput(
                "lastHours must be positive".to_string(),
            ));
        }
        let end = Utc::now();
        let start = end - Duration::hours(h as i64);
        Ok((start, end))
    } else if let (Some(start_str), Some(end_str)) = (&tr.start_time, &tr.end_time) {
        let start = chrono::DateTime::parse_from_rfc3339(start_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| VaporError::InvalidInput("Invalid ISO 8601 timestamp".to_string()))?;
        let end = chrono::DateTime::parse_from_rfc3339(end_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| VaporError::InvalidInput("Invalid ISO 8601 timestamp".to_string()))?;
        if start >= end {
            return Err(VaporError::InvalidInput(
                "startTime must be before endTime".to_string(),
            ));
        }
        Ok((start, end))
    } else {
        Err(VaporError::InvalidInput(
            "TimeRange must specify either lastMinutes, lastHours, or both startTime and endTime"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AlarmState ---

    #[test]
    fn test_alarm_state_all_variants() {
        assert_eq!(
            AlarmState::from_sdk(&aws_sdk_cloudwatch::types::StateValue::Ok),
            AlarmState::Ok
        );
        assert_eq!(
            AlarmState::from_sdk(&aws_sdk_cloudwatch::types::StateValue::Alarm),
            AlarmState::Alarm
        );
        assert_eq!(
            AlarmState::from_sdk(&aws_sdk_cloudwatch::types::StateValue::InsufficientData),
            AlarmState::InsufficientData
        );

        assert!(matches!(
            AlarmState::Ok.to_sdk(),
            aws_sdk_cloudwatch::types::StateValue::Ok
        ));
        assert!(matches!(
            AlarmState::Alarm.to_sdk(),
            aws_sdk_cloudwatch::types::StateValue::Alarm
        ));
        assert!(matches!(
            AlarmState::InsufficientData.to_sdk(),
            aws_sdk_cloudwatch::types::StateValue::InsufficientData
        ));
    }

    // --- ComparisonOperator ---

    #[test]
    fn test_comparison_operator_all_variants() {
        use aws_sdk_cloudwatch::types::ComparisonOperator as SdkOp;
        assert!(matches!(
            ComparisonOperator::from_sdk(&SdkOp::GreaterThanThreshold),
            ComparisonOperator::GreaterThanThreshold
        ));
        assert!(matches!(
            ComparisonOperator::from_sdk(&SdkOp::GreaterThanOrEqualToThreshold),
            ComparisonOperator::GreaterThanOrEqualToThreshold
        ));
        assert!(matches!(
            ComparisonOperator::from_sdk(&SdkOp::LessThanThreshold),
            ComparisonOperator::LessThanThreshold
        ));
        assert!(matches!(
            ComparisonOperator::from_sdk(&SdkOp::LessThanOrEqualToThreshold),
            ComparisonOperator::LessThanOrEqualToThreshold
        ));
        assert!(matches!(
            ComparisonOperator::from_sdk(&SdkOp::LessThanLowerOrGreaterThanUpperThreshold),
            ComparisonOperator::LessThanLowerOrGreaterThanUpperThreshold
        ));
        assert!(matches!(
            ComparisonOperator::from_sdk(&SdkOp::LessThanLowerThreshold),
            ComparisonOperator::LessThanLowerThreshold
        ));
        assert!(matches!(
            ComparisonOperator::from_sdk(&SdkOp::GreaterThanUpperThreshold),
            ComparisonOperator::GreaterThanUpperThreshold
        ));
    }

    // --- Dimension ---

    #[test]
    fn test_dimension_from_sdk() {
        let sdk = aws_sdk_cloudwatch::types::Dimension::builder()
            .name("InstanceId")
            .value("i-1234567890abcdef0")
            .build();

        let dim = Dimension::from(sdk);
        assert_eq!(dim.name, "InstanceId");
        assert_eq!(dim.value, "i-1234567890abcdef0");
    }

    // --- Metric ---

    #[test]
    fn test_metric_from_sdk() {
        let dim = aws_sdk_cloudwatch::types::Dimension::builder()
            .name("InstanceId")
            .value("i-abc123")
            .build();

        let sdk = aws_sdk_cloudwatch::types::Metric::builder()
            .namespace("AWS/EC2")
            .metric_name("CPUUtilization")
            .dimensions(dim)
            .build();

        let metric = Metric::from(sdk);
        assert_eq!(metric.namespace, "AWS/EC2");
        assert_eq!(metric.metric_name, "CPUUtilization");
        assert_eq!(metric.dimensions.len(), 1);
        assert_eq!(metric.dimensions[0].name, "InstanceId");
        assert_eq!(metric.dimensions[0].value, "i-abc123");
    }

    // --- MetricResult ---

    #[test]
    fn test_metric_result_from_sdk_no_unit() {
        let t1 = aws_sdk_cloudwatch::primitives::DateTime::from_secs(1_000_000);
        let t2 = aws_sdk_cloudwatch::primitives::DateTime::from_secs(1_000_060);
        let t3 = aws_sdk_cloudwatch::primitives::DateTime::from_secs(1_000_120);

        let sdk = aws_sdk_cloudwatch::types::MetricDataResult::builder()
            .id("m1")
            .label("CPU")
            .timestamps(t1)
            .timestamps(t2)
            .timestamps(t3)
            .values(10.0)
            .values(20.0)
            .values(30.0)
            .build();

        let result = MetricResult::from_sdk(sdk, None);
        assert_eq!(result.id, "m1");
        assert_eq!(result.label, Some("CPU".to_string()));
        assert_eq!(result.data_points.len(), 3);
        assert_eq!(result.data_points[0].value, 10.0);
        assert_eq!(result.data_points[1].value, 20.0);
        assert_eq!(result.data_points[2].value, 30.0);
        assert!(!result.data_points[0].timestamp.is_empty());
        assert!(result.data_points[0].unit.is_none());
    }

    #[test]
    fn test_metric_result_from_sdk_with_unit() {
        let t1 = aws_sdk_cloudwatch::primitives::DateTime::from_secs(1_000_000);

        let sdk = aws_sdk_cloudwatch::types::MetricDataResult::builder()
            .id("m2")
            .label("Mem")
            .timestamps(t1)
            .values(42.0)
            .build();

        let result = MetricResult::from_sdk(sdk, Some("Percent".to_string()));
        assert_eq!(result.data_points.len(), 1);
        assert_eq!(result.data_points[0].unit, Some("Percent".to_string()));
    }

    // --- Alarm ---

    #[test]
    fn test_alarm_from_sdk() {
        let dim = aws_sdk_cloudwatch::types::Dimension::builder()
            .name("InstanceId")
            .value("i-abc")
            .build();

        let sdk = aws_sdk_cloudwatch::types::MetricAlarm::builder()
            .alarm_name("high-cpu")
            .alarm_arn("arn:aws:cloudwatch:us-east-1:123456789012:alarm:high-cpu")
            .alarm_description("CPU usage above 80%")
            .state_value(aws_sdk_cloudwatch::types::StateValue::Alarm)
            .state_reason("Threshold crossed")
            .namespace("AWS/EC2")
            .metric_name("CPUUtilization")
            .dimensions(dim)
            .comparison_operator(
                aws_sdk_cloudwatch::types::ComparisonOperator::GreaterThanThreshold,
            )
            .threshold(80.0)
            .period(300)
            .evaluation_periods(2)
            .statistic(aws_sdk_cloudwatch::types::Statistic::Average)
            .treat_missing_data("missing")
            .actions_enabled(true)
            .alarm_actions("arn:aws:sns:us-east-1:123:my-topic")
            .ok_actions("arn:aws:sns:us-east-1:123:ok-topic")
            .build();

        let alarm = Alarm::from(sdk);
        assert_eq!(alarm.name, "high-cpu");
        assert_eq!(
            alarm.arn,
            Some("arn:aws:cloudwatch:us-east-1:123456789012:alarm:high-cpu".to_string())
        );
        assert_eq!(alarm.description, Some("CPU usage above 80%".to_string()));
        assert_eq!(alarm.state, AlarmState::Alarm);
        assert_eq!(alarm.state_reason, Some("Threshold crossed".to_string()));
        assert_eq!(alarm.metric.namespace, Some("AWS/EC2".to_string()));
        assert_eq!(alarm.metric.metric_name, Some("CPUUtilization".to_string()));
        assert_eq!(alarm.metric.dimensions.len(), 1);
        assert!(matches!(
            alarm.comparison_operator,
            Some(ComparisonOperator::GreaterThanThreshold)
        ));
        assert_eq!(alarm.threshold, Some(80.0));
        assert_eq!(alarm.metric.period, Some(300));
        assert_eq!(alarm.evaluation_periods, Some(2));
        assert_eq!(alarm.metric.stat, Some("Average".to_string()));
        assert_eq!(alarm.treat_missing_data, Some("missing".to_string()));
        assert!(alarm.actions_enabled);
        assert_eq!(alarm.alarm_actions, vec!["arn:aws:sns:us-east-1:123:my-topic"]);
        assert_eq!(alarm.ok_actions, vec!["arn:aws:sns:us-east-1:123:ok-topic"]);
    }

    #[test]
    fn test_alarm_defaults_actions_enabled() {
        let sdk = aws_sdk_cloudwatch::types::MetricAlarm::builder()
            .alarm_name("test")
            .state_value(aws_sdk_cloudwatch::types::StateValue::Ok)
            .build();

        let alarm = Alarm::from(sdk);
        // When actions_enabled is not set, defaults to true
        assert!(alarm.actions_enabled);
        assert!(alarm.alarm_actions.is_empty());
        assert!(alarm.ok_actions.is_empty());
    }

    // --- LogGroup ---

    #[test]
    fn test_log_group_from_sdk() {
        let sdk = aws_sdk_cloudwatchlogs::types::LogGroup::builder()
            .log_group_name("/aws/lambda/my-function")
            .arn("arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/my-function")
            .creation_time(1_620_000_000_000i64)
            .retention_in_days(30)
            .stored_bytes(1024i64)
            .build();

        let lg = LogGroup::from(sdk);
        assert_eq!(lg.name, "/aws/lambda/my-function");
        assert_eq!(
            lg.arn,
            Some("arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/my-function".to_string())
        );
        // creation_time should now be an ISO-8601 string, not i64
        let ct = lg.creation_time.expect("creation_time should be Some");
        assert!(ct.contains("2021"), "Expected ISO-8601 string, got: {ct}");
        assert_eq!(lg.retention_in_days, Some(30));
        assert_eq!(lg.stored_bytes, Some(1024i64));
        assert_eq!(lg.kms_key_id, None);
    }

    #[test]
    fn test_log_group_kms_key_id() {
        let sdk = aws_sdk_cloudwatchlogs::types::LogGroup::builder()
            .log_group_name("/aws/lambda/encrypted")
            .kms_key_id("arn:aws:kms:us-east-1:123456789012:key/abc-123")
            .build();

        let lg = LogGroup::from(sdk);
        assert_eq!(
            lg.kms_key_id,
            Some("arn:aws:kms:us-east-1:123456789012:key/abc-123".to_string())
        );
    }

    // --- LogStream ---

    #[test]
    fn test_log_stream_from_sdk() {
        let sdk = aws_sdk_cloudwatchlogs::types::LogStream::builder()
            .log_stream_name("2024/01/15/[$LATEST]abc123")
            .arn("arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/fn:log-stream:2024/01/15/[$LATEST]abc123")
            .creation_time(1_705_276_800_000i64)
            .first_event_timestamp(1_705_276_900_000i64)
            .last_event_timestamp(1_705_277_000_000i64)
            .last_ingestion_time(1_705_277_100_000i64)
            .stored_bytes(512i64)
            .build();

        let ls = LogStream::from(sdk);
        assert_eq!(ls.name, "2024/01/15/[$LATEST]abc123");
        assert!(ls.arn.is_some());

        // All timestamps should be ISO-8601 strings now
        let ct = ls.creation_time.expect("creation_time should be Some");
        assert!(ct.contains("2024"), "creation_time should be ISO-8601: {ct}");

        let let_ = ls.last_event_time.expect("last_event_time should be Some");
        assert!(let_.contains("2024"), "last_event_time should be ISO-8601: {let_}");

        let lit = ls.last_ingestion_time.expect("last_ingestion_time should be Some");
        assert!(lit.contains("2024"), "last_ingestion_time should be ISO-8601: {lit}");

        assert_eq!(ls.stored_bytes, Some(512i64));
    }

    // --- LogEvent ---

    #[test]
    fn test_log_event_from_sdk() {
        let sdk = aws_sdk_cloudwatchlogs::types::FilteredLogEvent::builder()
            .timestamp(1_705_276_900_000i64)
            .message("ERROR: Something went wrong")
            .ingestion_time(1_705_276_901_000i64)
            .build();

        let ev = LogEvent::from(sdk);

        assert!(ev.timestamp.contains("2024"), "timestamp should be ISO-8601: {}", ev.timestamp);

        assert_eq!(ev.message, "ERROR: Something went wrong");

        let it = ev.ingestion_time.expect("ingestion_time should be Some");
        assert!(it.contains("2024"), "ingestion_time should be ISO-8601: {it}");
    }

    // --- ms_to_iso8601 ---

    #[test]
    fn test_ms_to_iso8601() {
        // Unix epoch
        let s = ms_to_iso8601(0);
        assert_eq!(s, "1970-01-01T00:00:00+00:00");

        // 2021-05-03T00:00:00Z = 1620000000000 ms
        let s = ms_to_iso8601(1_620_000_000_000);
        assert!(s.starts_with("2021-05-03"), "got: {s}");
    }

    // --- resolve_time_range ---

    #[test]
    fn test_resolve_time_range_last_minutes() {
        let tr = TimeRange {
            last_minutes: Some(30),
            last_hours: None,
            start_time: None,
            end_time: None,
        };
        let (start, end) = resolve_time_range(&tr).expect("should succeed");
        let diff = end - start;
        assert_eq!(diff.num_minutes(), 30);
    }

    #[test]
    fn test_resolve_time_range_last_hours() {
        let tr = TimeRange {
            last_minutes: None,
            last_hours: Some(2),
            start_time: None,
            end_time: None,
        };
        let (start, end) = resolve_time_range(&tr).expect("should succeed");
        let diff = end - start;
        assert_eq!(diff.num_hours(), 2);
    }

    #[test]
    fn test_resolve_time_range_absolute() {
        let tr = TimeRange {
            last_minutes: None,
            last_hours: None,
            start_time: Some("2024-01-01T00:00:00Z".to_string()),
            end_time: Some("2024-01-01T01:00:00Z".to_string()),
        };
        let (start, end) = resolve_time_range(&tr).expect("should succeed");
        let diff = end - start;
        assert_eq!(diff.num_hours(), 1);
    }

    #[test]
    fn test_resolve_time_range_missing_fields() {
        let tr = TimeRange {
            last_minutes: None,
            last_hours: None,
            start_time: None,
            end_time: None,
        };
        let err = resolve_time_range(&tr).expect_err("should fail");
        assert!(matches!(err, VaporError::InvalidInput(_)));
    }

    #[test]
    fn test_resolve_time_range_start_after_end() {
        let tr = TimeRange {
            last_minutes: None,
            last_hours: None,
            start_time: Some("2024-01-01T02:00:00Z".to_string()),
            end_time: Some("2024-01-01T01:00:00Z".to_string()),
        };
        let err = resolve_time_range(&tr).expect_err("should fail");
        assert!(matches!(err, VaporError::InvalidInput(_)));
    }

    // --- MetricTransformation ---

    #[test]
    fn test_metric_transformation_from_sdk() {
        let sdk = aws_sdk_cloudwatchlogs::types::MetricTransformation::builder()
            .metric_name("UnauthorizedAPICalls")
            .metric_namespace("CISBenchmark")
            .metric_value("1")
            .default_value(0.0)
            .build()
            .expect("MetricTransformation requires name, namespace, and value");

        let t = MetricTransformation::from(sdk);
        assert_eq!(t.metric_name, "UnauthorizedAPICalls");
        assert_eq!(t.metric_namespace, "CISBenchmark");
        assert_eq!(t.metric_value, "1");
        assert_eq!(t.default_value, Some(0.0));
        assert!(t.unit.is_none());
    }

    // --- MetricFilter ---

    #[test]
    fn test_metric_filter_from_sdk() {
        let transformation = aws_sdk_cloudwatchlogs::types::MetricTransformation::builder()
            .metric_name("RootAccountUsage")
            .metric_namespace("CISBenchmark")
            .metric_value("1")
            .build()
            .expect("MetricTransformation requires name, namespace, and value");

        let sdk = aws_sdk_cloudwatchlogs::types::MetricFilter::builder()
            .filter_name("root-account-usage")
            .filter_pattern(
                "{ $.userIdentity.type = \"Root\" && $.userIdentity.invokedBy NOT EXISTS && $.eventType != \"AwsServiceEvent\" }",
            )
            .log_group_name("/aws/cloudtrail/logs")
            .creation_time(1_620_000_000_000i64)
            .metric_transformations(transformation)
            .build();

        let f = MetricFilter::from(sdk);
        assert_eq!(f.filter_name, "root-account-usage");
        assert!(f.filter_pattern.contains("Root"));
        assert_eq!(
            f.log_group_name,
            Some("/aws/cloudtrail/logs".to_string())
        );
        let ct = f.creation_time.expect("creation_time should be Some");
        assert!(ct.contains("2021"), "Expected ISO-8601 string, got: {ct}");
        assert_eq!(f.metric_transformations.len(), 1);
        assert_eq!(f.metric_transformations[0].metric_name, "RootAccountUsage");
    }

    #[test]
    fn test_metric_filter_no_log_group() {
        let transformation = aws_sdk_cloudwatchlogs::types::MetricTransformation::builder()
            .metric_name("SomeMetric")
            .metric_namespace("Custom")
            .metric_value("1")
            .build()
            .expect("MetricTransformation requires name, namespace, and value");

        let sdk = aws_sdk_cloudwatchlogs::types::MetricFilter::builder()
            .filter_name("some-filter")
            .filter_pattern("ERROR")
            .metric_transformations(transformation)
            .build();

        let f = MetricFilter::from(sdk);
        assert_eq!(f.filter_name, "some-filter");
        assert!(f.log_group_name.is_none());
        assert!(f.creation_time.is_none());
    }

    #[test]
    fn test_resolve_time_range_negative_minutes() {
        let tr = TimeRange {
            last_minutes: Some(-5),
            last_hours: None,
            start_time: None,
            end_time: None,
        };
        let err = resolve_time_range(&tr).expect_err("should fail");
        assert!(matches!(err, VaporError::InvalidInput(_)));
    }

    #[test]
    fn test_resolve_time_range_relative_takes_precedence() {
        // last_minutes takes precedence over absolute start/end
        let tr = TimeRange {
            last_minutes: Some(15),
            last_hours: None,
            start_time: Some("2024-01-01T00:00:00Z".to_string()),
            end_time: Some("2024-01-01T01:00:00Z".to_string()),
        };
        let (start, end) = resolve_time_range(&tr).expect("should succeed");
        let diff = end - start;
        // Should be 15 minutes, not 1 hour
        assert_eq!(diff.num_minutes(), 15);
    }
}
