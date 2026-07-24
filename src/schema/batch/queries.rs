use async_graphql::{Context, Object, Result};

use crate::aws::batch::BatchClient;
use crate::schema::batch::types::{BatchComputeEnvironment, BatchJobDefinition, BatchJobQueue};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct BatchQuery;

#[Object]
impl BatchQuery {
    async fn batch_job_queues(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BatchJobQueue>> {
        let client = ctx.data::<BatchClient>()?;
        let (queues, next_token) = client.describe_job_queues(limit, next_token).await?;
        Ok(Page {
            items: queues.into_iter().map(BatchJobQueue::from).collect(),
            next_token,
        })
    }

    async fn batch_compute_environments(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BatchComputeEnvironment>> {
        let client = ctx.data::<BatchClient>()?;
        let (envs, next_token) = client
            .describe_compute_environments(limit, next_token)
            .await?;
        Ok(Page {
            items: envs
                .into_iter()
                .map(BatchComputeEnvironment::from)
                .collect(),
            next_token,
        })
    }

    async fn batch_job_definitions(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BatchJobDefinition>> {
        let client = ctx.data::<BatchClient>()?;
        let (defs, next_token) = client
            .describe_job_definitions(status.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: defs.into_iter().map(BatchJobDefinition::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `BatchClient` method each (see `src/aws/batch.rs`'s own test module for
// the pagination/limit/status-filter/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::batch::BatchClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::BatchQuery;

    const JOB_QUEUES: &str = "https://batch.us-east-1.amazonaws.com/v1/describejobqueues";
    const COMPUTE_ENVS: &str =
        "https://batch.us-east-1.amazonaws.com/v1/describecomputeenvironments";
    const JOB_DEFS: &str = "https://batch.us-east-1.amazonaws.com/v1/describejobdefinitions";

    #[tokio::test]
    async fn batch_job_queues_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_QUEUES, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"jobQueues":[{"jobQueueName":"q1","jobQueueArn":"arn:q1","state":"ENABLED","status":"VALID","priority":10}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(BatchQuery)
            .data(BatchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ batchJobQueues(limit: 1) { items { name arn state status priority } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["batchJobQueues"]["items"];
        assert_eq!(items[0]["name"], "q1");
        assert_eq!(items[0]["arn"], "arn:q1");
        assert_eq!(items[0]["state"], "ENABLED");
        assert_eq!(items[0]["status"], "VALID");
        assert_eq!(items[0]["priority"], 10);
        assert_eq!(json["batchJobQueues"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_compute_environments_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(COMPUTE_ENVS, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"computeEnvironments":[{"computeEnvironmentName":"ce1","computeEnvironmentArn":"arn:ce1","state":"ENABLED","status":"VALID","type":"MANAGED","computeResources":{"type":"EC2","maxvCpus":256,"desiredvCpus":0,"instanceTypes":["m5.large"]}}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(BatchQuery)
            .data(BatchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ batchComputeEnvironments(limit: 1) { items { name arn state status computeType instanceTypes maxVcpus desiredVcpus } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["batchComputeEnvironments"]["items"];
        assert_eq!(items[0]["name"], "ce1");
        assert_eq!(items[0]["arn"], "arn:ce1");
        assert_eq!(items[0]["computeType"], "MANAGED");
        assert_eq!(items[0]["instanceTypes"], serde_json::json!(["m5.large"]));
        assert_eq!(items[0]["maxVcpus"], 256);
        assert_eq!(items[0]["desiredVcpus"], 0);
        assert_eq!(json["batchComputeEnvironments"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_job_definitions_maps_items_and_forwards_status_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(JOB_DEFS, r#"{"status":"RUNNABLE"}"#),
            json_response(
                200,
                r#"{"jobDefinitions":[{"jobDefinitionName":"jd1","jobDefinitionArn":"arn:jd1","revision":1,"status":"RUNNABLE","type":"container"}]}"#,
            ),
        )]);
        let schema = build_query_schema(BatchQuery)
            .data(BatchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ batchJobDefinitions(status: "RUNNABLE") { items { name arn revision status jobType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["batchJobDefinitions"]["items"];
        assert_eq!(items[0]["name"], "jd1");
        assert_eq!(items[0]["arn"], "arn:jd1");
        assert_eq!(items[0]["revision"], 1);
        assert_eq!(items[0]["status"], "RUNNABLE");
        assert_eq!(items[0]["jobType"], "container");
        assert!(json["batchJobDefinitions"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
