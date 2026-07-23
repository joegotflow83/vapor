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
            .map_err(crate::error::sdk_err)?;
        Ok(output
            .metadata()
            .and_then(|m| m.pipeline_arn())
            .map(|s| s.to_string()))
    }

    /// Lists pipelines, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `max_results` is capped to the remaining
    /// budget so a `limit`-truncated page boundary always lines up with the
    /// token AWS returns (`ListPipelinesInput.max_results` confirmed in pinned
    /// `aws-sdk-codepipeline` 1.113.0).
    pub async fn list_pipelines(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PipelineSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_pipelines();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.pipelines.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists executions for a pipeline, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Same server-side
    /// `max_results` capping as `list_pipelines`
    /// (`ListPipelineExecutionsInput.max_results` confirmed in the pinned SDK).
    pub async fn list_pipeline_executions(
        &self,
        pipeline_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PipelineExecutionSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_pipeline_executions().pipeline_name(pipeline_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.pipeline_execution_summaries.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
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
            .map_err(crate::error::sdk_err)?;

        Ok(output.stage_states().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://codepipeline.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn get_pipeline_arn_returns_arn_when_present() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"name":"my-pipeline"}"#),
            json_response(
                200,
                r#"{"metadata":{"pipelineArn":"arn:aws:codepipeline:us-east-1:111122223333:my-pipeline"}}"#,
            ),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let arn = client.get_pipeline_arn("my-pipeline").await.unwrap();

        assert_eq!(
            arn,
            Some("arn:aws:codepipeline:us-east-1:111122223333:my-pipeline".to_string())
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_pipeline_arn_returns_none_when_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"name":"my-pipeline"}"#),
            json_response(200, r#"{}"#),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let arn = client.get_pipeline_arn("my-pipeline").await.unwrap();

        assert_eq!(arn, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_pipeline_arn_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"name":"my-pipeline"}"#),
            json_error_response("PipelineNotFoundException", "pipeline not found"),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let err = client.get_pipeline_arn("my-pipeline").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("PipelineNotFoundException".to_string()));
                assert_eq!(message, "pipeline not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipelines_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"pipelines":[{"name":"pipeline-1"},{"name":"pipeline-2"}]}"#,
            ),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let (pipelines, token) = client.list_pipelines(None, None).await.unwrap();

        assert_eq!(pipelines.len(), 2);
        assert_eq!(pipelines[0].name(), Some("pipeline-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipelines_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"pipelines":[{"name":"pipeline-3"}]}"#),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let (pipelines, token) = client
            .list_pipelines(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].name(), Some("pipeline-3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipelines_stops_at_limit_and_returns_resume_token() {
        // `ListPipelinesInput.max_results` is a genuine server-side field
        // (confirmed in pinned `aws-sdk-codepipeline` 1.113.0), capped to the
        // remaining budget on each request.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"maxResults":2}"#),
            json_response(
                200,
                r#"{"pipelines":[{"name":"pipeline-1"},{"name":"pipeline-2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let (pipelines, token) = client.list_pipelines(Some(2), None).await.unwrap();

        assert_eq!(pipelines.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipelines_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":10}"#),
                json_response(200, r#"{"pipelines":[{"name":"pipeline-1"}],"nextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"p2","maxResults":9}"#),
                json_response(200, r#"{"pipelines":[{"name":"pipeline-2"}]}"#),
            ),
        ]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let (pipelines, token) = client.list_pipelines(Some(10), None).await.unwrap();

        assert_eq!(pipelines.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipelines_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let err = client.list_pipelines(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipeline_executions_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pipelineName":"my-pipeline"}"#),
            json_response(
                200,
                r#"{"pipelineExecutionSummaries":[{"pipelineExecutionId":"exec-1","status":"Succeeded"}]}"#,
            ),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let (executions, token) = client
            .list_pipeline_executions("my-pipeline", None, None)
            .await
            .unwrap();

        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].pipeline_execution_id(), Some("exec-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipeline_executions_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pipelineName":"my-pipeline","maxResults":1}"#),
            json_response(
                200,
                r#"{"pipelineExecutionSummaries":[{"pipelineExecutionId":"exec-1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let (executions, token) = client
            .list_pipeline_executions("my-pipeline", Some(1), None)
            .await
            .unwrap();

        assert_eq!(executions.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipeline_executions_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"pipelineName":"my-pipeline","nextToken":"cursor-a"}"#,
            ),
            json_response(
                200,
                r#"{"pipelineExecutionSummaries":[{"pipelineExecutionId":"exec-2"}]}"#,
            ),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let (executions, token) = client
            .list_pipeline_executions("my-pipeline", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].pipeline_execution_id(), Some("exec-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_pipeline_executions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"pipelineName":"my-pipeline"}"#),
            json_error_response("PipelineNotFoundException", "pipeline not found"),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_pipeline_executions("my-pipeline", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("PipelineNotFoundException".to_string()));
                assert_eq!(message, "pipeline not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_pipeline_state_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"name":"my-pipeline"}"#),
            json_response(
                200,
                r#"{"pipelineName":"my-pipeline","stageStates":[{"stageName":"Source"},{"stageName":"Deploy"}]}"#,
            ),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let stages = client.get_pipeline_state("my-pipeline").await.unwrap();

        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].stage_name(), Some("Source"));
        assert_eq!(stages[1].stage_name(), Some("Deploy"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_pipeline_state_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"name":"my-pipeline"}"#),
            json_error_response("PipelineNotFoundException", "pipeline not found"),
        )]);
        let client = CodePipelineClient::new(&sdk_config(http_client.clone()));

        let err = client.get_pipeline_state("my-pipeline").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("PipelineNotFoundException".to_string()));
                assert_eq!(message, "pipeline not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

