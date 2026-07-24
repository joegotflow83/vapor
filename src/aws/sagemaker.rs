use aws_config::SdkConfig;
use futures::future::join_all;

use aws_sdk_sagemaker::operation::describe_endpoint::DescribeEndpointOutput;
use aws_sdk_sagemaker::operation::describe_training_job::DescribeTrainingJobOutput;
use aws_sdk_sagemaker::types::{EndpointStatus, ModelSummary, TrainingJobStatus};

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

    /// Lists endpoints, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListEndpoints` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-sagemaker` 1.210.0's `operation/list_endpoints/
    /// _list_endpoints_input.rs`) despite also having a generated
    /// paginator — dropped in favor of a hand-rolled loop that exposes the
    /// token (kinesis-class: the paginator hides it). The N+1
    /// `describe_endpoint` fan-out (list summaries lack most fields) only
    /// covers the single page of names collected this call, matching
    /// `mq.rs`'s `list_brokers`/`sso_admin.rs`'s `list_permission_sets`
    /// pattern; per-item describe failures are swallowed (not
    /// hard-propagated), matching this project's field-enrichment fan-out
    /// convention (connect/datasync/rekognition).
    pub async fn list_endpoints(
        &self,
        status: Option<EndpointStatus>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DescribeEndpointOutput>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - names.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.list_endpoints();
            if let Some(s) = &status {
                req = req.status_equals(s.clone());
            }
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            names.extend(
                output
                    .endpoints
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|s| s.endpoint_name),
            );
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| names.len() as i32 >= l) {
                break;
            }
        }

        let futures = names.iter().map(|name| self.describe_endpoint(name));
        let results = join_all(futures).await;
        let endpoints = results.into_iter().filter_map(|r| r.ok()).collect();

        Ok((endpoints, token))
    }

    /// Lists training jobs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListTrainingJobs` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-sagemaker` 1.210.0's `operation/list_training_jobs/
    /// _list_training_jobs_input.rs`) despite also having a generated
    /// paginator — same kinesis-class reasoning as `list_endpoints`. The N+1
    /// `describe_training_job` fan-out only covers the single page of names
    /// collected this call; per-item failures are swallowed, matching
    /// `list_endpoints` above.
    pub async fn list_training_jobs(
        &self,
        status: Option<TrainingJobStatus>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DescribeTrainingJobOutput>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - names.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.list_training_jobs();
            if let Some(s) = &status {
                req = req.status_equals(s.clone());
            }
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            names.extend(
                output
                    .training_job_summaries
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|s| s.training_job_name),
            );
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| names.len() as i32 >= l) {
                break;
            }
        }

        let futures = names.iter().map(|name| self.describe_training_job(name));
        let results = join_all(futures).await;
        let jobs = results.into_iter().filter_map(|r| r.ok()).collect();

        Ok((jobs, token))
    }

    pub async fn describe_endpoint(
        &self,
        name: &str,
    ) -> Result<DescribeEndpointOutput, VaporError> {
        self.inner
            .describe_endpoint()
            .endpoint_name(name)
            .send()
            .await
            .map_err(crate::error::sdk_err)
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
            .map_err(crate::error::sdk_err)
    }

    /// Lists models, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListModels` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-sagemaker` 1.210.0's `operation/list_models/
    /// _list_models_input.rs`) despite also having a generated paginator —
    /// same kinesis-class reasoning as the other two ops in this file. No
    /// fan-out needed: `ModelSummary` already carries every field
    /// `SageMakerModel` needs.
    pub async fn list_models(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ModelSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.list_models();
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.models.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // awsJson1.1: POST JSON to a fixed `/` path, differentiated only by the
    // `x-amz-target` header (which `test_util::request` doesn't compare) —
    // same shape as `rekognition.rs`/`redshift_serverless.rs`. Crate name
    // (`aws-sdk-sagemaker`) does NOT match the endpoint hostname — the
    // resolved endpoint has an `api.` prefix (`api.sagemaker.*`, verified
    // against pinned `aws-sdk-sagemaker` 1.210.0's `config/endpoint.rs`
    // `test_18`), extending memory gotcha 3. Request/response bodies use
    // PascalCase keys throughout (`NextToken`, `MaxResults`, `StatusEquals`,
    // `EndpointName`, `TrainingJobName`, ...) per each op's
    // `ser_*_input_input`/`de_*` codegen. All 3 ops' aws-layer pagination
    // loops forward `limit` straight to AWS's `MaxResults` with no
    // client-side truncation (memory gotcha 13), so capped-pagination tests
    // must can exactly `limit` items. `serde_util.rs`'s
    // `endpoint_summary_correct_errors`/`training_job_summary_correct_errors`/
    // `model_summary_correct_errors` default-fill the name field to
    // `Some(String::new())` (empty string, not `None`) when omitted from a
    // summary response (memory gotcha 14/20 extension) — the vapor loop's
    // `filter_map(|s| s.endpoint_name)` therefore still collects a
    // zero-length id and triggers a fan-out `describe_endpoint`/
    // `describe_training_job` call for it, unlike a file where a missing
    // name is skippable. `describe_endpoint_output_output_correct_errors`/
    // `describe_training_job_output_output_correct_errors` similarly
    // default-fill several `Describe*Output` fields (empty strings, epoch-0
    // timestamps, an `Unknown("no value was set")` status) rather than
    // leaving them `None` — canned describe responses below always supply
    // those fields explicitly to avoid asserting on those defaults
    // incidentally.
    const BASE: &str = "https://api.sagemaker.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_endpoints_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(200, r#"{"Endpoints":[{"EndpointName":"ep-1"}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-1"}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"ep-1","EndpointArn":"arn:aws:sagemaker:us-east-1:111111111111:endpoint/ep-1","EndpointConfigName":"cfg-1","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000100}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_endpoints(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].endpoint_name(), Some("ep-1"));
        assert_eq!(
            items[0].endpoint_arn(),
            Some("arn:aws:sagemaker:us-east-1:111111111111:endpoint/ep-1")
        );
        assert_eq!(items[0].endpoint_config_name(), Some("cfg-1"));
        assert_eq!(items[0].endpoint_status(), Some(&EndpointStatus::InService));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_filters_by_status_equals() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"StatusEquals":"InService"}"#),
            json_response(200, r#"{"Endpoints":[]}"#),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_endpoints(Some(EndpointStatus::InService), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Endpoints":[]}"#),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_endpoints(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Endpoints":[{"EndpointName":"ep-1"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-1"}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"ep-1","EndpointArn":"arn:ep-1","EndpointConfigName":"cfg-1","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000000}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_endpoints(None, Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Endpoints":[{"EndpointName":"ep-1"},{"EndpointName":"ep-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Endpoints":[{"EndpointName":"ep-3"}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-1"}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"ep-1","EndpointArn":"arn:ep-1","EndpointConfigName":"cfg-1","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000000}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-2"}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"ep-2","EndpointArn":"arn:ep-2","EndpointConfigName":"cfg-2","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000000}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-3"}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"ep-3","EndpointArn":"arn:ep-3","EndpointConfigName":"cfg-3","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000000}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_endpoints(None, Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_propagates_errors() {
        // `ValidationException`, not a throttling-classified code (memory
        // gotcha 1).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_endpoints(None, None, None).await.unwrap_err();

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
    async fn list_endpoints_swallows_describe_endpoint_fan_out_errors() {
        // The `describe_endpoint` fan-out result is folded through `.ok()`
        // (memory gotcha 10) — a failure drops that endpoint from the result
        // entirely instead of propagating or leaving a partial entry, and
        // the top-level call still succeeds.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(
                    200,
                    r#"{"Endpoints":[{"EndpointName":"ep-1"},{"EndpointName":"ep-2"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-1"}"#),
                json_error_response("ResourceNotFound", "no such endpoint"),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-2"}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"ep-2","EndpointArn":"arn:ep-2","EndpointConfigName":"cfg-2","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000000}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_endpoints(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].endpoint_name(), Some("ep-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_endpoints_fans_out_on_empty_name_when_summary_omits_it() {
        // `endpoint_summary_correct_errors` default-fills a missing
        // `EndpointName` to `Some(String::new())`, not `None` — the
        // `filter_map(|s| s.endpoint_name)` loop therefore still collects an
        // empty-string id and issues a fan-out `describe_endpoint("")` call
        // (a second `ReplayEvent` is required here, unlike a file where a
        // missing name is genuinely skippable).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(200, r#"{"Endpoints":[{}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":""}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"","EndpointArn":"","EndpointConfigName":"","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000000}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_endpoints(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].endpoint_name(), Some(""));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_endpoint_returns_output() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"EndpointName":"ep-1"}"#),
            json_response(
                200,
                r#"{"EndpointName":"ep-1","EndpointArn":"arn:ep-1","EndpointConfigName":"cfg-1","EndpointStatus":"Creating","CreationTime":1700000000,"LastModifiedTime":1700000000}"#,
            ),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let output = client.describe_endpoint("ep-1").await.unwrap();

        assert_eq!(output.endpoint_name(), Some("ep-1"));
        assert_eq!(output.endpoint_status(), Some(&EndpointStatus::Creating));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_endpoint_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"EndpointName":"missing"}"#),
            json_error_response("ResourceNotFound", "no such endpoint"),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_endpoint("missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFound".to_string()));
                assert_eq!(message, "no such endpoint");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_training_jobs_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(
                    200,
                    r#"{"TrainingJobSummaries":[{"TrainingJobName":"job-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"TrainingJobName":"job-1"}"#),
                json_response(
                    200,
                    r#"{"TrainingJobName":"job-1","TrainingJobArn":"arn:aws:sagemaker:us-east-1:111111111111:training-job/job-1","TrainingJobStatus":"Completed","SecondaryStatus":"Completed","CreationTime":1700000000}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_training_jobs(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].training_job_name(), Some("job-1"));
        assert_eq!(
            items[0].training_job_arn(),
            Some("arn:aws:sagemaker:us-east-1:111111111111:training-job/job-1")
        );
        assert_eq!(
            items[0].training_job_status(),
            Some(&TrainingJobStatus::Completed)
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_training_jobs_filters_by_status_equals() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"StatusEquals":"InProgress"}"#),
            json_response(200, r#"{"TrainingJobSummaries":[]}"#),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_training_jobs(Some(TrainingJobStatus::InProgress), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_training_jobs_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"TrainingJobSummaries":[]}"#),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_training_jobs(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_training_jobs_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"TrainingJobSummaries":[{"TrainingJobName":"job-1"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"TrainingJobName":"job-1"}"#),
                json_response(
                    200,
                    r#"{"TrainingJobName":"job-1","TrainingJobArn":"arn:job-1","TrainingJobStatus":"InProgress","SecondaryStatus":"Training","CreationTime":1700000000}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_training_jobs(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_training_jobs_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"TrainingJobSummaries":[{"TrainingJobName":"job-1"},{"TrainingJobName":"job-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(
                    200,
                    r#"{"TrainingJobSummaries":[{"TrainingJobName":"job-3"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"TrainingJobName":"job-1"}"#),
                json_response(
                    200,
                    r#"{"TrainingJobName":"job-1","TrainingJobArn":"arn:job-1","TrainingJobStatus":"Completed","SecondaryStatus":"Completed","CreationTime":1700000000}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"TrainingJobName":"job-2"}"#),
                json_response(
                    200,
                    r#"{"TrainingJobName":"job-2","TrainingJobArn":"arn:job-2","TrainingJobStatus":"Completed","SecondaryStatus":"Completed","CreationTime":1700000000}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"TrainingJobName":"job-3"}"#),
                json_response(
                    200,
                    r#"{"TrainingJobName":"job-3","TrainingJobArn":"arn:job-3","TrainingJobStatus":"Completed","SecondaryStatus":"Completed","CreationTime":1700000000}"#,
                ),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_training_jobs(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_training_jobs_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_training_jobs(None, None, None)
            .await
            .unwrap_err();

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
    async fn list_training_jobs_swallows_describe_training_job_fan_out_errors() {
        // Same swallowed-fan-out shape as `list_endpoints` above (memory
        // gotcha 10) — a `describe_training_job` failure drops that job from
        // the result instead of propagating.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(
                    200,
                    r#"{"TrainingJobSummaries":[{"TrainingJobName":"job-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"TrainingJobName":"job-1"}"#),
                json_error_response("ResourceNotFound", "no such training job"),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_training_jobs(None, None, None).await.unwrap();

        assert!(items.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_training_job_returns_output() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"TrainingJobName":"job-1"}"#),
            json_response(
                200,
                r#"{"TrainingJobName":"job-1","TrainingJobArn":"arn:job-1","TrainingJobStatus":"Failed","SecondaryStatus":"Failed","FailureReason":"boom","CreationTime":1700000000}"#,
            ),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let output = client.describe_training_job("job-1").await.unwrap();

        assert_eq!(output.training_job_name(), Some("job-1"));
        assert_eq!(
            output.training_job_status(),
            Some(&TrainingJobStatus::Failed)
        );
        assert_eq!(output.failure_reason(), Some("boom"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_training_job_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"TrainingJobName":"missing"}"#),
            json_error_response("ResourceNotFound", "no such training job"),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_training_job("missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFound".to_string()));
                assert_eq!(message, "no such training job");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_models_lists_all_when_no_limit() {
        // Plain paginated list, no fan-out (unlike `list_endpoints`/
        // `list_training_jobs`) — `ModelSummary` already carries every field
        // `list_models` needs.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Models":[{"ModelName":"model-1","ModelArn":"arn:model-1","CreationTime":1700000000}]}"#,
            ),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_models(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].model_name(), Some("model-1"));
        assert_eq!(items[0].model_arn(), Some("arn:model-1"));
        assert_eq!(
            items[0].creation_time(),
            Some(&aws_smithy_types::DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_models_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Models":[]}"#),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_models(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_models_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Models":[{"ModelName":"model-1"},{"ModelName":"model-2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_models(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_models_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Models":[{"ModelName":"model-1"},{"ModelName":"model-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Models":[{"ModelName":"model-3"}]}"#),
            ),
        ]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_models(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_models_defaults_missing_name_to_empty_string() {
        // `model_summary_correct_errors` default-fills a missing `ModelName`
        // to `Some(String::new())`, not `None` (memory gotcha 14/20
        // extension, same shape as `list_endpoints`' summary above) — unlike
        // `list_endpoints`/`list_training_jobs`, there's no fan-out here to
        // observe the effect through, so this asserts it directly on the
        // returned `ModelSummary`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(200, r#"{"Models":[{}]}"#),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_models(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].model_name(), Some(""));
        assert_eq!(items[0].model_arn(), Some(""));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_models_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = SageMakerClient::new(&sdk_config(http_client.clone()));

        let err = client.list_models(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
