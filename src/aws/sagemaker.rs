use aws_config::SdkConfig;
use aws_sdk_sagemaker::operation::describe_endpoint::DescribeEndpointOutput;
use aws_sdk_sagemaker::operation::describe_training_job::DescribeTrainingJobOutput;
use aws_sdk_sagemaker::types::{EndpointStatus, EndpointSummary, ModelSummary, TrainingJobStatus, TrainingJobSummary};

use crate::error::VaporError;

pub struct SageMakerClient {
    inner: aws_sdk_sagemaker::Client,
}

impl SageMakerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sagemaker::Client::new(config),
        }
    }

    pub async fn list_endpoints(
        &self,
        status: Option<EndpointStatus>,
    ) -> Result<Vec<EndpointSummary>, VaporError> {
        let mut endpoints = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_endpoints();
            if let Some(ref s) = status {
                req = req.status_equals(s.clone());
            }
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            endpoints.extend(output.endpoints().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(endpoints)
    }

    pub async fn list_training_jobs(
        &self,
        status: Option<TrainingJobStatus>,
        max_results: Option<i32>,
    ) -> Result<Vec<TrainingJobSummary>, VaporError> {
        let mut jobs = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_training_jobs();
            if let Some(ref s) = status {
                req = req.status_equals(s.clone());
            }
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            jobs.extend(output.training_job_summaries().to_vec());

            if let Some(max) = max_results {
                if jobs.len() >= max as usize {
                    jobs.truncate(max as usize);
                    break;
                }
            }

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(jobs)
    }

    pub async fn describe_endpoint(&self, name: &str) -> Result<DescribeEndpointOutput, VaporError> {
        self.inner
            .describe_endpoint()
            .endpoint_name(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn describe_training_job(
        &self,
        name: &str,
    ) -> Result<DescribeTrainingJobOutput, VaporError> {
        self.inner
            .describe_training_job()
            .training_job_name(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_models(&self) -> Result<Vec<ModelSummary>, VaporError> {
        let mut models = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_models();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            models.extend(output.models().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(models)
    }
}
