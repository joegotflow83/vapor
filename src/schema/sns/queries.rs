use async_graphql::{Context, Object, Result};

use crate::aws::sns::SnsClient;
use crate::schema::sns::types::{SnsSubscription, SnsTopic};

#[derive(Default)]
pub struct SnsQuery;

#[Object]
impl SnsQuery {
    /// List all SNS topics with their attributes.
    async fn sns_topics(&self, ctx: &Context<'_>) -> Result<Vec<SnsTopic>> {
        let client = ctx.data::<SnsClient>()?;
        let results = client.list_topics_with_attributes().await?;
        Ok(results
            .into_iter()
            .map(|(arn, attrs)| SnsTopic::from_attrs(arn, attrs))
            .collect())
    }

    /// Fetch a single SNS topic by ARN.
    async fn sns_topic(
        &self,
        ctx: &Context<'_>,
        topic_arn: String,
    ) -> Result<Option<SnsTopic>> {
        let client = ctx.data::<SnsClient>()?;
        let attrs = client.get_topic_attributes(&topic_arn).await?;
        Ok(Some(SnsTopic::from_attrs(topic_arn, attrs)))
    }

    /// List SNS subscriptions. Optionally filter by topicArn.
    async fn sns_subscriptions(
        &self,
        ctx: &Context<'_>,
        topic_arn: Option<String>,
    ) -> Result<Vec<SnsSubscription>> {
        let client = ctx.data::<SnsClient>()?;
        let results = client.list_subscriptions(topic_arn.as_deref()).await?;
        Ok(results.into_iter().map(SnsSubscription::from).collect())
    }
}
