use async_graphql::{Context, Object, Result};

use crate::aws::redshift_serverless::RedshiftServerlessClient;
use crate::schema::redshift_serverless::types::{
    RedshiftServerlessNamespace, RedshiftServerlessWorkgroup,
};

#[derive(Default)]
pub struct RedshiftServerlessQuery;

#[Object]
impl RedshiftServerlessQuery {
    async fn redshift_serverless_namespaces(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<RedshiftServerlessNamespace>> {
        let client = ctx.data::<RedshiftServerlessClient>()?;
        let namespaces = client.list_namespaces().await?;
        Ok(namespaces
            .into_iter()
            .map(RedshiftServerlessNamespace::from)
            .collect())
    }

    async fn redshift_serverless_workgroups(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<RedshiftServerlessWorkgroup>> {
        let client = ctx.data::<RedshiftServerlessClient>()?;
        let workgroups = client.list_workgroups().await?;
        Ok(workgroups
            .into_iter()
            .map(RedshiftServerlessWorkgroup::from)
            .collect())
    }
}
