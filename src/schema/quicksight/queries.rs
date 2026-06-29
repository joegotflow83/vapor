use async_graphql::{Context, Object, Result};

use crate::aws::quicksight::QuickSightClient;
use crate::schema::quicksight::types::{
    QuickSightDashboard, QuickSightDataSet, QuickSightDataSource, QuickSightUser,
};

#[derive(Default)]
pub struct QuickSightQuery;

#[Object]
impl QuickSightQuery {
    async fn quick_sight_users(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
        namespace: Option<String>,
    ) -> Result<Vec<QuickSightUser>> {
        let client = ctx.data::<QuickSightClient>()?;
        let users = client.list_users(aws_account_id, namespace).await?;
        Ok(users.into_iter().map(QuickSightUser::from).collect())
    }

    async fn quick_sight_dashboards(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
    ) -> Result<Vec<QuickSightDashboard>> {
        let client = ctx.data::<QuickSightClient>()?;
        let dashboards = client.list_dashboards(aws_account_id).await?;
        Ok(dashboards.into_iter().map(QuickSightDashboard::from).collect())
    }

    async fn quick_sight_data_sets(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
    ) -> Result<Vec<QuickSightDataSet>> {
        let client = ctx.data::<QuickSightClient>()?;
        let data_sets = client.list_data_sets(aws_account_id).await?;
        Ok(data_sets.into_iter().map(QuickSightDataSet::from).collect())
    }

    async fn quick_sight_data_sources(
        &self,
        ctx: &Context<'_>,
        aws_account_id: String,
    ) -> Result<Vec<QuickSightDataSource>> {
        let client = ctx.data::<QuickSightClient>()?;
        let sources = client.list_data_sources(aws_account_id).await?;
        Ok(sources.into_iter().map(QuickSightDataSource::from).collect())
    }
}
