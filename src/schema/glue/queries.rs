use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::glue::GlueClient;
use crate::schema::glue::types::{GlueCrawler, GlueDatabase, GlueJob, GlueTable};

#[derive(Default)]
pub struct GlueQuery;

#[Object]
impl GlueQuery {
    async fn glue_databases(&self, ctx: &Context<'_>) -> Result<Vec<GlueDatabase>> {
        let client = ctx.data::<GlueClient>()?;
        let databases = client.get_databases().await?;
        Ok(databases.iter().map(GlueDatabase::from).collect())
    }

    async fn glue_tables(
        &self,
        ctx: &Context<'_>,
        database_name: String,
    ) -> Result<Vec<GlueTable>> {
        let client = ctx.data::<GlueClient>()?;
        let tables = client.get_tables(&database_name).await?;
        Ok(tables.iter().map(GlueTable::from).collect())
    }

    async fn glue_crawlers(&self, ctx: &Context<'_>) -> Result<Vec<GlueCrawler>> {
        let client = ctx.data::<GlueClient>()?;
        let crawlers = client.get_crawlers().await?;
        Ok(crawlers.iter().map(GlueCrawler::from).collect())
    }

    async fn glue_jobs(&self, ctx: &Context<'_>) -> Result<Vec<GlueJob>> {
        let client = ctx.data::<GlueClient>()?;
        let jobs = client.get_jobs().await?;

        let futures: Vec<_> = jobs
            .iter()
            .map(|job| async {
                let name = job.name().unwrap_or_default();
                let runs = client.get_job_runs(name, 1).await;
                let last_status = runs.ok().and_then(|r| {
                    r.first()
                        .and_then(|run| run.job_run_state().map(|s| s.as_str().to_string()))
                });
                GlueJob::from_sdk(job, last_status)
            })
            .collect();

        Ok(join_all(futures).await)
    }
}
