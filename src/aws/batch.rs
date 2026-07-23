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

    /// Lists job queues, capped at `limit` results (default unlimited),
    /// resuming from `next_token` if given. Returns the page's items plus a
    /// token to resume from, if more results remain.
    pub async fn describe_job_queues(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<JobQueueDetail>, Option<String>), VaporError> {
        let mut queues = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - queues.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.describe_job_queues();
            if let Some(r) = remaining {
                req = req.max_results(r);
            }
            if let Some(t) = token.take() {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            queues.extend(output.job_queues.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| queues.len() as i32 >= l) {
                break;
            }
        }

        Ok((queues, token))
    }

    /// Lists compute environments, capped at `limit` results (default
    /// unlimited), resuming from `next_token` if given. Returns the page's
    /// items plus a token to resume from, if more results remain.
    pub async fn describe_compute_environments(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ComputeEnvironmentDetail>, Option<String>), VaporError> {
        let mut envs = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - envs.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.describe_compute_environments();
            if let Some(r) = remaining {
                req = req.max_results(r);
            }
            if let Some(t) = token.take() {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            envs.extend(output.compute_environments.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| envs.len() as i32 >= l) {
                break;
            }
        }

        Ok((envs, token))
    }

    /// Lists job definitions, capped at `limit` results (default
    /// unlimited), resuming from `next_token` if given. Returns the page's
    /// items plus a token to resume from, if more results remain.
    pub async fn describe_job_definitions(
        &self,
        status: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<JobDefinition>, Option<String>), VaporError> {
        let mut defs = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - defs.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.describe_job_definitions();
            if let Some(s) = status {
                req = req.status(s);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }
            if let Some(t) = token.take() {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            defs.extend(output.job_definitions.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| defs.len() as i32 >= l) {
                break;
            }
        }

        Ok((defs, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const JOB_QUEUES: &str = "https://batch.us-east-1.amazonaws.com/v1/describejobqueues";
    const COMPUTE_ENVS: &str = "https://batch.us-east-1.amazonaws.com/v1/describecomputeenvironments";
    const JOB_DEFS: &str = "https://batch.us-east-1.amazonaws.com/v1/describejobdefinitions";

    #[tokio::test]
    async fn describe_job_queues_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_QUEUES, "{}"),
            json_response(
                200,
                r#"{"jobQueues":[{"jobQueueName":"q1","jobQueueArn":"arn:q1","state":"ENABLED","status":"VALID"},{"jobQueueName":"q2","jobQueueArn":"arn:q2","state":"ENABLED","status":"VALID"}]}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (queues, token) = client.describe_job_queues(None, None).await.unwrap();

        assert_eq!(queues.len(), 2);
        assert_eq!(queues[0].job_queue_name(), Some("q1"));
        assert_eq!(queues[1].job_queue_arn(), Some("arn:q2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_job_queues_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_QUEUES, r#"{"nextToken":"cursor-a"}"#),
            json_response(
                200,
                r#"{"jobQueues":[{"jobQueueName":"q3","jobQueueArn":"arn:q3","state":"ENABLED","status":"VALID"}]}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (queues, token) = client
            .describe_job_queues(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(queues.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_job_queues_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_QUEUES, r#"{"maxResults":2}"#),
            json_response(
                200,
                r#"{"jobQueues":[{"jobQueueName":"q1","jobQueueArn":"arn:q1"},{"jobQueueName":"q2","jobQueueArn":"arn:q2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (queues, token) = client.describe_job_queues(Some(2), None).await.unwrap();

        assert_eq!(queues.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_job_queues_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(JOB_QUEUES, r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"jobQueues":[{"jobQueueName":"q1","jobQueueArn":"arn:q1"},{"jobQueueName":"q2","jobQueueArn":"arn:q2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(JOB_QUEUES, r#"{"maxResults":8,"nextToken":"p2"}"#),
                json_response(200, r#"{"jobQueues":[{"jobQueueName":"q3","jobQueueArn":"arn:q3"}]}"#),
            ),
        ]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (queues, token) = client.describe_job_queues(Some(10), None).await.unwrap();

        assert_eq!(queues.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_job_queues_propagates_errors() {
        // `ClientException` (one of this op's 2 modeled errors — `ClientException`/
        // `ServerException`, neither is a built-in throttling-exception name — see
        // apigateway.rs's retry-pitfall precedent) rather than a throttling name.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_QUEUES, "{}"),
            json_error_response("ClientException", "invalid job queue"),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_job_queues(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ClientException".to_string()));
                assert_eq!(message, "invalid job queue");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compute_environments_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COMPUTE_ENVS, "{}"),
            json_response(
                200,
                r#"{"computeEnvironments":[{"computeEnvironmentName":"ce1","computeEnvironmentArn":"arn:ce1","state":"ENABLED","status":"VALID"}]}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client.describe_compute_environments(None, None).await.unwrap();

        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].compute_environment_name(), Some("ce1"));
        assert_eq!(envs[0].compute_environment_arn(), Some("arn:ce1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compute_environments_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COMPUTE_ENVS, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"computeEnvironments":[{"computeEnvironmentName":"ce1","computeEnvironmentArn":"arn:ce1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (envs, token) = client.describe_compute_environments(Some(1), None).await.unwrap();

        assert_eq!(envs.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_job_definitions_lists_all_when_no_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_DEFS, "{}"),
            json_response(
                200,
                r#"{"jobDefinitions":[{"jobDefinitionName":"jd1","jobDefinitionArn":"arn:jd1","revision":1,"status":"ACTIVE"}]}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (defs, token) = client.describe_job_definitions(None, None, None).await.unwrap();

        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].job_definition_name(), Some("jd1"));
        assert_eq!(defs[0].revision(), Some(1));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_job_definitions_passes_status_filter_and_orders_fields_before_pagination() {
        // Verifies both the `status` filter passthrough and that the wrapper's
        // field order (`max_results` set, then `next_token`, then `status` on
        // the request builder) doesn't affect the emitted JSON key order — the
        // SDK's codegen always writes `maxResults` before `status` regardless
        // of call order (same "codegen order, not call order" lesson as
        // apigatewayv2/appsync/athena).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_DEFS, r#"{"maxResults":5,"status":"RUNNABLE"}"#),
            json_response(
                200,
                r#"{"jobDefinitions":[{"jobDefinitionName":"jd1","jobDefinitionArn":"arn:jd1","status":"RUNNABLE"}]}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (defs, token) = client
            .describe_job_definitions(Some("RUNNABLE"), Some(5), None)
            .await
            .unwrap();

        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].status(), Some("RUNNABLE"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_job_definitions_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_DEFS, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"jobDefinitions":[{"jobDefinitionName":"jd1","jobDefinitionArn":"arn:jd1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = BatchClient::new(&sdk_config(http_client.clone()));

        let (defs, token) = client.describe_job_definitions(None, Some(1), None).await.unwrap();

        assert_eq!(defs.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }
}

