use aws_config::SdkConfig;
use aws_sdk_costexplorer::types::{
    DateInterval, Granularity, GroupDefinition, GroupDefinitionType, Metric,
};

use crate::aws::pagination::apply_limit;
use crate::error::VaporError;

pub struct CostExplorerClient {
    inner: aws_sdk_costexplorer::Client,
}

impl CostExplorerClient {
    pub fn new(config: &SdkConfig) -> Self {
        let ce_config = aws_sdk_costexplorer::config::Builder::from(config)
            .region(aws_sdk_costexplorer::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_costexplorer::Client::from_conf(ce_config),
        }
    }

    /// Fetches cost and usage data, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetCostAndUsage`
    /// has no `max_results`-equivalent input (confirmed: `builders.rs` only
    /// exposes `next_page_token`, no size hint), so AWS alone decides each
    /// page's size — unlike kinesis/eventbridge, `limit` can only be enforced
    /// by truncating client-side via `apply_limit`. When that trips mid-page,
    /// the returned `next_token` is still the *next* AWS page's token, so the
    /// remainder of the current (truncated) page is permanently skipped, not
    /// resumable — this is the documented `apply_limit`-as-guard caveat from
    /// `specs/plan-2-schema-v2-pagination-timestamps.md` ("the returned token
    /// may then over-advance").
    pub async fn get_cost_and_usage(
        &self,
        start: &str,
        end: &str,
        granularity: &str,
        group_by: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_costexplorer::types::ResultByTime>,
            Option<String>,
        ),
        VaporError,
    > {
        let gran = match granularity {
            "HOURLY" => Granularity::Hourly,
            "MONTHLY" => Granularity::Monthly,
            _ => Granularity::Daily,
        };

        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .get_cost_and_usage()
                .time_period(
                    DateInterval::builder()
                        .start(start)
                        .end(end)
                        .build()
                        .map_err(|e| VaporError::AwsSdk {
                            code: None,
                            message: e.to_string(),
                        })?,
                )
                .granularity(gran.clone())
                .metrics("UnblendedCost");

            if let Some(ref groups) = group_by {
                for g in groups {
                    let (gtype, key) = if let Some(tag) = g.strip_prefix("TAG:") {
                        (GroupDefinitionType::Tag, tag.to_string())
                    } else {
                        (GroupDefinitionType::Dimension, g.clone())
                    };
                    req = req.group_by(GroupDefinition::builder().r#type(gtype).key(key).build());
                }
            }

            if let Some(ref t) = token {
                req = req.next_page_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.results_by_time().to_vec());
            token = output
                .next_page_token()
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string());

            if apply_limit(&mut items, limit) {
                break;
            }
            if token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn get_cost_forecast(
        &self,
        start: &str,
        end: &str,
        granularity: &str,
    ) -> Result<Vec<aws_sdk_costexplorer::types::ForecastResult>, VaporError> {
        let gran = match granularity {
            "HOURLY" => Granularity::Hourly,
            "MONTHLY" => Granularity::Monthly,
            _ => Granularity::Daily,
        };

        let output = self
            .inner
            .get_cost_forecast()
            .time_period(
                DateInterval::builder()
                    .start(start)
                    .end(end)
                    .build()
                    .map_err(|e| VaporError::AwsSdk {
                        code: None,
                        message: e.to_string(),
                    })?,
            )
            .granularity(gran)
            .metric(Metric::UnblendedCost)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output.forecast_results_by_time().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://ce.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn get_cost_and_usage_happy_path_with_group_by_and_tag() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Granularity":"MONTHLY","Metrics":["UnblendedCost"],"GroupBy":[{"Type":"DIMENSION","Key":"SERVICE"},{"Type":"TAG","Key":"Team"}]}"#,
            ),
            json_response(
                200,
                r#"{"ResultsByTime":[{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Total":{"UnblendedCost":{"Amount":"12.34","Unit":"USD"}},"Estimated":false}]}"#,
            ),
        )]);
        let client = CostExplorerClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .get_cost_and_usage(
                "2026-01-01",
                "2026-01-31",
                "MONTHLY",
                Some(vec!["SERVICE".to_string(), "TAG:Team".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let total = results[0].total().unwrap();
        assert_eq!(total.get("UnblendedCost").unwrap().amount(), Some("12.34"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_cost_and_usage_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TimePeriod":{"Start":"2026-02-01","End":"2026-02-28"},"Granularity":"DAILY","Metrics":["UnblendedCost"],"NextPageToken":"cursor-a"}"#,
            ),
            json_response(
                200,
                r#"{"ResultsByTime":[{"TimePeriod":{"Start":"2026-02-01","End":"2026-02-28"},"Total":{"UnblendedCost":{"Amount":"1.00","Unit":"USD"}},"Estimated":true}]}"#,
            ),
        )]);
        let client = CostExplorerClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .get_cost_and_usage(
                "2026-02-01",
                "2026-02-28",
                "DAILY",
                None,
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].estimated());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_cost_and_usage_stops_at_limit_and_returns_next_page_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Granularity":"DAILY","Metrics":["UnblendedCost"]}"#,
            ),
            json_response(
                200,
                r#"{"ResultsByTime":[
                    {"TimePeriod":{"Start":"2026-01-01","End":"2026-01-02"},"Total":{"UnblendedCost":{"Amount":"1.00","Unit":"USD"}},"Estimated":false},
                    {"TimePeriod":{"Start":"2026-01-02","End":"2026-01-03"},"Total":{"UnblendedCost":{"Amount":"2.00","Unit":"USD"}},"Estimated":false},
                    {"TimePeriod":{"Start":"2026-01-03","End":"2026-01-04"},"Total":{"UnblendedCost":{"Amount":"3.00","Unit":"USD"}},"Estimated":false}
                ],"NextPageToken":"page2-token"}"#,
            ),
        )]);
        let client = CostExplorerClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .get_cost_and_usage("2026-01-01", "2026-01-31", "DAILY", None, Some(2), None)
            .await
            .unwrap();

        // apply_limit truncates mid-page; the surfaced token is AWS's *next*
        // page token, so the 3rd item on this page is permanently skipped
        // (documented over-advance caveat on `get_cost_and_usage`).
        assert_eq!(results.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_cost_and_usage_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Granularity":"DAILY","Metrics":["UnblendedCost"]}"#,
                ),
                json_response(
                    200,
                    r#"{"ResultsByTime":[{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-02"},"Total":{"UnblendedCost":{"Amount":"1.00","Unit":"USD"}},"Estimated":false}],"NextPageToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Granularity":"DAILY","Metrics":["UnblendedCost"],"NextPageToken":"p2"}"#,
                ),
                json_response(
                    200,
                    r#"{"ResultsByTime":[{"TimePeriod":{"Start":"2026-01-02","End":"2026-01-03"},"Total":{"UnblendedCost":{"Amount":"2.00","Unit":"USD"}},"Estimated":false}]}"#,
                ),
            ),
        ]);
        let client = CostExplorerClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .get_cost_and_usage("2026-01-01", "2026-01-31", "DAILY", None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_cost_and_usage_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Granularity":"DAILY","Metrics":["UnblendedCost"]}"#,
            ),
            json_error_response("ResourceNotFoundException", "no such cost data"),
        )]);
        let client = CostExplorerClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_cost_and_usage("2026-01-01", "2026-01-31", "DAILY", None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "no such cost data");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_cost_forecast_returns_forecast_results() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TimePeriod":{"Start":"2026-02-01","End":"2026-02-28"},"Granularity":"MONTHLY","Metric":"UNBLENDED_COST"}"#,
            ),
            json_response(
                200,
                r#"{"Total":{"Amount":"100.00","Unit":"USD"},"ForecastResultsByTime":[{"TimePeriod":{"Start":"2026-02-01","End":"2026-02-28"},"MeanValue":"100.00","PredictionIntervalLowerBound":"90.00","PredictionIntervalUpperBound":"110.00"}]}"#,
            ),
        )]);
        let client = CostExplorerClient::new(&sdk_config(http_client.clone()));

        let results = client
            .get_cost_forecast("2026-02-01", "2026-02-28", "MONTHLY")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].mean_value(), Some("100.00"));
        assert_eq!(results[0].prediction_interval_lower_bound(), Some("90.00"));
        assert_eq!(results[0].prediction_interval_upper_bound(), Some("110.00"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_cost_forecast_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TimePeriod":{"Start":"2026-02-01","End":"2026-02-28"},"Granularity":"DAILY","Metric":"UNBLENDED_COST"}"#,
            ),
            json_error_response("DataUnavailableException", "forecast data not available"),
        )]);
        let client = CostExplorerClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_cost_forecast("2026-02-01", "2026-02-28", "DAILY")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DataUnavailableException".to_string()));
                assert_eq!(message, "forecast data not available");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
