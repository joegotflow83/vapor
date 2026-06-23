use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::kinesis::KinesisClient;
use crate::schema::kinesis::types::{DataStream, Shard, Tag};

#[derive(Default)]
pub struct KinesisQuery;

#[Object]
impl KinesisQuery {
    async fn kinesis_streams(&self, ctx: &Context<'_>) -> Result<Vec<DataStream>> {
        let client = ctx.data::<KinesisClient>()?;
        let names = client.list_streams().await?;
        let futures = names.iter().map(|name| async {
            let summary = client.describe_stream_summary(name).await?;
            let tags = client.list_tags_for_stream(name).await?;
            let tag_list: Vec<Tag> = tags.into_iter().map(Tag::from).collect();
            Ok::<DataStream, async_graphql::Error>(DataStream::from_summary(summary, tag_list))
        });
        let results: Vec<_> = join_all(futures).await;
        let mut streams = Vec::new();
        for r in results {
            streams.push(r?);
        }
        Ok(streams)
    }

    async fn kinesis_shards(&self, ctx: &Context<'_>, stream_name: String) -> Result<Vec<Shard>> {
        let client = ctx.data::<KinesisClient>()?;
        let shards = client.list_shards(&stream_name).await?;
        Ok(shards.into_iter().map(Shard::from).collect())
    }
}
