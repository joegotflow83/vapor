use async_graphql::{Context, Object, Result};

use crate::aws::datasync::DataSyncClient;
use crate::schema::datasync::types::{DataSyncAgent, DataSyncLocation, DataSyncTask, DataSyncTaskExecution};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct DataSyncQuery;

#[Object]
impl DataSyncQuery {
    /// Lists DataSync agents, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn data_sync_agents(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DataSyncAgent>> {
        let client = ctx.data::<DataSyncClient>()?;
        let (agents, next_token) = client.list_agents(limit, next_token).await?;
        Ok(Page {
            items: agents.into_iter().map(DataSyncAgent::from).collect(),
            next_token,
        })
    }

    /// Lists DataSync locations, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn data_sync_locations(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DataSyncLocation>> {
        let client = ctx.data::<DataSyncClient>()?;
        let (locations, next_token) = client.list_locations(limit, next_token).await?;
        Ok(Page {
            items: locations.into_iter().map(DataSyncLocation::from).collect(),
            next_token,
        })
    }

    /// Lists DataSync tasks, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn data_sync_tasks(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DataSyncTask>> {
        let client = ctx.data::<DataSyncClient>()?;
        let (tasks, next_token) = client.list_tasks(limit, next_token).await?;
        Ok(Page {
            items: tasks.into_iter().map(DataSyncTask::from).collect(),
            next_token,
        })
    }

    /// Lists executions for a DataSync task, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn data_sync_task_executions(
        &self,
        ctx: &Context<'_>,
        task_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DataSyncTaskExecution>> {
        let client = ctx.data::<DataSyncClient>()?;
        let (executions, next_token) = client.list_task_executions(task_arn, limit, next_token).await?;
        Ok(Page {
            items: executions.into_iter().map(DataSyncTaskExecution::from).collect(),
            next_token,
        })
    }
}

// All 4 resolvers are 1:1 passthroughs to already-tested `DataSyncClient`
// methods (see `src/aws/datasync.rs`'s own test module for pagination/
// fan-out/error-mapping behavior) — only light smoke tests are needed here
// per the resolver-layer sweep's stated scope. `dataSyncAgents`/
// `dataSyncTasks` wrap a per-item `describe_*` fan-out inside the aws-layer
// client, so per the connect/control_tower precedent those two are
// exercised end-to-end via 2 `ReplayEvent`s (list + describe) rather than a
// forced zero-result response, to prove real item-mapping.
#[cfg(test)]
mod tests {
    use crate::aws::datasync::DataSyncClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::DataSyncQuery;

    const ENDPOINT: &str = "https://datasync.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn data_sync_agents_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Agents":[{"AgentArn":"arn:agent-1","Name":"agent-one","Status":"ONLINE"}],"NextToken":"next-agents"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"AgentArn":"arn:agent-1"}"#),
                json_response(200, r#"{"AgentArn":"arn:agent-1","CreationTime":1700000000}"#),
            ),
        ]);
        let schema = build_query_schema(DataSyncQuery)
            .data(DataSyncClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ dataSyncAgents(limit: 1) { items { agentArn name status creationTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["dataSyncAgents"]["items"];
        assert_eq!(items[0]["agentArn"], "arn:agent-1");
        assert_eq!(items[0]["name"], "agent-one");
        assert_eq!(items[0]["status"], "ONLINE");
        assert_eq!(items[0]["creationTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(json["dataSyncAgents"]["nextToken"], "next-agents");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn data_sync_locations_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Locations":[{"LocationArn":"arn:loc-1","LocationUri":"s3://bucket/prefix"}],"NextToken":"next-locations"}"#,
            ),
        )]);
        let schema = build_query_schema(DataSyncQuery)
            .data(DataSyncClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ dataSyncLocations(limit: 1) { items { locationArn locationUri creationTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["dataSyncLocations"]["items"];
        assert_eq!(items[0]["locationArn"], "arn:loc-1");
        assert_eq!(items[0]["locationUri"], "s3://bucket/prefix");
        assert!(items[0]["creationTime"].is_null());
        assert_eq!(json["dataSyncLocations"]["nextToken"], "next-locations");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn data_sync_tasks_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Tasks":[{"TaskArn":"arn:task-1","Status":"AVAILABLE","Name":"task-one"}],"NextToken":"next-tasks"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskArn":"arn:task-1"}"#),
                json_response(
                    200,
                    r#"{"TaskArn":"arn:task-1","Status":"AVAILABLE","Name":"task-one","SourceLocationArn":"arn:loc-src","DestinationLocationArn":"arn:loc-dst","CreationTime":1700000000}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(DataSyncQuery)
            .data(DataSyncClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ dataSyncTasks(limit: 1) { items { taskArn name status sourceLocationArn destinationLocationArn creationTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["dataSyncTasks"]["items"];
        assert_eq!(items[0]["taskArn"], "arn:task-1");
        assert_eq!(items[0]["name"], "task-one");
        assert_eq!(items[0]["status"], "AVAILABLE");
        assert_eq!(items[0]["sourceLocationArn"], "arn:loc-src");
        assert_eq!(items[0]["destinationLocationArn"], "arn:loc-dst");
        assert_eq!(items[0]["creationTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(json["dataSyncTasks"]["nextToken"], "next-tasks");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn data_sync_task_executions_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskArn":"arn:task-1","MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"TaskExecutions":[{"TaskExecutionArn":"arn:task-1/execution/exec-1","Status":"SUCCESS"}],"NextToken":"next-execs"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"TaskExecutionArn":"arn:task-1/execution/exec-1"}"#),
                json_response(
                    200,
                    r#"{"TaskExecutionArn":"arn:task-1/execution/exec-1","Status":"SUCCESS","StartTime":1700000000,"EstimatedFilesToTransfer":100,"FilesTransferred":100,"BytesTransferred":204800}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(DataSyncQuery)
            .data(DataSyncClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ dataSyncTaskExecutions(taskArn: "arn:task-1", limit: 1) { items { taskExecutionArn status startTime estimatedFilesToTransfer filesTransferred bytesTransferred } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["dataSyncTaskExecutions"]["items"];
        assert_eq!(items[0]["taskExecutionArn"], "arn:task-1/execution/exec-1");
        assert_eq!(items[0]["status"], "SUCCESS");
        assert_eq!(items[0]["startTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["estimatedFilesToTransfer"], 100);
        assert_eq!(items[0]["filesTransferred"], 100);
        assert_eq!(items[0]["bytesTransferred"], 204800);
        assert_eq!(json["dataSyncTaskExecutions"]["nextToken"], "next-execs");
        http_client.relaxed_requests_match();
    }
}
