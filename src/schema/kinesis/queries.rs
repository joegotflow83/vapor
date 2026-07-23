use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::kinesis::KinesisClient;
use crate::schema::kinesis::types::{DataStream, Shard, Tag};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct KinesisQuery;

#[Object]
impl KinesisQuery {
    /// Lists Kinesis data streams, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn kinesis_streams(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DataStream>> {
        let client = ctx.data::<KinesisClient>()?;
        let (names, token) = client.list_streams(limit, next_token).await?;
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
        Ok(Page { items: streams, next_token: token })
    }

    /// Lists shards for a stream, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn kinesis_shards(
        &self,
        ctx: &Context<'_>,
        stream_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Shard>> {
        let client = ctx.data::<KinesisClient>()?;
        let (shards, token) = client.list_shards(&stream_name, limit, next_token).await?;
        Ok(Page {
            items: shards.into_iter().map(Shard::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::kinesis::KinesisClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::KinesisQuery;

    const ENDPOINT: &str = "https://kinesis.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn kinesis_streams_maps_fan_out_summary_and_tags() {
        // Three sequential calls per the resolver's own code: `list_streams`
        // (discovery), then per-name `describe_stream_summary` +
        // `list_tags_for_stream` (fan-out via `join_all`) — the mock
        // connector serves responses strictly in request order, and
        // `StaticReplayClient`'s queue lines up with `join_all`'s
        // per-future poll order on the single-threaded test runtime (acm/
        // elasticache precedent), so this list mirrors that order exactly.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"cursor-a","Limit":1}"#),
                json_response(
                    200,
                    r#"{"StreamNames":["my-stream"],"HasMoreStreams":true,"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
                json_response(
                    200,
                    r#"{"StreamDescriptionSummary":{"StreamName":"my-stream","StreamARN":"arn:aws:kinesis:us-east-1:123456789012:stream/my-stream","StreamStatus":"ACTIVE","StreamModeDetails":{"StreamMode":"PROVISIONED"},"RetentionPeriodHours":24,"StreamCreationTimestamp":1700000000,"EnhancedMonitoring":[],"OpenShardCount":4,"EncryptionType":"KMS","KeyId":"key1"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"StreamName":"my-stream"}"#),
                json_response(200, r#"{"Tags":[{"Key":"env","Value":"prod"}],"HasMoreTags":false}"#),
            ),
        ]);
        let schema = build_query_schema(KinesisQuery)
            .data(KinesisClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ kinesisStreams(limit: 1, nextToken: "cursor-a") { items { name arn status streamMode shardCount retentionPeriodHours encryptionType keyId createdAt tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["kinesisStreams"]["items"];
        assert_eq!(items[0]["name"], "my-stream");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:kinesis:us-east-1:123456789012:stream/my-stream"
        );
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["streamMode"], "PROVISIONED");
        assert_eq!(items[0]["shardCount"], 4);
        assert_eq!(items[0]["retentionPeriodHours"], 24);
        assert_eq!(items[0]["encryptionType"], "KMS");
        assert_eq!(items[0]["keyId"], "key1");
        assert_eq!(items[0]["createdAt"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["kinesisStreams"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn kinesis_streams_propagates_describe_error() {
        // `describe_stream_summary(name).await?` short-circuits the
        // per-name future on error (unlike acm's fan-out, which awaits both
        // calls unconditionally before matching) — so no `list_tags_for_stream`
        // call follows a failed describe, and only 2 replay events are needed.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"StreamNames":["bad-stream"],"HasMoreStreams":false}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"StreamName":"bad-stream"}"#),
                json_error_response("ResourceNotFoundException", "stream not found"),
            ),
        ]);
        let schema = build_query_schema(KinesisQuery)
            .data(KinesisClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ kinesisStreams { items { name } nextToken } }")
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("stream not found"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn kinesis_shards_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StreamName":"my-stream","MaxResults":1}"#),
            json_response(
                200,
                r#"{"Shards":[{"ShardId":"shardId-000000000000","ParentShardId":"shardId-parent","HashKeyRange":{"StartingHashKey":"0","EndingHashKey":"170141183460469231731687303715884105727"},"SequenceNumberRange":{"StartingSequenceNumber":"49600000000000000000"}}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(KinesisQuery)
            .data(KinesisClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ kinesisShards(streamName: "my-stream", limit: 1) { items { shardId parentShardId startingHashKey endingHashKey startingSequenceNumber endingSequenceNumber } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["kinesisShards"]["items"];
        assert_eq!(items[0]["shardId"], "shardId-000000000000");
        assert_eq!(items[0]["parentShardId"], "shardId-parent");
        assert_eq!(items[0]["startingHashKey"], "0");
        assert_eq!(
            items[0]["endingHashKey"],
            "170141183460469231731687303715884105727"
        );
        assert_eq!(items[0]["startingSequenceNumber"], "49600000000000000000");
        assert!(items[0]["endingSequenceNumber"].is_null());
        assert_eq!(json["kinesisShards"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
