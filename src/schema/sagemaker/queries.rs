use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use aws_sdk_sagemaker::types::{EndpointStatus, TrainingJobStatus};

use crate::aws::sagemaker::SageMakerClient;
use crate::schema::sagemaker::types::{SageMakerEndpoint, SageMakerModel, SageMakerTrainingJob};

#[derive(Default)]
pub struct SageMakerQuery;

#[Object]
impl SageMakerQuery {
    async fn sagemaker_endpoints(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
    ) -> Result<Vec<SageMakerEndpoint>> {
        let client = ctx.data::<SageMakerClient>()?;
        let status = status_filter.map(|s| EndpointStatus::from(s.as_str()));
        let summaries = client.list_endpoints(status).await?;

        let futures: Vec<_> = summaries
            .iter()
            .filter_map(|s| s.endpoint_name())
            .map(|name| client.describe_endpoint(name))
            .collect();

        let results = join_all(futures).await;
        let endpoints = results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(SageMakerEndpoint::from)
            .collect();

        Ok(endpoints)
    }

    async fn sagemaker_training_jobs(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
        max_results: Option<i32>,
    ) -> Result<Vec<SageMakerTrainingJob>> {
        let client = ctx.data::<SageMakerClient>()?;
        let status = status_filter.map(|s| TrainingJobStatus::from(s.as_str()));
        let summaries = client.list_training_jobs(status, max_results).await?;

        let futures: Vec<_> = summaries
            .iter()
            .filter_map(|s| s.training_job_name())
            .map(|name| client.describe_training_job(name))
            .collect();

        let results = join_all(futures).await;
        let jobs = results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(SageMakerTrainingJob::from)
            .collect();

        Ok(jobs)
    }

    async fn sagemaker_models(&self, ctx: &Context<'_>) -> Result<Vec<SageMakerModel>> {
        let client = ctx.data::<SageMakerClient>()?;
        let models = client.list_models().await?;
        Ok(models.into_iter().map(SageMakerModel::from).collect())
    }
}
