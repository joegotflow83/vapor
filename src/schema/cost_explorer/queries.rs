use async_graphql::{Context, Object, Result};

use crate::aws::cost_explorer::CostExplorerClient;
use crate::schema::cost_explorer::types::{CostAndUsageResult, ForecastResult};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CostExplorerQuery;

#[Object]
impl CostExplorerQuery {
    /// Fetches cost and usage data, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn cost_and_usage(
        &self,
        ctx: &Context<'_>,
        start: String,
        end: String,
        granularity: String,
        group_by: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CostAndUsageResult>> {
        let client = ctx.data::<CostExplorerClient>()?;
        let (results, token) = client
            .get_cost_and_usage(&start, &end, &granularity, group_by, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(CostAndUsageResult::from).collect(),
            next_token: token,
        })
    }

    async fn cost_forecast(
        &self,
        ctx: &Context<'_>,
        start: String,
        end: String,
        granularity: String,
    ) -> Result<Vec<ForecastResult>> {
        let client = ctx.data::<CostExplorerClient>()?;
        let results = client.get_cost_forecast(&start, &end, &granularity).await?;
        Ok(results.into_iter().map(ForecastResult::from).collect())
    }
}

// Both resolvers are 1:1 passthroughs to a single already-tested
// `CostExplorerClient` method each (see `src/aws/cost_explorer.rs`'s own
// test module for pagination/error-mapping behavior) — only light smoke
// tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::cost_explorer::CostExplorerClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CostExplorerQuery;

    const ENDPOINT: &str = "https://ce.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn cost_and_usage_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Granularity":"MONTHLY","Metrics":["UnblendedCost"],"GroupBy":[{"Type":"DIMENSION","Key":"SERVICE"}]}"#,
            ),
            json_response(
                200,
                r#"{"ResultsByTime":[{"TimePeriod":{"Start":"2026-01-01","End":"2026-01-31"},"Total":{"UnblendedCost":{"Amount":"12.34","Unit":"USD"}},"Groups":[{"Keys":["Amazon EC2"],"Metrics":{"UnblendedCost":{"Amount":"5.00","Unit":"USD"}}}]}],"NextPageToken":"cursor-b"}"#,
            ),
        )]);
        let schema = build_query_schema(CostExplorerQuery)
            .data(CostExplorerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ costAndUsage(start: "2026-01-01", end: "2026-01-31", granularity: "MONTHLY", groupBy: ["SERVICE"], limit: 1) { items { timePeriodStart timePeriodEnd totalAmount totalUnit groups { keys amount unit } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["costAndUsage"]["items"];
        assert_eq!(items[0]["timePeriodStart"], "2026-01-01");
        assert_eq!(items[0]["timePeriodEnd"], "2026-01-31");
        assert_eq!(items[0]["totalAmount"], "12.34");
        assert_eq!(items[0]["totalUnit"], "USD");
        assert_eq!(items[0]["groups"][0]["keys"][0], "Amazon EC2");
        assert_eq!(items[0]["groups"][0]["amount"], "5.00");
        assert_eq!(items[0]["groups"][0]["unit"], "USD");
        assert_eq!(json["costAndUsage"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cost_forecast_maps_results() {
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
        let schema = build_query_schema(CostExplorerQuery)
            .data(CostExplorerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ costForecast(start: "2026-02-01", end: "2026-02-28", granularity: "MONTHLY") { timePeriodStart timePeriodEnd meanValue predictionIntervalLowerBound predictionIntervalUpperBound } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let forecast = &json["costForecast"][0];
        assert_eq!(forecast["timePeriodStart"], "2026-02-01");
        assert_eq!(forecast["timePeriodEnd"], "2026-02-28");
        assert_eq!(forecast["meanValue"], "100.00");
        assert_eq!(forecast["predictionIntervalLowerBound"], "90.00");
        assert_eq!(forecast["predictionIntervalUpperBound"], "110.00");
        http_client.relaxed_requests_match();
    }
}
