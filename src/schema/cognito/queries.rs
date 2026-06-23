use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::cognito::CognitoClient;
use crate::schema::cognito::types::{UserPool, UserPoolClient};

#[derive(Default)]
pub struct CognitoQuery;

#[Object]
impl CognitoQuery {
    async fn cognito_user_pools(&self, ctx: &Context<'_>) -> Result<Vec<UserPool>> {
        let client = ctx.data::<CognitoClient>()?;
        let pools = client.list_user_pools().await?;

        let futures: Vec<_> = pools
            .iter()
            .map(|p| async {
                let id = p.id().unwrap_or_default();
                client.describe_user_pool(id).await
            })
            .collect();

        let results = join_all(futures).await;
        let mut out = Vec::new();
        for result in results {
            if let Ok(pool) = result {
                out.push(UserPool::from_sdk(&pool));
            }
        }
        Ok(out)
    }

    async fn cognito_user_pool_clients(
        &self,
        ctx: &Context<'_>,
        user_pool_id: String,
    ) -> Result<Vec<UserPoolClient>> {
        let client = ctx.data::<CognitoClient>()?;
        let descriptions = client.list_user_pool_clients(&user_pool_id).await?;

        let futures: Vec<_> = descriptions
            .iter()
            .filter_map(|d| {
                let cid = d.client_id()?;
                Some(client.describe_user_pool_client(&user_pool_id, cid))
            })
            .collect();

        let results = join_all(futures).await;
        let out = results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(UserPoolClient::from_sdk)
            .collect();
        Ok(out)
    }
}
