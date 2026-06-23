use aws_config::SdkConfig;
use aws_sdk_batch::types::{ComputeEnvironmentDetail, JobDefinition, JobQueueDetail};

use crate::error::VaporError;

pub struct BatchClient {
    inner: aws_sdk_batch::Client,
}

impl BatchClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_batch::Client::new(config),
        }
    }

    pub async fn describe_job_queues(&self) -> Result<Vec<JobQueueDetail>, VaporError> {
        let mut queues = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_job_queues();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            queues.extend(output.job_queues().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(queues)
    }

    pub async fn describe_compute_environments(
        &self,
    ) -> Result<Vec<ComputeEnvironmentDetail>, VaporError> {
        let mut envs = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_compute_environments();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            envs.extend(output.compute_environments().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(envs)
    }

    pub async fn describe_job_definitions(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<JobDefinition>, VaporError> {
        let mut defs = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_job_definitions();
            if let Some(s) = status {
                req = req.status(s);
            }
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            defs.extend(output.job_definitions().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(defs)
    }
}
