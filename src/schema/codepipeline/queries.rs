use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::codepipeline::CodePipelineClient;
use crate::schema::codepipeline::types::{Pipeline, PipelineExecution, StageState};

#[derive(Default)]
pub struct CodePipelineQuery;

#[Object]
impl CodePipelineQuery {
    async fn pipelines(&self, ctx: &Context<'_>) -> Result<Vec<Pipeline>> {
        let client = ctx.data::<CodePipelineClient>()?;
        let summaries = client.list_pipelines().await?;
        let pipelines = join_all(summaries.into_iter().map(|s| async {
            let name = s.name().unwrap_or_default().to_string();
            let arn = client.get_pipeline_arn(&name).await.ok().flatten();
            Pipeline::from_summary(s, arn)
        }))
        .await;
        Ok(pipelines)
    }

    async fn pipeline_executions(
        &self,
        ctx: &Context<'_>,
        pipeline_name: String,
    ) -> Result<Vec<PipelineExecution>> {
        let client = ctx.data::<CodePipelineClient>()?;
        let summaries = client.list_pipeline_executions(&pipeline_name).await?;
        Ok(summaries
            .iter()
            .map(|s| PipelineExecution::from_summary(&pipeline_name, s))
            .collect())
    }

    async fn pipeline_state(
        &self,
        ctx: &Context<'_>,
        pipeline_name: String,
    ) -> Result<Vec<StageState>> {
        let client = ctx.data::<CodePipelineClient>()?;
        let stages = client.get_pipeline_state(&pipeline_name).await?;
        Ok(stages.iter().map(StageState::from).collect())
    }
}
