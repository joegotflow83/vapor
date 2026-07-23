use aws_config::SdkConfig;
use aws_sdk_memorydb::types::{Cluster, SubnetGroup, Tag};

use crate::error::VaporError;

pub struct MemoryDbClient {
    inner: aws_sdk_memorydb::Client,
}

impl MemoryDbClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_memorydb::Client::new(config),
        }
    }

    /// Describes MemoryDB clusters, optionally capped at `limit` results (default unlimited).
    pub async fn describe_clusters(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Cluster>, Option<String>), VaporError> {
        let mut req = self.inner.describe_clusters();
        if let Some(limit) = limit {
            req = req.max_results(limit);
        }
        if let Some(token) = next_token {
            req = req.next_token(token);
        }
        let output = req.send().await.map_err(crate::error::sdk_err)?;
        Ok((output.clusters.unwrap_or_default(), output.next_token))
    }

    /// Describes MemoryDB subnet groups, optionally capped at `limit` results (default unlimited).
    pub async fn describe_subnet_groups(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<SubnetGroup>, Option<String>), VaporError> {
        let mut req = self.inner.describe_subnet_groups();
        if let Some(limit) = limit {
            req = req.max_results(limit);
        }
        if let Some(token) = next_token {
            req = req.next_token(token);
        }
        let output = req.send().await.map_err(crate::error::sdk_err)?;
        Ok((output.subnet_groups.unwrap_or_default(), output.next_token))
    }

    pub async fn list_tags(&self, resource_arn: &str) -> Result<Vec<Tag>, VaporError> {
        let output = self
            .inner
            .list_tags()
            .resource_arn(resource_arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.tag_list().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    // Crate name `aws-sdk-memorydb` doesn't match the endpoint hostname
    // `memory-db.*` (durable gotcha 3) — verified against the pinned SDK's
    // `config/endpoint.rs` test fixtures.
    const ENDPOINT: &str = "https://memory-db.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_clusters_returns_items_and_token_with_no_args() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Clusters":[{"Name":"my-cluster","Status":"available","NodeType":"db.r6g.large","ARN":"arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_clusters(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), Some("my-cluster"));
        assert_eq!(items[0].status(), Some("available"));
        assert_eq!(items[0].node_type(), Some("db.r6g.large"));
        assert_eq!(
            items[0].arn(),
            Some("arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster")
        );
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_maps_minimal_cluster_with_no_optional_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"Clusters":[{}]}"#),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_clusters(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), None);
        assert_eq!(items[0].status(), None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_returns_empty_when_response_omits_clusters_field() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, "{}"),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_clusters(None, None).await.unwrap();

        assert_eq!(items, Vec::new());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_forwards_limit_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":5,"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Clusters":[{"Name":"my-cluster-2"}]}"#),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_clusters(Some(5), Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), Some("my-cluster-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_propagates_errors() {
        // `ClusterNotFoundFault`, not a throttling-classified code (durable
        // gotcha 1: those get retried and exhaust the single replay event).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("ClusterNotFoundFault", "cluster not found"),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_clusters(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ClusterNotFoundFault".to_string()));
                assert_eq!(message, "cluster not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnet_groups_returns_items_and_token_with_no_args() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"SubnetGroups":[{"Name":"my-subnet-group","VpcId":"vpc-123","ARN":"arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_subnet_groups(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), Some("my-subnet-group"));
        assert_eq!(items[0].vpc_id(), Some("vpc-123"));
        assert_eq!(
            items[0].arn(),
            Some("arn:aws:memorydb:us-east-1:123456789012:subnetgroup/my-subnet-group")
        );
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnet_groups_maps_minimal_group_with_no_optional_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"SubnetGroups":[{}]}"#),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_subnet_groups(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), None);
        assert_eq!(items[0].vpc_id(), None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnet_groups_returns_empty_when_response_omits_field() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, "{}"),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_subnet_groups(None, None).await.unwrap();

        assert_eq!(items, Vec::new());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnet_groups_forwards_limit_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":5,"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"SubnetGroups":[{"Name":"my-subnet-group-2"}]}"#),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_subnet_groups(Some(5), Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), Some("my-subnet-group-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subnet_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("SubnetGroupNotFoundFault", "subnet group not found"),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_subnet_groups(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("SubnetGroupNotFoundFault".to_string()));
                assert_eq!(message, "subnet group not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_returns_tags() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ResourceArn":"arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster"}"#),
            json_response(200, r#"{"TagList":[{"Key":"env","Value":"prod"}]}"#),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let tags = client
            .list_tags("arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster")
            .await
            .unwrap();

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), Some("env"));
        assert_eq!(tags[0].value(), Some("prod"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_returns_empty_when_response_omits_tag_list_field() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ResourceArn":"arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster"}"#),
            json_response(200, "{}"),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let tags = client
            .list_tags("arn:aws:memorydb:us-east-1:123456789012:cluster/my-cluster")
            .await
            .unwrap();

        assert_eq!(tags, Vec::new());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ResourceArn":"arn:aws:memorydb:us-east-1:123456789012:cluster/missing"}"#),
            json_error_response("ClusterNotFoundFault", "cluster not found"),
        )]);
        let client = MemoryDbClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_tags("arn:aws:memorydb:us-east-1:123456789012:cluster/missing")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ClusterNotFoundFault".to_string()));
                assert_eq!(message, "cluster not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

