use async_graphql::{Context, Object, Result};

use aws_sdk_sagemaker::types::{EndpointStatus, TrainingJobStatus};

use crate::aws::sagemaker::SageMakerClient;
use crate::schema::pagination::Page;
use crate::schema::sagemaker::types::{SageMakerEndpoint, SageMakerModel, SageMakerTrainingJob};

#[derive(Default)]
pub struct SageMakerQuery;

#[Object]
impl SageMakerQuery {
    /// Lists endpoints, optionally filtered by status, capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn sagemaker_endpoints(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SageMakerEndpoint>> {
        let client = ctx.data::<SageMakerClient>()?;
        let status = status_filter.map(|s| EndpointStatus::from(s.as_str()));
        let (endpoints, next_token) = client.list_endpoints(status, limit, next_token).await?;
        Ok(Page {
            items: endpoints.into_iter().map(SageMakerEndpoint::from).collect(),
            next_token,
        })
    }

    /// Lists training jobs, optionally filtered by status, capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    async fn sagemaker_training_jobs(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SageMakerTrainingJob>> {
        let client = ctx.data::<SageMakerClient>()?;
        let status = status_filter.map(|s| TrainingJobStatus::from(s.as_str()));
        let (jobs, next_token) = client.list_training_jobs(status, limit, next_token).await?;
        Ok(Page {
            items: jobs.into_iter().map(SageMakerTrainingJob::from).collect(),
            next_token,
        })
    }

    /// Lists models, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn sagemaker_models(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SageMakerModel>> {
        let client = ctx.data::<SageMakerClient>()?;
        let (models, next_token) = client.list_models(limit, next_token).await?;
        Ok(Page {
            items: models.into_iter().map(SageMakerModel::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    // awsJson1.1 fixed `/` path; endpoint hostname has an `api.` prefix not
    // present in the crate name (`aws-sdk-sagemaker`) — memory gotcha 3
    // extension, same note as `src/aws/sagemaker.rs`'s own test module.
    const BASE: &str = "https://api.sagemaker.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn sagemaker_endpoints_maps_items_through_describe_fan_out() {
        // `list_endpoints` fans list->per-name `describe_endpoint` out
        // internally (already covered in src/aws/sagemaker.rs), so this
        // resolver is a bare passthrough exercised end-to-end with both
        // `ReplayEvent`s (rekognition precedent). `limit: 1` applied
        // proactively (gotcha 29) since the mocked list response carries a
        // `NextToken`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Endpoints":[{"EndpointName":"ep-1"}],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"EndpointName":"ep-1"}"#),
                json_response(
                    200,
                    r#"{"EndpointName":"ep-1","EndpointArn":"arn:aws:sagemaker:us-east-1:111111111111:endpoint/ep-1","EndpointConfigName":"cfg-1","EndpointStatus":"InService","CreationTime":1700000000,"LastModifiedTime":1700000100}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(SageMakerQuery)
            .data(SageMakerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ sagemakerEndpoints(limit: 1) \
                 { items { name arn status endpointConfigName } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["sagemakerEndpoints"]["items"];
        assert_eq!(items[0]["name"], "ep-1");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:sagemaker:us-east-1:111111111111:endpoint/ep-1"
        );
        assert_eq!(items[0]["status"], "InService");
        assert_eq!(items[0]["endpointConfigName"], "cfg-1");
        assert_eq!(json["sagemakerEndpoints"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn sagemaker_training_jobs_maps_items_through_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"TrainingJobSummaries":[{"TrainingJobName":"job-1"}],"NextToken":"page2"}"#,
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
        let schema = build_query_schema(SageMakerQuery)
            .data(SageMakerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ sagemakerTrainingJobs(limit: 1) \
                 { items { name arn status } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["sagemakerTrainingJobs"]["items"];
        assert_eq!(items[0]["name"], "job-1");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:sagemaker:us-east-1:111111111111:training-job/job-1"
        );
        assert_eq!(items[0]["status"], "Completed");
        assert_eq!(json["sagemakerTrainingJobs"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn sagemaker_models_maps_items() {
        // No fan-out (unlike the other two): `ModelSummary` already carries
        // every field `SageMakerModel` needs.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Models":[{"ModelName":"model-1","ModelArn":"arn:model-1","CreationTime":1700000000}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(SageMakerQuery)
            .data(SageMakerClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ sagemakerModels(limit: 1) { items { name arn } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["sagemakerModels"]["items"];
        assert_eq!(items[0]["name"], "model-1");
        assert_eq!(items[0]["arn"], "arn:model-1");
        assert_eq!(json["sagemakerModels"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
