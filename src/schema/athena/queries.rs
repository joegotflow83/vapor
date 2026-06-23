use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::athena::AthenaClient;
use crate::schema::athena::types::{AthenaNamedQuery, AthenaQueryExecution, AthenaWorkgroup};

#[derive(Default)]
pub struct AthenaQuery;

#[Object]
impl AthenaQuery {
    async fn athena_workgroups(&self, ctx: &Context<'_>) -> Result<Vec<AthenaWorkgroup>> {
        let client = ctx.data::<AthenaClient>()?;
        let summaries = client.list_work_groups().await?;

        let futures: Vec<_> = summaries
            .iter()
            .map(|s| async {
                let name = s.name().unwrap_or_default();
                client.get_work_group(name).await
            })
            .collect();

        let results = join_all(futures).await;
        Ok(results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(|wg| AthenaWorkgroup::from_sdk(&wg))
            .collect())
    }

    async fn athena_named_queries(
        &self,
        ctx: &Context<'_>,
        workgroup: Option<String>,
    ) -> Result<Vec<AthenaNamedQuery>> {
        let client = ctx.data::<AthenaClient>()?;
        let ids = client.list_named_queries(workgroup.as_deref()).await?;
        let queries = client.batch_get_named_query(ids).await?;
        Ok(queries.into_iter().map(AthenaNamedQuery::from).collect())
    }

    async fn athena_query_executions(
        &self,
        ctx: &Context<'_>,
        workgroup: Option<String>,
        max_results: Option<i32>,
    ) -> Result<Vec<AthenaQueryExecution>> {
        let client = ctx.data::<AthenaClient>()?;
        let ids = client.list_query_executions(workgroup.as_deref(), max_results).await?;
        let executions = client.batch_get_query_execution(ids).await?;
        Ok(executions.into_iter().map(AthenaQueryExecution::from).collect())
    }
}
