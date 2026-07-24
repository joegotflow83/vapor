use async_graphql::{Context, Object, Result};

use crate::aws::health::HealthClient;
use crate::schema::health::types::HealthEvent;
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct HealthQuery;

#[Object]
impl HealthQuery {
    /// Lists Health events, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn health_events(
        &self,
        ctx: &Context<'_>,
        status_codes: Option<Vec<String>>,
        services: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<HealthEvent>> {
        let client = ctx.data::<HealthClient>()?;
        let (events, next_token) = client
            .describe_events(status_codes, services, limit, next_token)
            .await?;
        Ok(Page {
            items: events.into_iter().map(HealthEvent::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::health::HealthClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::HealthQuery;

    const ENDPOINT: &str = "https://health.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn health_events_maps_items_and_forwards_limit_and_next_token() {
        // `limit` must match the single mocked item count even though the
        // response also carries a `NextToken` — `describe_events`'s
        // hand-rolled loop only stops once `events.len() >= limit`, so a
        // higher `limit` here would over-page past this one mocked
        // `ReplayEvent` (same gotcha noted for `acm_pca`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"filter":{},"maxResults":1}"#),
            json_response(
                200,
                r#"{"events":[{"arn":"arn:aws:health:us-east-1::event/EC2/AWS_EC2_OPERATIONAL_ISSUE/1","service":"EC2","eventTypeCode":"AWS_EC2_OPERATIONAL_ISSUE","eventTypeCategory":"issue","region":"us-east-1","statusCode":"open","startTime":1700000000,"endTime":1700003600,"lastUpdatedTime":1700003600}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(HealthQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ healthEvents(limit: 1) { items { arn service eventTypeCode eventTypeCategory region status startTime endTime lastUpdatedTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let item = &data["healthEvents"]["items"][0];
        assert_eq!(
            item["arn"],
            "arn:aws:health:us-east-1::event/EC2/AWS_EC2_OPERATIONAL_ISSUE/1"
        );
        assert_eq!(item["service"], "EC2");
        assert_eq!(item["eventTypeCode"], "AWS_EC2_OPERATIONAL_ISSUE");
        assert_eq!(item["eventTypeCategory"], "issue");
        assert_eq!(item["region"], "us-east-1");
        assert_eq!(item["status"], "open");
        assert_eq!(item["startTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(item["endTime"], "2023-11-14T23:13:20+00:00");
        assert_eq!(item["lastUpdatedTime"], "2023-11-14T23:13:20+00:00");
        assert_eq!(data["healthEvents"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn health_events_forwards_status_and_service_filters() {
        // "bogus" isn't a recognized status code so it's filtered out before
        // reaching the filter builder, same as `health.rs`'s own aws-layer
        // test.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"filter":{"eventStatusCodes":["open"],"services":["EC2"]}}"#,
            ),
            json_response(200, r#"{"events":[{"arn":"e1","service":"EC2"}]}"#),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(HealthQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ healthEvents(statusCodes: ["open", "bogus"], services: ["EC2"]) { items { arn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["healthEvents"]["items"][0]["arn"], "e1");
        assert!(data["healthEvents"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn health_events_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"filter":{}}"#),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(HealthQuery).data(client).finish();

        let res = schema
            .execute(r#"{ healthEvents { items { arn } } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("bad request"));
        http_client.relaxed_requests_match();
    }
}
