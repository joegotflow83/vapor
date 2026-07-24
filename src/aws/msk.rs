#[cfg(feature = "kafka")]
use aws_config::SdkConfig;
#[cfg(feature = "kafka")]
use aws_sdk_kafka::types::{Cluster, NodeInfo};

#[cfg(feature = "kafka")]
use crate::error::VaporError;

#[cfg(feature = "kafka")]
pub struct MskClient {
    inner: aws_sdk_kafka::Client,
}

impl MskClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_kafka::Client::new(config),
        }
    }

    /// Lists MSK clusters, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListClustersV2` has a
    /// generated paginator but it hides the token, so this is hand-rolled
    /// (detective.rs pattern); `limit` caps via
    /// `ListClustersV2Input::max_results` so a capped page boundary lands
    /// exactly on the returned token (kinesis.rs pattern).
    pub async fn list_clusters_v2(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Cluster>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_clusters_v2();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.cluster_info_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists nodes for a cluster, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListNodes` has a
    /// generated paginator but it hides the token, so this is hand-rolled
    /// (detective.rs pattern); same `max_results` capping as
    /// `list_clusters_v2`.
    pub async fn list_nodes(
        &self,
        cluster_arn: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<NodeInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_nodes().cluster_arn(cluster_arn);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.node_info_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(feature = "kafka")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://kafka.us-east-1.amazonaws.com";
    const CLUSTER_ARN: &str = "arn:aws:kafka:us-east-1:123456789012:cluster/c1/abc-123";
    const CLUSTER_ARN_ENCODED: &str =
        "arn%3Aaws%3Akafka%3Aus-east-1%3A123456789012%3Acluster%2Fc1%2Fabc-123";

    #[tokio::test]
    async fn list_clusters_v2_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/api/v2/clusters"), ""),
            json_response(
                200,
                r#"{"clusterInfoList":[{"clusterArn":"arn:aws:kafka:us-east-1:123456789012:cluster/c1/abc","clusterName":"cluster-one","clusterType":"PROVISIONED","state":"ACTIVE","creationTime":"2024-01-01T00:00:00Z","tags":{"env":"prod"}},{"clusterArn":"arn:c2"}]}"#,
            ),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_clusters_v2(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let c1 = &items[0];
        assert_eq!(
            c1.cluster_arn(),
            Some("arn:aws:kafka:us-east-1:123456789012:cluster/c1/abc")
        );
        assert_eq!(c1.cluster_name(), Some("cluster-one"));
        assert_eq!(
            c1.cluster_type(),
            Some(&aws_sdk_kafka::types::ClusterType::Provisioned)
        );
        assert_eq!(
            c1.state(),
            Some(&aws_sdk_kafka::types::ClusterState::Active)
        );
        assert_eq!(
            c1.creation_time(),
            Some(
                &aws_smithy_types::DateTime::from_str(
                    "2024-01-01T00:00:00Z",
                    aws_smithy_types::date_time::Format::DateTimeWithOffset
                )
                .unwrap()
            )
        );
        assert_eq!(c1.tags().unwrap().get("env"), Some(&"prod".to_string()));

        let c2 = &items[1];
        assert_eq!(c2.cluster_arn(), Some("arn:c2"));
        assert_eq!(c2.cluster_name(), None);
        assert_eq!(c2.state(), None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_v2_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/api/v2/clusters?nextToken=cursor-a"), ""),
            json_response(200, r#"{"clusterInfoList":[{"clusterArn":"arn:c3"}]}"#),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_clusters_v2(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_v2_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/api/v2/clusters?maxResults=2"), ""),
            json_response(
                200,
                r#"{"clusterInfoList":[{"clusterArn":"arn:c1"},{"clusterArn":"arn:c2"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_clusters_v2(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_v2_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/api/v2/clusters?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"clusterInfoList":[{"clusterArn":"arn:c1"},{"clusterArn":"arn:c2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/api/v2/clusters?maxResults=8&nextToken=p2"),
                    "",
                ),
                json_response(200, r#"{"clusterInfoList":[{"clusterArn":"arn:c3"}]}"#),
            ),
        ]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_clusters_v2(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_clusters_v2_propagates_errors() {
        // `BadRequestException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/api/v2/clusters"), ""),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let err = client.list_clusters_v2(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodes_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes"),
                "",
            ),
            json_response(
                200,
                r#"{"nodeInfoList":[{"nodeARN":"arn:node1","nodeType":"BROKER","instanceType":"kafka.m5.large","addedToClusterTime":"2024-01-01T00:00:00Z","brokerNodeInfo":{"brokerId":1.0,"clientVpcIpAddress":"10.0.0.5","attachedENIId":"eni-1","endpoints":["b-1.cluster:9092"]}},{"nodeARN":"arn:node2"}]}"#,
            ),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_nodes(CLUSTER_ARN, None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let n1 = &items[0];
        assert_eq!(n1.node_arn(), Some("arn:node1"));
        assert_eq!(
            n1.node_type(),
            Some(&aws_sdk_kafka::types::NodeType::Broker)
        );
        assert_eq!(n1.instance_type(), Some("kafka.m5.large"));
        let broker_info = n1.broker_node_info().unwrap();
        assert_eq!(broker_info.broker_id(), Some(1.0));
        assert_eq!(broker_info.client_vpc_ip_address(), Some("10.0.0.5"));
        assert_eq!(broker_info.attached_eni_id(), Some("eni-1"));
        assert_eq!(broker_info.endpoints(), ["b-1.cluster:9092".to_string()]);

        let n2 = &items[1];
        assert_eq!(n2.node_arn(), Some("arn:node2"));
        assert!(n2.broker_node_info().is_none());

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodes_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes?nextToken=cursor-a"),
                "",
            ),
            json_response(200, r#"{"nodeInfoList":[{"nodeARN":"arn:node3"}]}"#),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_nodes(CLUSTER_ARN, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodes_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes?maxResults=2"),
                "",
            ),
            json_response(
                200,
                r#"{"nodeInfoList":[{"nodeARN":"arn:node1"},{"nodeARN":"arn:node2"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_nodes(CLUSTER_ARN, Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodes_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes?maxResults=10"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"nodeInfoList":[{"nodeARN":"arn:node1"},{"nodeARN":"arn:node2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!(
                        "{BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes?maxResults=8&nextToken=p2"
                    ),
                    "",
                ),
                json_response(200, r#"{"nodeInfoList":[{"nodeARN":"arn:node3"}]}"#),
            ),
        ]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_nodes(CLUSTER_ARN, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_nodes_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes"),
                "",
            ),
            json_error_response("NotFoundException", "cluster not found"),
        )]);
        let client = MskClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_nodes(CLUSTER_ARN, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFoundException".to_string()));
                assert_eq!(message, "cluster not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
