use async_graphql::{Context, Object, Result};

use crate::aws::cost_explorer::CostExplorerClient;
use crate::schema::cost_explorer::types::{CostAndUsageResult, ForecastResult};

#[derive(Default)]
pub struct CostExplorerQuery;

#[Object]
impl CostExplorerQuery {
    async fn cost_and_usage(
        &self,
        ctx: &Context<'_>,
        start: String,
        end: String,
        granularity: String,
        group_by: Option<Vec<String>>,
    ) -> Result<Vec<CostAndUsageResult>> {
        let client = ctx.data::<CostExplorerClient>()?;
        let results = client
            .get_cost_and_usage(&start, &end, &granularity, group_by)
            .await?;
        Ok(results.into_iter().map(CostAndUsageResult::from).collect())
    }

    async fn cost_forecast(
        &self,
        ctx: &Context<'_>,
        start: String,
        end: String,
        granularity: String,
    ) -> Result<Vec<ForecastResult>> {
        let client = ctx.data::<CostExplorerClient>()?;
        let results = client
            .get_cost_forecast(&start, &end, &granularity)
            .await?;
        Ok(results.into_iter().map(ForecastResult::from).collect())
    }
}
