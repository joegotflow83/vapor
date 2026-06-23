use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::cloudfront::CloudFrontClient;
use crate::schema::cloudfront::types::{
    CfDistribution, distribution_from_get, distribution_from_summary, map_tags,
};

#[derive(Default)]
pub struct CloudFrontQuery;

#[Object]
impl CloudFrontQuery {
    /// List all CloudFront distributions with tags fetched concurrently.
    async fn cloudfront_distributions(&self, ctx: &Context<'_>) -> Result<Vec<CfDistribution>> {
        let cf = ctx.data::<CloudFrontClient>()?;
        let summaries = cf.list_distributions().await?;

        let futures: Vec<_> = summaries
            .iter()
            .map(|d| {
                let arn = d.arn().to_string();
                async move {
                    let tags = cf
                        .list_tags_for_resource(&arn)
                        .await
                        .unwrap_or_default();
                    let tags = map_tags(tags);
                    distribution_from_summary(d, tags)
                }
            })
            .collect();

        Ok(join_all(futures).await)
    }

    /// Fetch a single CloudFront distribution by ID.
    async fn cloudfront_distribution(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<CfDistribution>> {
        let cf = ctx.data::<CloudFrontClient>()?;
        let dist = match cf.get_distribution(&id).await? {
            Some(d) => d,
            None => return Ok(None),
        };
        let arn = dist.arn().to_string();
        let tags = cf.list_tags_for_resource(&arn).await.unwrap_or_default();
        let tags = map_tags(tags);
        Ok(Some(distribution_from_get(&dist, tags)))
    }
}
