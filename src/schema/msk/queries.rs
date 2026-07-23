use async_graphql::{Context, Object, Result};
use std::collections::{HashMap, HashSet};

use crate::aws::ec2::Ec2Client;
use crate::aws::msk::MskClient;
use crate::schema::msk::types::{BrokerNode, MskCluster};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct MskQuery;

#[Object]
impl MskQuery {
    /// Lists MSK clusters. `limit` caps the total number of results across
    /// pages (default: unlimited); `next_token` resumes from a previous page.
    async fn msk_clusters(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MskCluster>> {
        let client = ctx.data::<MskClient>()?;
        let (clusters, next_token) = client.list_clusters_v2(limit, next_token).await?;
        Ok(Page {
            items: clusters.into_iter().map(MskCluster::from).collect(),
            next_token,
        })
    }

    /// Lists broker nodes for a cluster. `limit` caps the total number of
    /// results across pages (default: unlimited); `next_token` resumes from
    /// a previous page.
    async fn msk_broker_nodes(
        &self,
        ctx: &Context<'_>,
        cluster_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BrokerNode>> {
        let client = ctx.data::<MskClient>()?;
        let ec2_client = ctx.data::<Ec2Client>()?;
        let (nodes, next_token) = client.list_nodes(&cluster_arn, limit, next_token).await?;

        // Collect unique subnet IDs for AZ enrichment (MSK SDK does not expose AZ directly)
        let subnet_ids: Vec<String> = nodes
            .iter()
            .filter_map(|n| {
                n.broker_node_info()
                    .and_then(|b| b.client_subnet().map(|s| s.to_string()))
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Fetch AZ for each subnet via EC2 describe_subnets
        let subnet_to_az: HashMap<String, String> = if !subnet_ids.is_empty() {
            let (subnets, _) = ec2_client
                .describe_subnets(Some(subnet_ids), None, None, None, None)
                .await?;
            subnets
                .into_iter()
                .filter_map(|s| {
                    let id = s.subnet_id()?.to_string();
                    let az = s.availability_zone()?.to_string();
                    Some((id, az))
                })
                .collect()
        } else {
            HashMap::new()
        };

        let broker_nodes = nodes
            .into_iter()
            .map(|n| {
                let subnet = n
                    .broker_node_info()
                    .and_then(|b| b.client_subnet().map(|s| s.to_string()));
                let mut node = BrokerNode::from(n);
                node.az = subnet.as_ref().and_then(|id| subnet_to_az.get(id).cloned());
                node
            })
            .collect();

        Ok(Page {
            items: broker_nodes,
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::ec2::Ec2Client;
    use crate::aws::msk::MskClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::MskQuery;

    const KAFKA_BASE: &str = "https://kafka.us-east-1.amazonaws.com";
    const EC2_ENDPOINT: &str = "https://ec2.us-east-1.amazonaws.com/";
    const CLUSTER_ARN: &str = "arn:aws:kafka:us-east-1:123456789012:cluster/c1/abc-123";
    const CLUSTER_ARN_ENCODED: &str = "arn%3Aaws%3Akafka%3Aus-east-1%3A123456789012%3Acluster%2Fc1%2Fabc-123";

    // `msk_clusters` is a bare passthrough to the already-tested
    // `MskClient::list_clusters_v2` (pagination/limit/error-mapping covered
    // in `src/aws/msk.rs`) — one light smoke test, established pattern.
    #[tokio::test]
    async fn msk_clusters_lists_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{KAFKA_BASE}/api/v2/clusters?maxResults=1"), ""),
            json_response(
                200,
                r#"{"clusterInfoList":[{"clusterArn":"arn:c1","clusterName":"cluster-one","state":"ACTIVE","tags":{"env":"prod"}}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(MskQuery)
            .data(MskClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ mskClusters(limit: 1) { items { arn name state tags { key value } } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["mskClusters"]["items"];
        assert_eq!(items[0]["arn"], "arn:c1");
        assert_eq!(items[0]["name"], "cluster-one");
        assert_eq!(items[0]["state"], "ACTIVE");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["mskClusters"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    // `msk_broker_nodes` has real logic beyond a passthrough: it fans out to
    // `Ec2Client::describe_subnets` to enrich nodes with an AZ, since the
    // Kafka SDK doesn't expose it directly. Two nodes sharing the same
    // subnet both prove the AZ lookup *and* that unique subnet IDs are
    // deduped before the EC2 call (only one `SubnetId` param sent) — using a
    // single shared subnet (rather than two distinct ones) keeps the
    // expected EC2 request body deterministic, since the dedup step collects
    // IDs through a `HashSet` with unspecified iteration order.
    #[tokio::test]
    async fn msk_broker_nodes_enriches_az_and_dedupes_subnet_ids() {
        let msk_http = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{KAFKA_BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes"), ""),
            json_response(
                200,
                r#"{"nodeInfoList":[{"nodeARN":"arn:node1","instanceType":"kafka.m5.large","brokerNodeInfo":{"brokerId":1.0,"clientSubnet":"subnet-1","clientVpcIpAddress":"10.0.0.5","attachedENIId":"eni-1"}},{"nodeARN":"arn:node2","instanceType":"kafka.m5.large","brokerNodeInfo":{"brokerId":2.0,"clientSubnet":"subnet-1"}}]}"#,
            ),
        )]);
        let ec2_http = StaticReplayClient::new(vec![ReplayEvent::new(
            request(EC2_ENDPOINT, "Action=DescribeSubnets&Version=2016-11-15&SubnetId.1=subnet-1"),
            xml_response(
                200,
                "<DescribeSubnetsResponse><subnetSet>\
                 <item><subnetId>subnet-1</subnetId><availabilityZone>us-east-1a</availabilityZone></item>\
                 </subnetSet></DescribeSubnetsResponse>",
            ),
        )]);
        let schema = build_query_schema(MskQuery)
            .data(MskClient::new(&sdk_config(msk_http.clone())))
            .data(Ec2Client::new(&sdk_config(ec2_http.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ mskBrokerNodes(clusterArn: "{CLUSTER_ARN}") {{ items {{ brokerId az clientVpcIp attachedEniId }} }} }}"#
            ))
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["mskBrokerNodes"]["items"];
        assert_eq!(items[0]["brokerId"], 1.0);
        assert_eq!(items[0]["az"], "us-east-1a");
        assert_eq!(items[0]["clientVpcIp"], "10.0.0.5");
        assert_eq!(items[0]["attachedEniId"], "eni-1");
        assert_eq!(items[1]["brokerId"], 2.0);
        assert_eq!(items[1]["az"], "us-east-1a");
        msk_http.relaxed_requests_match();
        ec2_http.relaxed_requests_match();
    }

    // When no node carries a `clientSubnet`, the subnet-ID set is empty and
    // the resolver must skip the EC2 call entirely (`if !subnet_ids.is_empty()`
    // branch) — proven by giving the EC2 client an empty reply queue, which
    // would fail with an unmatched-request error if the resolver called it
    // anyway.
    #[tokio::test]
    async fn msk_broker_nodes_skips_ec2_call_when_no_subnet_info() {
        let msk_http = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{KAFKA_BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes"), ""),
            json_response(
                200,
                r#"{"nodeInfoList":[{"nodeARN":"arn:node1","instanceType":"kafka.m5.large"}]}"#,
            ),
        )]);
        let ec2_http = StaticReplayClient::new(vec![]);
        let schema = build_query_schema(MskQuery)
            .data(MskClient::new(&sdk_config(msk_http.clone())))
            .data(Ec2Client::new(&sdk_config(ec2_http.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ mskBrokerNodes(clusterArn: "{CLUSTER_ARN}") {{ items {{ brokerId az }} }} }}"#
            ))
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["mskBrokerNodes"]["items"][0]["az"], serde_json::Value::Null);
        msk_http.relaxed_requests_match();
        ec2_http.relaxed_requests_match();
    }

    #[tokio::test]
    async fn msk_broker_nodes_propagates_describe_subnets_errors() {
        let msk_http = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{KAFKA_BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes"), ""),
            json_response(
                200,
                r#"{"nodeInfoList":[{"nodeARN":"arn:node1","brokerNodeInfo":{"brokerId":1.0,"clientSubnet":"subnet-1"}}]}"#,
            ),
        )]);
        let ec2_http = StaticReplayClient::new(vec![ReplayEvent::new(
            request(EC2_ENDPOINT, "Action=DescribeSubnets&Version=2016-11-15&SubnetId.1=subnet-1"),
            crate::aws::test_util::ec2_error_response("InvalidSubnetID.NotFound", "no such subnet"),
        )]);
        let schema = build_query_schema(MskQuery)
            .data(MskClient::new(&sdk_config(msk_http.clone())))
            .data(Ec2Client::new(&sdk_config(ec2_http.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ mskBrokerNodes(clusterArn: "{CLUSTER_ARN}") {{ items {{ brokerId }} }} }}"#
            ))
            .await;

        assert!(!res.errors.is_empty(), "expected an error, got none");
        msk_http.relaxed_requests_match();
        ec2_http.relaxed_requests_match();
    }

    #[tokio::test]
    async fn msk_broker_nodes_propagates_list_nodes_errors() {
        let msk_http = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{KAFKA_BASE}/v1/clusters/{CLUSTER_ARN_ENCODED}/nodes"), ""),
            json_error_response("NotFoundException", "cluster not found"),
        )]);
        let ec2_http = StaticReplayClient::new(vec![]);
        let schema = build_query_schema(MskQuery)
            .data(MskClient::new(&sdk_config(msk_http.clone())))
            .data(Ec2Client::new(&sdk_config(ec2_http.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ mskBrokerNodes(clusterArn: "{CLUSTER_ARN}") {{ items {{ brokerId }} }} }}"#
            ))
            .await;

        assert!(!res.errors.is_empty(), "expected an error, got none");
        msk_http.relaxed_requests_match();
        ec2_http.relaxed_requests_match();
    }
}
