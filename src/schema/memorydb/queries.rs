use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::memorydb::MemoryDbClient;
use crate::schema::memorydb::types::{MemoryDbCluster, MemoryDbSubnetGroup};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct MemoryDbQuery;

#[Object]
impl MemoryDbQuery {
    async fn memorydb_clusters(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MemoryDbCluster>> {
        let client = ctx.data::<MemoryDbClient>()?;
        let (clusters, next_token) = client.describe_clusters(limit, next_token).await?;
        let items = join_all(clusters.into_iter().map(|c| async {
            let arn = c.arn().unwrap_or_default().to_string();
            let tags = client.list_tags(&arn).await.unwrap_or_default();
            MemoryDbCluster::from_sdk(c, &tags)
        }))
        .await;
        Ok(Page { items, next_token })
    }

    async fn memorydb_subnet_groups(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MemoryDbSubnetGroup>> {
        let client = ctx.data::<MemoryDbClient>()?;
        let (groups, next_token) = client.describe_subnet_groups(limit, next_token).await?;
        let items = join_all(groups.into_iter().map(|sg| async {
            let arn = sg.arn().unwrap_or_default().to_string();
            let tags = client.list_tags(&arn).await.unwrap_or_default();
            MemoryDbSubnetGroup::from_sdk(sg, &tags)
        }))
        .await;
        Ok(Page { items, next_token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    // Crate name `aws-sdk-memorydb` doesn't match the endpoint hostname
    // `memory-db.*` (durable gotcha 3), same as `src/aws/memorydb.rs`'s tests.
    const ENDPOINT: &str = "https://memory-db.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn memorydb_clusters_maps_items_with_tags() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"Clusters":[{"Name":"my-cluster","Status":"available","ARN":"arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"ResourceArn":"arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster"}"#,
                ),
                json_response(200, r#"{"TagList":[{"Key":"env","Value":"prod"}]}"#),
            ),
        ]);
        let schema = build_query_schema(MemoryDbQuery)
            .data(MemoryDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ memorydbClusters { items { name status arn tags { key value } } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data,
            serde_json::json!({
                "memorydbClusters": {
                    "items": [{
                        "name": "my-cluster",
                        "status": "available",
                        "arn": "arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster",
                        "tags": [{"key": "env", "value": "prod"}],
                    }],
                    "nextToken": null,
                }
            })
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn memorydb_clusters_swallows_list_tags_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"Clusters":[{"Name":"my-cluster","ARN":"arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"ResourceArn":"arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster"}"#,
                ),
                json_error_response("ClusterNotFoundFault", "cluster not found"),
            ),
        ]);
        let schema = build_query_schema(MemoryDbQuery)
            .data(MemoryDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ memorydbClusters { items { name tags { key value } } } }")
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data,
            serde_json::json!({
                "memorydbClusters": {
                    "items": [{"name": "my-cluster", "tags": []}],
                }
            })
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn memorydb_clusters_forwards_limit_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":5,"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"NextToken":"cursor-b"}"#),
        )]);
        let schema = build_query_schema(MemoryDbQuery)
            .data(MemoryDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ memorydbClusters(limit: 5, nextToken: "cursor-a") { items { name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data,
            serde_json::json!({
                "memorydbClusters": {"items": [], "nextToken": "cursor-b"}
            })
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn memorydb_subnet_groups_maps_items_with_tags() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"SubnetGroups":[{"Name":"my-subnet-group","VpcId":"vpc-123","ARN":"arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"ResourceArn":"arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group"}"#,
                ),
                json_response(200, r#"{"TagList":[{"Key":"env","Value":"staging"}]}"#),
            ),
        ]);
        let schema = build_query_schema(MemoryDbQuery)
            .data(MemoryDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ memorydbSubnetGroups { items { name vpcId arn tags { key value } } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data,
            serde_json::json!({
                "memorydbSubnetGroups": {
                    "items": [{
                        "name": "my-subnet-group",
                        "vpcId": "vpc-123",
                        "arn": "arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group",
                        "tags": [{"key": "env", "value": "staging"}],
                    }],
                    "nextToken": null,
                }
            })
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn memorydb_subnet_groups_swallows_list_tags_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"SubnetGroups":[{"Name":"my-subnet-group","ARN":"arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"ResourceArn":"arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group"}"#,
                ),
                json_error_response("SubnetGroupNotFoundFault", "subnet group not found"),
            ),
        ]);
        let schema = build_query_schema(MemoryDbQuery)
            .data(MemoryDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ memorydbSubnetGroups { items { name tags { key value } } } }")
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data,
            serde_json::json!({
                "memorydbSubnetGroups": {
                    "items": [{"name": "my-subnet-group", "tags": []}],
                }
            })
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn memorydb_subnet_groups_forwards_limit_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":5,"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"NextToken":"cursor-b"}"#),
        )]);
        let schema = build_query_schema(MemoryDbQuery)
            .data(MemoryDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ memorydbSubnetGroups(limit: 5, nextToken: "cursor-a") { items { name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(
            data,
            serde_json::json!({
                "memorydbSubnetGroups": {"items": [], "nextToken": "cursor-b"}
            })
        );
        http_client.relaxed_requests_match();
    }
}
