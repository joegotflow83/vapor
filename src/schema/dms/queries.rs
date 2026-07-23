use async_graphql::{Context, Object, Result};

use crate::aws::dms::DmsClient;
use crate::schema::dms::types::{DmsEndpoint, DmsReplicationInstance, DmsReplicationTask};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct DmsQuery;

#[Object]
impl DmsQuery {
    /// Lists DMS replication instances. `limit` caps the total number of
    /// results (default unlimited); pass `nextToken` from a prior page to
    /// resume.
    async fn dms_replication_instances(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DmsReplicationInstance>> {
        let client = ctx.data::<DmsClient>()?;
        let (instances, next_token) = client.describe_replication_instances(limit, next_token).await?;
        Ok(Page {
            items: instances.into_iter().map(DmsReplicationInstance::from).collect(),
            next_token,
        })
    }

    /// Lists DMS endpoints, optionally filtered by `endpointType`. `limit`
    /// caps the total number of results (default unlimited); pass
    /// `nextToken` from a prior page to resume.
    async fn dms_endpoints(
        &self,
        ctx: &Context<'_>,
        endpoint_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DmsEndpoint>> {
        let client = ctx.data::<DmsClient>()?;
        let (endpoints, next_token) = client.describe_endpoints(endpoint_type, limit, next_token).await?;
        Ok(Page {
            items: endpoints.into_iter().map(DmsEndpoint::from).collect(),
            next_token,
        })
    }

    /// Lists DMS replication tasks. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn dms_replication_tasks(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DmsReplicationTask>> {
        let client = ctx.data::<DmsClient>()?;
        let (tasks, next_token) = client.describe_replication_tasks(limit, next_token).await?;
        Ok(Page {
            items: tasks.into_iter().map(DmsReplicationTask::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::dms::DmsClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::DmsQuery;

    const ENDPOINT: &str = "https://dms.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn dms_replication_instances_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxRecords":20}"#),
            json_response(
                200,
                r#"{"ReplicationInstances":[{"ReplicationInstanceIdentifier":"my-repl","ReplicationInstanceArn":"arn:aws:dms:us-east-1:1:rep:abc","ReplicationInstanceClass":"dms.t3.medium","ReplicationInstanceStatus":"available","AllocatedStorage":50,"PubliclyAccessible":true,"MultiAZ":false}],"Marker":"cursor-a"}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DmsQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ dmsReplicationInstances(limit: 1) { items { replicationInstanceIdentifier replicationInstanceClass allocatedStorage publiclyAccessible multiAz } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data["dmsReplicationInstances"]["items"][0]["replicationInstanceIdentifier"],
            "my-repl"
        );
        assert_eq!(data["dmsReplicationInstances"]["items"][0]["replicationInstanceClass"], "dms.t3.medium");
        assert_eq!(data["dmsReplicationInstances"]["items"][0]["allocatedStorage"], 50);
        assert_eq!(data["dmsReplicationInstances"]["items"][0]["publiclyAccessible"], true);
        assert_eq!(data["dmsReplicationInstances"]["items"][0]["multiAz"], false);
        assert_eq!(data["dmsReplicationInstances"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn dms_endpoints_forwards_type_filter_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Filters":[{"Name":"endpoint-type","Values":["source"]}]}"#),
            json_response(
                200,
                r#"{"Endpoints":[{"EndpointIdentifier":"my-src","EndpointArn":"arn:aws:dms:us-east-1:1:endpoint:abc","EndpointType":"source","EngineName":"postgres","Status":"active","Port":5432}]}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DmsQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ dmsEndpoints(endpointType: "SOURCE") { items { endpointIdentifier endpointType engineName port } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["dmsEndpoints"]["items"][0]["endpointIdentifier"], "my-src");
        assert_eq!(data["dmsEndpoints"]["items"][0]["endpointType"], "source");
        assert_eq!(data["dmsEndpoints"]["items"][0]["engineName"], "postgres");
        assert_eq!(data["dmsEndpoints"]["items"][0]["port"], 5432);
        assert!(data["dmsEndpoints"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn dms_replication_tasks_maps_items_incl_timestamps() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ReplicationTasks":[{"ReplicationTaskIdentifier":"my-task","ReplicationTaskArn":"arn:aws:dms:us-east-1:1:task:abc","Status":"running","MigrationType":"full-load-and-cdc","SourceEndpointArn":"arn:src","TargetEndpointArn":"arn:tgt","ReplicationInstanceArn":"arn:inst","ReplicationTaskCreationDate":1700000000,"ReplicationTaskStartDate":1700000100}]}"#,
            ),
        )]);
        let client = DmsClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DmsQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ dmsReplicationTasks { items { replicationTaskIdentifier status migrationType sourceEndpointArn targetEndpointArn replicationTaskCreationDate replicationTaskStartDate } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["dmsReplicationTasks"]["items"][0]["replicationTaskIdentifier"], "my-task");
        assert_eq!(data["dmsReplicationTasks"]["items"][0]["status"], "running");
        assert_eq!(data["dmsReplicationTasks"]["items"][0]["migrationType"], "full-load-and-cdc");
        assert_eq!(
            data["dmsReplicationTasks"]["items"][0]["replicationTaskCreationDate"],
            "2023-11-14T22:13:20+00:00"
        );
        assert_eq!(
            data["dmsReplicationTasks"]["items"][0]["replicationTaskStartDate"],
            "2023-11-14T22:15:00+00:00"
        );
        http_client.relaxed_requests_match();
    }
}
