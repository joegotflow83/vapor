use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::firehose::FirehoseClient;
use crate::schema::firehose::types::{FirehoseDeliveryStream, FirehoseTag};

#[derive(Default)]
pub struct FirehoseQuery;

#[Object]
impl FirehoseQuery {
    async fn firehose_delivery_streams(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<FirehoseDeliveryStream>> {
        let client = ctx.data::<FirehoseClient>()?;
        let names = client.list_delivery_streams().await?;
        let futures = names.iter().map(|name| async {
            let desc = client.describe_delivery_stream(name).await?;
            let sdk_tags = client.list_tags_for_delivery_stream(name).await?;
            let tags: Vec<FirehoseTag> = sdk_tags.into_iter().map(FirehoseTag::from).collect();
            Ok::<FirehoseDeliveryStream, async_graphql::Error>(
                FirehoseDeliveryStream::from_description(desc, tags),
            )
        });
        let results: Vec<_> = join_all(futures).await;
        let mut streams = Vec::new();
        for r in results {
            streams.push(r?);
        }
        Ok(streams)
    }

    async fn firehose_delivery_stream(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<Option<FirehoseDeliveryStream>> {
        let client = ctx.data::<FirehoseClient>()?;
        match client.describe_delivery_stream(&name).await {
            Ok(desc) => {
                let sdk_tags = client.list_tags_for_delivery_stream(&name).await?;
                let tags: Vec<FirehoseTag> =
                    sdk_tags.into_iter().map(FirehoseTag::from).collect();
                Ok(Some(FirehoseDeliveryStream::from_description(desc, tags)))
            }
            Err(_) => Ok(None),
        }
    }
}
