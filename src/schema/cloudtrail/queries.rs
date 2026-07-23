use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use aws_sdk_cloudtrail::primitives::DateTime;
use aws_sdk_cloudtrail::types::{LookupAttribute, LookupAttributeKey};

use crate::aws::cloudtrail::CloudTrailClient;
use crate::schema::cloudtrail::types::{CloudTrailEvent, Trail};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CloudTrailQuery;

#[Object]
impl CloudTrailQuery {
    async fn cloudtrail_trails(&self, ctx: &Context<'_>) -> Result<Vec<Trail>> {
        let client = ctx.data::<CloudTrailClient>()?;
        let trails = client.describe_trails().await?;

        let futures: Vec<_> = trails
            .iter()
            .map(|t| async move {
                let name = t.trail_arn().or(t.name()).unwrap_or_default();
                let status = client.get_trail_status(name).await;
                (t, status)
            })
            .collect();

        let results = join_all(futures).await;
        let mut out = Vec::new();
        for (trail, status_result) in results {
            let is_logging = match status_result {
                Ok(status) => status.is_logging().unwrap_or(false),
                Err(_) => false,
            };
            out.push(Trail::from_sdk(trail, is_logging));
        }
        Ok(out)
    }

    async fn cloudtrail_events(
        &self,
        ctx: &Context<'_>,
        start_time: String,
        end_time: String,
        event_name: Option<String>,
        username: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CloudTrailEvent>> {
        let client = ctx.data::<CloudTrailClient>()?;

        let start = DateTime::from_secs_f64(
            chrono::DateTime::parse_from_rfc3339(&start_time)
                .map_err(|e| async_graphql::Error::new(format!("Invalid startTime: {e}")))?
                .timestamp() as f64,
        );
        let end = DateTime::from_secs_f64(
            chrono::DateTime::parse_from_rfc3339(&end_time)
                .map_err(|e| async_graphql::Error::new(format!("Invalid endTime: {e}")))?
                .timestamp() as f64,
        );

        let mut attributes = Vec::new();
        if let Some(ref name) = event_name {
            attributes.push(
                LookupAttribute::builder()
                    .attribute_key(LookupAttributeKey::EventName)
                    .attribute_value(name)
                    .build()
                    .map_err(|e| async_graphql::Error::new(format!("Failed to build attribute: {e}")))?,
            );
        }
        if let Some(ref user) = username {
            attributes.push(
                LookupAttribute::builder()
                    .attribute_key(LookupAttributeKey::Username)
                    .attribute_value(user)
                    .build()
                    .map_err(|e| async_graphql::Error::new(format!("Failed to build attribute: {e}")))?,
            );
        }

        let (events, token) = client
            .lookup_events(start, end, attributes, limit, next_token)
            .await?;
        Ok(Page {
            items: events.into_iter().map(CloudTrailEvent::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::cloudtrail::CloudTrailClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CloudTrailQuery;

    const ENDPOINT: &str = "https://cloudtrail.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn cloudtrail_trails_maps_status_and_falls_back_to_name_when_arn_missing() {
        // `describe_trails` (discovery), then per-trail `get_trail_status` in
        // the same order as the returned trail list — the resolver's `name`
        // arg is `trail_arn().or(name())`, so trail-1 (has an arn) is looked
        // up by arn while trail-2 (no arn) falls back to its name.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"trailList":[{"Name":"t1","TrailARN":"arn1","S3BucketName":"b1"},{"Name":"t2","S3BucketName":"b2"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Name":"arn1"}"#),
                json_response(200, r#"{"IsLogging":true}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Name":"t2"}"#),
                json_response(200, r#"{"IsLogging":false}"#),
            ),
        ]);
        let schema = build_query_schema(CloudTrailQuery)
            .data(CloudTrailClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ cloudtrailTrails { name arn s3BucketName isLogging } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let trails = json["cloudtrailTrails"].as_array().unwrap();
        assert_eq!(trails.len(), 2);
        assert_eq!(trails[0]["name"], "t1");
        assert_eq!(trails[0]["arn"], "arn1");
        assert_eq!(trails[0]["isLogging"], true);
        assert_eq!(trails[1]["name"], "t2");
        assert!(trails[1]["arn"].is_null());
        assert_eq!(trails[1]["isLogging"], false);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cloudtrail_trails_treats_status_error_as_not_logging_but_keeps_trail() {
        // Unlike acm's discovery+fan-out (which drops a not-found item), a
        // `get_trail_status` error here is swallowed via `match ... Err(_) =>
        // false` — the trail still comes back, just with `isLogging: false`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"trailList":[{"Name":"t1","TrailARN":"arn1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Name":"arn1"}"#),
                json_error_response("TrailNotFoundException", "trail not found"),
            ),
        ]);
        let schema = build_query_schema(CloudTrailQuery)
            .data(CloudTrailClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ cloudtrailTrails { name isLogging } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let trails = json["cloudtrailTrails"].as_array().unwrap();
        assert_eq!(trails.len(), 1);
        assert_eq!(trails[0]["name"], "t1");
        assert_eq!(trails[0]["isLogging"], false);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cloudtrail_events_builds_lookup_attributes_and_forwards_pagination() {
        // `event_name`/`username` args each build one `LookupAttribute`
        // (event_name pushed first, matching the resolver's own push order),
        // and `limit`/`next_token` forward straight into
        // `lookup_events`'s `MaxResults`/`NextToken`. `limit: 1` matches the
        // single mocked event so the client's own internal pagination loop
        // (see `CloudTrailClient::lookup_events`) stops after one request
        // instead of issuing a second one for the mocked `NextToken`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"LookupAttributes":[{"AttributeKey":"EventName","AttributeValue":"ConsoleLogin"},{"AttributeKey":"Username","AttributeValue":"alice"}],"StartTime":1700000000,"EndTime":1700003600,"NextToken":"cursor-a","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Events":[{"EventId":"evt-1","EventName":"ConsoleLogin","EventSource":"signin.amazonaws.com","Username":"alice","AccessKeyId":"AKIA1","ReadOnly":"true","EventTime":1700000100,"Resources":[{"ResourceType":"AWS::IAM::User","ResourceName":"alice"}]}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(CloudTrailQuery)
            .data(CloudTrailClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cloudtrailEvents(startTime: "2023-11-14T22:13:20Z", endTime: "2023-11-14T23:13:20Z", eventName: "ConsoleLogin", username: "alice", limit: 1, nextToken: "cursor-a") { items { eventId eventName eventSource username accessKeyId readOnly resources { resourceType resourceName } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cloudtrailEvents"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["eventId"], "evt-1");
        assert_eq!(items[0]["eventName"], "ConsoleLogin");
        assert_eq!(items[0]["eventSource"], "signin.amazonaws.com");
        assert_eq!(items[0]["username"], "alice");
        assert_eq!(items[0]["accessKeyId"], "AKIA1");
        assert_eq!(items[0]["readOnly"], "true");
        assert_eq!(items[0]["resources"][0]["resourceType"], "AWS::IAM::User");
        assert_eq!(items[0]["resources"][0]["resourceName"], "alice");
        assert_eq!(json["cloudtrailEvents"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cloudtrail_events_invalid_start_time_returns_error_without_calling_aws() {
        let http_client = StaticReplayClient::new(vec![]);
        let schema = build_query_schema(CloudTrailQuery)
            .data(CloudTrailClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cloudtrailEvents(startTime: "not-a-date", endTime: "2023-11-14T23:13:20Z") { items { eventId } } }"#,
            )
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("Invalid startTime"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cloudtrail_events_invalid_end_time_returns_error_without_calling_aws() {
        let http_client = StaticReplayClient::new(vec![]);
        let schema = build_query_schema(CloudTrailQuery)
            .data(CloudTrailClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cloudtrailEvents(startTime: "2023-11-14T22:13:20Z", endTime: "not-a-date") { items { eventId } } }"#,
            )
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("Invalid endTime"));
        http_client.relaxed_requests_match();
    }
}
