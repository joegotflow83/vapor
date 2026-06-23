use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use aws_sdk_emr::types::ClusterState;

use crate::aws::emr::EmrClient;
use crate::schema::emr::types::{EmrCluster, EmrStep};

#[derive(Default)]
pub struct EmrQuery;

#[Object]
impl EmrQuery {
    async fn emr_clusters(
        &self,
        ctx: &Context<'_>,
        states: Option<Vec<String>>,
    ) -> Result<Vec<EmrCluster>> {
        let client = ctx.data::<EmrClient>()?;

        let cluster_states: Option<Vec<ClusterState>> = states.map(|ss| {
            ss.into_iter()
                .map(|s| ClusterState::from(s.as_str()))
                .collect()
        });

        let summaries = client.list_clusters(cluster_states).await?;

        let futures: Vec<_> = summaries
            .iter()
            .filter_map(|s| s.id())
            .map(|id| client.describe_cluster(id))
            .collect();

        let results = join_all(futures).await;

        let clusters = results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(EmrCluster::from)
            .collect();

        Ok(clusters)
    }

    async fn emr_steps(
        &self,
        ctx: &Context<'_>,
        cluster_id: String,
    ) -> Result<Vec<EmrStep>> {
        let client = ctx.data::<EmrClient>()?;
        let steps = client.list_steps(&cluster_id).await?;
        Ok(steps.into_iter().map(EmrStep::from).collect())
    }
}
