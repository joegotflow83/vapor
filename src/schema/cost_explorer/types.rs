use async_graphql::SimpleObject;
use aws_sdk_costexplorer::types::{
    ForecastResult as SdkForecast, ResultByTime as SdkResultByTime,
};

#[derive(SimpleObject, Clone)]
pub struct CostAndUsageResult {
    pub time_period_start: String,
    pub time_period_end: String,
    pub groups: Vec<CostGroup>,
    pub total_amount: Option<String>,
    pub total_unit: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct CostGroup {
    pub keys: Vec<String>,
    pub amount: Option<String>,
    pub unit: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct ForecastResult {
    pub time_period_start: String,
    pub time_period_end: String,
    pub mean_value: Option<String>,
    pub prediction_interval_lower_bound: Option<String>,
    pub prediction_interval_upper_bound: Option<String>,
}

impl From<SdkResultByTime> for CostAndUsageResult {
    fn from(r: SdkResultByTime) -> Self {
        let (total_amount, total_unit) = r
            .total()
            .and_then(|m| m.get("UnblendedCost"))
            .map(|mv| {
                (
                    mv.amount().map(|a| a.to_string()),
                    mv.unit().map(|u| u.to_string()),
                )
            })
            .unwrap_or((None, None));

        let groups = r
            .groups()
            .iter()
            .map(|g| {
                let (amount, unit) = g
                    .metrics()
                    .and_then(|m| m.get("UnblendedCost"))
                    .map(|mv| {
                        (
                            mv.amount().map(|a| a.to_string()),
                            mv.unit().map(|u| u.to_string()),
                        )
                    })
                    .unwrap_or((None, None));
                CostGroup {
                    keys: g.keys().iter().map(|k| k.to_string()).collect(),
                    amount,
                    unit,
                }
            })
            .collect();

        let (start, end) = r
            .time_period()
            .map(|tp| {
                (
                    tp.start().to_string(),
                    tp.end().to_string(),
                )
            })
            .unwrap_or_default();

        Self {
            time_period_start: start,
            time_period_end: end,
            groups,
            total_amount,
            total_unit,
        }
    }
}

impl From<SdkForecast> for ForecastResult {
    fn from(f: SdkForecast) -> Self {
        let (start, end) = f
            .time_period()
            .map(|tp| (tp.start().to_string(), tp.end().to_string()))
            .unwrap_or_default();

        Self {
            time_period_start: start,
            time_period_end: end,
            mean_value: f.mean_value().map(|v| v.to_string()),
            prediction_interval_lower_bound: f
                .prediction_interval_lower_bound()
                .map(|v| v.to_string()),
            prediction_interval_upper_bound: f
                .prediction_interval_upper_bound()
                .map(|v| v.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_costexplorer::types::{DateInterval, Group, MetricValue};
    use std::collections::HashMap;

    #[test]
    fn test_cost_and_usage_result_from_sdk() {
        let mut total_map = HashMap::new();
        total_map.insert(
            "UnblendedCost".to_string(),
            MetricValue::builder()
                .amount("123.45")
                .unit("USD")
                .build(),
        );

        let result = SdkResultByTime::builder()
            .time_period(
                DateInterval::builder()
                    .start("2024-01-01")
                    .end("2024-01-02")
                    .build()
                    .unwrap(),
            )
            .set_total(Some(total_map))
            .build();

        let converted = CostAndUsageResult::from(result);
        assert_eq!(converted.time_period_start, "2024-01-01");
        assert_eq!(converted.time_period_end, "2024-01-02");
        assert_eq!(converted.total_amount, Some("123.45".to_string()));
        assert_eq!(converted.total_unit, Some("USD".to_string()));
        assert!(converted.groups.is_empty());
    }

    #[test]
    fn test_cost_and_usage_result_with_groups() {
        let mut metrics = HashMap::new();
        metrics.insert(
            "UnblendedCost".to_string(),
            MetricValue::builder()
                .amount("50.00")
                .unit("USD")
                .build(),
        );

        let group = Group::builder()
            .keys("Amazon EC2")
            .set_metrics(Some(metrics))
            .build();

        let result = SdkResultByTime::builder()
            .time_period(
                DateInterval::builder()
                    .start("2024-01-01")
                    .end("2024-02-01")
                    .build()
                    .unwrap(),
            )
            .groups(group)
            .build();

        let converted = CostAndUsageResult::from(result);
        assert_eq!(converted.groups.len(), 1);
        assert_eq!(converted.groups[0].keys, vec!["Amazon EC2".to_string()]);
        assert_eq!(converted.groups[0].amount, Some("50.00".to_string()));
    }

    #[test]
    fn test_forecast_result_from_sdk() {
        let forecast = SdkForecast::builder()
            .time_period(
                DateInterval::builder()
                    .start("2024-02-01")
                    .end("2024-02-02")
                    .build()
                    .unwrap(),
            )
            .mean_value("100.00")
            .prediction_interval_lower_bound("80.00")
            .prediction_interval_upper_bound("120.00")
            .build();

        let converted = ForecastResult::from(forecast);
        assert_eq!(converted.time_period_start, "2024-02-01");
        assert_eq!(converted.time_period_end, "2024-02-02");
        assert_eq!(converted.mean_value, Some("100.00".to_string()));
        assert_eq!(
            converted.prediction_interval_lower_bound,
            Some("80.00".to_string())
        );
        assert_eq!(
            converted.prediction_interval_upper_bound,
            Some("120.00".to_string())
        );
    }

    #[test]
    fn test_forecast_result_no_bounds() {
        let forecast = SdkForecast::builder()
            .time_period(
                DateInterval::builder()
                    .start("2024-03-01")
                    .end("2024-03-02")
                    .build()
                    .unwrap(),
            )
            .mean_value("200.00")
            .build();

        let converted = ForecastResult::from(forecast);
        assert_eq!(converted.mean_value, Some("200.00".to_string()));
        assert!(converted.prediction_interval_lower_bound.is_none());
        assert!(converted.prediction_interval_upper_bound.is_none());
    }
}
