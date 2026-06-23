use aws_config::SdkConfig;
use aws_sdk_codepipeline::types::{PipelineSummary, PipelineExecutionSummary, StageState};

use crate::error::VaporError;

pub struct CodePipelineClient {
    inner: aws_sdk_codepipeline::Client,
}

impl CodePipelineClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codepipeline::Client::new(config),
        }
    }

    pub async fn get_pipeline_arn(&self, name: &str) -> Result<Option<String>, VaporError> {
        let output = self
            .inner
            .get_pipeline()
            .name(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output
            .metadata()
            .and_then(|m| m.pipeline_arn())
            .map(|s| s.to_string()))
    }

    pub async fn list_pipelines(&self) -> Result<Vec<PipelineSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_pipelines();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.pipelines().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn list_pipeline_executions(
        &self,
        pipeline_name: &str,
    ) -> Result<Vec<PipelineExecutionSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_pipeline_executions().pipeline_name(pipeline_name);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.pipeline_execution_summaries().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn get_pipeline_state(
        &self,
        pipeline_name: &str,
    ) -> Result<Vec<StageState>, VaporError> {
        let output = self
            .inner
            .get_pipeline_state()
            .name(pipeline_name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.stage_states().to_vec())
    }
}
