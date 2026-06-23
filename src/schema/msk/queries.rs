use async_graphql::{Context, Object, Result};
use std::collections::{HashMap, HashSet};

use crate::aws::ec2::Ec2Client;
use crate::aws::msk::MskClient;
use crate::schema::msk::types::{BrokerNode, MskCluster};

#[derive(Default)]
pub struct MskQuery;

#[Object]
impl MskQuery {
    async fn msk_clusters(&self, ctx: &Context<'_>) -> Result<Vec<MskCluster>> {
        let client = ctx.data::<MskClient>()?;
        let clusters = client.list_clusters_v2().await?;
        Ok(clusters.into_iter().map(MskCluster::from).collect())
    }

    async fn msk_broker_nodes(
        &self,
        ctx: &Context<'_>,
        cluster_arn: String,
    ) -> Result<Vec<BrokerNode>> {
        let client = ctx.data::<MskClient>()?;
        let ec2_client = ctx.data::<Ec2Client>()?;
        let nodes = client.list_nodes(&cluster_arn).await?;

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
            let subnets = ec2_client
                .describe_subnets(Some(subnet_ids), None, None)
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

        Ok(broker_nodes)
    }
}
