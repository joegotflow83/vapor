use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::firehose::FirehoseClient;
use crate::schema::firehose::types::{FirehoseDeliveryStream, FirehoseTag};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct FirehoseQuery;

#[Object]
impl FirehoseQuery {
    /// Lists delivery streams, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn firehose_delivery_streams(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<FirehoseDeliveryStream>> {
        let client = ctx.data::<FirehoseClient>()?;
        let (names, next_token) = client.list_delivery_streams(limit, next_token).await?;
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
        Ok(Page { items: streams, next_token })
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

#[cfg(test)]
mod tests {
    use crate::aws::firehose::FirehoseClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::FirehoseQuery;

    const ENDPOINT: &str = "https://firehose.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn firehose_delivery_streams_discovers_describes_tags_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":1}"#),
                json_response(
                    200,
                    r#"{"DeliveryStreamNames":["my-stream"],"HasMoreDeliveryStreams":true}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
                json_response(
                    200,
                    r#"{"DeliveryStreamDescription":{"DeliveryStreamName":"my-stream","DeliveryStreamARN":"arn:aws:firehose:us-east-1:1:deliverystream/my-stream","DeliveryStreamStatus":"ACTIVE","DeliveryStreamType":"DirectPut","VersionId":"1","Destinations":[],"HasMoreDestinations":false}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
                json_response(200, r#"{"Tags":[{"Key":"Env","Value":"prod"}],"HasMoreTags":false}"#),
            ),
        ]);
        let schema = build_query_schema(FirehoseQuery)
            .data(FirehoseClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ firehoseDeliveryStreams(limit: 1) { items { name status streamType tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["firehoseDeliveryStreams"]["items"];
        assert_eq!(items[0]["name"], "my-stream");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["streamType"], "DirectPut");
        assert_eq!(items[0]["tags"][0]["key"], "Env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["firehoseDeliveryStreams"]["nextToken"], "my-stream");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn firehose_delivery_stream_forwards_name_and_maps_detail() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
                json_response(
                    200,
                    r#"{"DeliveryStreamDescription":{"DeliveryStreamName":"my-stream","DeliveryStreamARN":"arn:aws:firehose:us-east-1:1:deliverystream/my-stream","DeliveryStreamStatus":"ACTIVE","DeliveryStreamType":"DirectPut","VersionId":"1","Destinations":[],"HasMoreDestinations":false}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"DeliveryStreamName":"my-stream"}"#),
                json_response(200, r#"{"Tags":[],"HasMoreTags":false}"#),
            ),
        ]);
        let schema = build_query_schema(FirehoseQuery)
            .data(FirehoseClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ firehoseDeliveryStream(name: "my-stream") { name arn } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["firehoseDeliveryStream"]["name"], "my-stream");
        assert_eq!(
            json["firehoseDeliveryStream"]["arn"],
            "arn:aws:firehose:us-east-1:1:deliverystream/my-stream"
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn firehose_delivery_stream_returns_none_on_describe_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DeliveryStreamName":"missing"}"#),
            json_error_response("ResourceNotFoundException", "delivery stream not found"),
        )]);
        let schema = build_query_schema(FirehoseQuery)
            .data(FirehoseClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ firehoseDeliveryStream(name: "missing") { name } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["firehoseDeliveryStream"].is_null());
        http_client.relaxed_requests_match();
    }
}
