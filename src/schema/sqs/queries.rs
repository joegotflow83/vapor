use async_graphql::{Context, Object, Result};

use crate::aws::sqs::SqsClient;
use crate::schema::pagination::Page;
use crate::schema::sqs::types::SqsQueue;

#[derive(Default)]
pub struct SqsQuery;

#[Object]
impl SqsQuery {
    /// List SQS queue URLs. Optionally filter by name prefix, cap the result
    /// count via `limit` (default unlimited), and resume from `next_token`.
    async fn sqs_queues(
        &self,
        ctx: &Context<'_>,
        prefix: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<String>> {
        let client = ctx.data::<SqsClient>()?;
        let (items, next_token) = client
            .list_queues(prefix.as_deref(), limit, next_token)
            .await?;
        Ok(Page { items, next_token })
    }

    /// Fetch full metadata for a single SQS queue by URL.
    async fn sqs_queue(&self, ctx: &Context<'_>, queue_url: String) -> Result<Option<SqsQueue>> {
        let client = ctx.data::<SqsClient>()?;
        let attrs = client.get_queue_attributes(&queue_url).await?;
        let tags = client.list_queue_tags(&queue_url).await?;
        Ok(Some(SqsQueue::from_parts(queue_url, attrs, tags)))
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::sqs::SqsClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::SqsQuery;

    const ENDPOINT: &str = "https://sqs.us-east-1.amazonaws.com/";
    const QUEUE_URL: &str = "https://sqs.us-east-1.amazonaws.com/111111111111/prod-orders";

    // --- sqs_queues (list, forwards prefix + limit) ---

    #[tokio::test]
    async fn sqs_queues_forwards_prefix_and_limit() {
        // `limit: 1` applied proactively (gotcha 29): the mocked response
        // carries a `NextToken`, so a higher limit than the item count would
        // make the client page again and exhaust this single replay event.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"QueueNamePrefix":"prod-","MaxResults":1}"#),
            json_response(
                200,
                format!(r#"{{"QueueUrls":["{QUEUE_URL}"],"NextToken":"page2"}}"#),
            ),
        )]);
        let schema = build_query_schema(SqsQuery)
            .data(SqsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ sqsQueues(prefix: "prod-", limit: 1) { items nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["sqsQueues"]["items"][0], QUEUE_URL);
        assert_eq!(json["sqsQueues"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn sqs_queues_lists_all_with_no_args() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, "{\"QueueUrls\":[]}"),
        )]);
        let schema = build_query_schema(SqsQuery)
            .data(SqsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute("{ sqsQueues { items nextToken } }").await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["sqsQueues"]["items"].as_array().unwrap().len(), 0);
        assert!(json["sqsQueues"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    // --- sqs_queue (combines get_queue_attributes + list_queue_tags) ---

    #[tokio::test]
    async fn sqs_queue_combines_attributes_and_tags() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    format!(r#"{{"QueueUrl":"{QUEUE_URL}","AttributeNames":["All"]}}"#),
                ),
                json_response(
                    200,
                    r#"{"Attributes":{"QueueArn":"arn:aws:sqs:us-east-1:111111111111:prod-orders","VisibilityTimeout":"30"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, format!(r#"{{"QueueUrl":"{QUEUE_URL}"}}"#)),
                json_response(200, r#"{"Tags":{"env":"prod"}}"#),
            ),
        ]);
        let schema = build_query_schema(SqsQuery)
            .data(SqsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ sqsQueue(queueUrl: "{QUEUE_URL}") {{ url arn visibilityTimeoutSeconds tags {{ key value }} }} }}"#
            ))
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["sqsQueue"]["url"], QUEUE_URL);
        assert_eq!(
            json["sqsQueue"]["arn"],
            "arn:aws:sqs:us-east-1:111111111111:prod-orders"
        );
        assert_eq!(json["sqsQueue"]["visibilityTimeoutSeconds"], 30);
        assert_eq!(json["sqsQueue"]["tags"][0]["key"], "env");
        assert_eq!(json["sqsQueue"]["tags"][0]["value"], "prod");
        http_client.relaxed_requests_match();
    }
}
