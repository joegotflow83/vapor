use async_graphql::{Context, Object, Result};

use crate::aws::ecs::EcsClient;
use crate::schema::ecs::types::{Cluster, Service, Task, TaskDefinition};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct EcsQuery;

#[Object]
impl EcsQuery {
    /// List clusters. If cluster_arns is None, returns all clusters.
    /// `limit`/`next_token` paginate the cluster-ARN discovery list before
    /// the describe fan-out; both are no-ops (token always null) if
    /// cluster_arns is given.
    async fn ecs_clusters(
        &self,
        ctx: &Context<'_>,
        cluster_arns: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Cluster>> {
        let client = ctx.data::<EcsClient>()?;
        let (results, token) = client
            .describe_clusters(cluster_arns, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(Cluster::from).collect(),
            next_token: token,
        })
    }

    /// List services in a cluster. cluster must be an ARN or name.
    /// `limit`/`next_token` paginate the service-ARN discovery list before
    /// the describe fan-out; both are no-ops (token always null) if
    /// service_arns is given.
    async fn ecs_services(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        service_arns: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Service>> {
        let client = ctx.data::<EcsClient>()?;
        let (results, token) = client
            .describe_services(&cluster, service_arns, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(Service::from).collect(),
            next_token: token,
        })
    }

    /// List tasks in a cluster, optionally filtered by service and desired status.
    /// desired_status: "RUNNING" | "PENDING" | "STOPPED"
    /// `limit`/`next_token` paginate the task-ARN discovery list before the
    /// describe fan-out.
    async fn ecs_tasks(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        service_arn: Option<String>,
        desired_status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Task>> {
        let client = ctx.data::<EcsClient>()?;
        let (results, token) = client
            .describe_tasks(&cluster, service_arn, desired_status, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(Task::from).collect(),
            next_token: token,
        })
    }

    /// Fetch a single task definition by ARN or family:revision.
    async fn ecs_task_definition(
        &self,
        ctx: &Context<'_>,
        task_definition: String,
    ) -> Result<Option<TaskDefinition>> {
        let client = ctx.data::<EcsClient>()?;
        let result = client.describe_task_definition(&task_definition).await?;
        Ok(result.map(TaskDefinition::from))
    }

    /// List task definition ARNs, optionally filtered by family prefix and status.
    /// status: "ACTIVE" | "INACTIVE" | "DELETE_IN_PROGRESS"
    /// `limit`/`next_token` paginate the returned list.
    async fn ecs_task_definitions(
        &self,
        ctx: &Context<'_>,
        family_prefix: Option<String>,
        status: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<String>> {
        let client = ctx.data::<EcsClient>()?;
        let (items, token) = client
            .list_task_definitions(family_prefix, status, limit, next_token)
            .await?;
        Ok(Page {
            items,
            next_token: token,
        })
    }
}

// All 5 resolvers are 1:1 passthroughs to a single already-tested
// `EcsClient` method each (see `src/aws/ecs.rs`'s own test module for the
// discovery-pagination/describe-fan-out/error-mapping behavior) — only
// light smoke tests are needed here per the resolver-layer sweep's stated
// scope (connect precedent: one test per resolver, `limit` matched to the
// mocked item count so the hand-rolled pagination loop doesn't over-page
// past the mock event list when the response also carries a `nextToken`).
#[cfg(test)]
mod tests {
    use crate::aws::ecs::EcsClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::EcsQuery;

    const ENDPOINT: &str = "https://ecs.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn ecs_clusters_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":1}"#),
                json_response(
                    200,
                    r#"{"clusterArns":["arn:cluster-a"],"nextToken":"cursor-a"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"clusters":["arn:cluster-a"],"include":["TAGS","STATISTICS"]}"#,
                ),
                json_response(
                    200,
                    r#"{"clusters":[{"clusterArn":"arn:cluster-a","clusterName":"a","status":"ACTIVE","runningTasksCount":5,"pendingTasksCount":0,"activeServicesCount":2,"registeredContainerInstancesCount":3}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EcsQuery)
            .data(EcsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ecsClusters(limit: 1) { items { name arn status runningTasksCount pendingTasksCount activeServicesCount registeredContainerInstancesCount } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ecsClusters"]["items"];
        assert_eq!(items[0]["name"], "a");
        assert_eq!(items[0]["arn"], "arn:cluster-a");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["runningTasksCount"], 5);
        assert_eq!(items[0]["pendingTasksCount"], 0);
        assert_eq!(items[0]["activeServicesCount"], 2);
        assert_eq!(items[0]["registeredContainerInstancesCount"], 3);
        assert_eq!(json["ecsClusters"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ecs_services_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"cluster":"my-cluster","maxResults":1}"#),
                json_response(
                    200,
                    r#"{"serviceArns":["arn:svc-a"],"nextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"cluster":"my-cluster","services":["arn:svc-a"],"include":["TAGS"]}"#,
                ),
                json_response(
                    200,
                    r#"{"services":[{"serviceArn":"arn:svc-a","serviceName":"svc-a","status":"ACTIVE","desiredCount":2,"runningCount":2,"pendingCount":0}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EcsQuery)
            .data(EcsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ecsServices(cluster: "my-cluster", limit: 1) { items { name arn status desiredCount runningCount pendingCount } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ecsServices"]["items"];
        assert_eq!(items[0]["name"], "svc-a");
        assert_eq!(items[0]["arn"], "arn:svc-a");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["desiredCount"], 2);
        assert_eq!(items[0]["runningCount"], 2);
        assert_eq!(items[0]["pendingCount"], 0);
        assert_eq!(json["ecsServices"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ecs_tasks_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"cluster":"my-cluster","maxResults":1}"#),
                json_response(
                    200,
                    r#"{"taskArns":["arn:task-a"],"nextToken":"cursor-c"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"cluster":"my-cluster","tasks":["arn:task-a"],"include":["TAGS"]}"#,
                ),
                json_response(
                    200,
                    r#"{"tasks":[{"taskArn":"arn:task-a","lastStatus":"RUNNING","desiredStatus":"RUNNING"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EcsQuery)
            .data(EcsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ecsTasks(cluster: "my-cluster", limit: 1) { items { taskArn lastStatus desiredStatus } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ecsTasks"]["items"];
        assert_eq!(items[0]["taskArn"], "arn:task-a");
        assert_eq!(items[0]["lastStatus"], "RUNNING");
        assert_eq!(items[0]["desiredStatus"], "RUNNING");
        assert_eq!(json["ecsTasks"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ecs_task_definition_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"taskDefinition":"my-family:1"}"#),
            json_response(
                200,
                r#"{"taskDefinition":{"taskDefinitionArn":"arn:td-1","family":"my-family","revision":1,"status":"ACTIVE"}}"#,
            ),
        )]);
        let schema = build_query_schema(EcsQuery)
            .data(EcsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ecsTaskDefinition(taskDefinition: "my-family:1") { family arn revision status } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let td = &json["ecsTaskDefinition"];
        assert_eq!(td["family"], "my-family");
        assert_eq!(td["arn"], "arn:td-1");
        assert_eq!(td["revision"], 1);
        assert_eq!(td["status"], "ACTIVE");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ecs_task_definitions_maps_items_and_forwards_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"familyPrefix":"my-family","status":"ACTIVE"}"#,
            ),
            json_response(200, r#"{"taskDefinitionArns":["arn:td-1","arn:td-2"]}"#),
        )]);
        let schema = build_query_schema(EcsQuery)
            .data(EcsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ ecsTaskDefinitions(familyPrefix: "my-family", status: "ACTIVE") { items nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["ecsTaskDefinitions"]["items"];
        assert_eq!(items[0], "arn:td-1");
        assert_eq!(items[1], "arn:td-2");
        assert_eq!(json["ecsTaskDefinitions"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }
}
