use async_graphql::{Context, Object, Result};

use crate::aws::detective::DetectiveClient;
use crate::schema::detective::types::{
    DetectiveDatasourcePackage, DetectiveGraph, DetectiveMember,
};

#[derive(Default)]
pub struct DetectiveQuery;

#[Object]
impl DetectiveQuery {
    async fn detective_graphs(&self, ctx: &Context<'_>) -> Result<Vec<DetectiveGraph>> {
        let client = ctx.data::<DetectiveClient>()?;
        let graphs = client.list_graphs().await?;
        Ok(graphs.into_iter().map(DetectiveGraph::from).collect())
    }

    async fn detective_members(
        &self,
        ctx: &Context<'_>,
        graph_arn: String,
    ) -> Result<Vec<DetectiveMember>> {
        let client = ctx.data::<DetectiveClient>()?;
        let members = client.list_members(graph_arn).await?;
        Ok(members.into_iter().map(DetectiveMember::from).collect())
    }

    async fn detective_datasource_packages(
        &self,
        ctx: &Context<'_>,
        graph_arn: String,
    ) -> Result<Vec<DetectiveDatasourcePackage>> {
        let client = ctx.data::<DetectiveClient>()?;
        let packages = client.list_datasource_packages(graph_arn).await?;
        Ok(packages
            .into_iter()
            .map(DetectiveDatasourcePackage::from)
            .collect())
    }
}
