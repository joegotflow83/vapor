use aws_config::SdkConfig;
use aws_sdk_health::types::{Event, EventFilter, EventStatusCode};

use crate::error::VaporError;

pub struct HealthClient {
    inner: aws_sdk_health::Client,
}

impl HealthClient {
    pub fn new(config: &SdkConfig) -> Self {
        let health_config = aws_sdk_health::config::Builder::from(config)
            .region(aws_sdk_health::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_health::Client::from_conf(health_config),
        }
    }

    /// Describes AWS Health events, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `DescribeEvents` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-health` 1.107.0's
    /// `operation/describe_events/_describe_events_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `kinesis.rs`'s `list_streams` pattern.
    pub async fn describe_events(
        &self,
        status_codes: Option<Vec<String>>,
        services: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Event>, Option<String>), VaporError> {
        let parsed_codes: Vec<EventStatusCode> = status_codes
            .unwrap_or_default()
            .iter()
            .filter_map(|s| match s.as_str() {
                "open" => Some(EventStatusCode::Open),
                "closed" => Some(EventStatusCode::Closed),
                "upcoming" => Some(EventStatusCode::Upcoming),
                _ => None,
            })
            .collect();

        let services = services.unwrap_or_default();

        let mut fb = EventFilter::builder();
        if !parsed_codes.is_empty() {
            fb = fb.set_event_status_codes(Some(parsed_codes));
        }
        if !services.is_empty() {
            fb = fb.set_services(Some(services));
        }
        let filter = fb.build();

        let mut events = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_events().filter(filter.clone());
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - events.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            events.extend(output.events.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if events.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((events, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://health.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_events_lists_all_when_no_limit_or_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"filter":{}}"#),
            json_response(
                200,
                r#"{"events":[{"arn":"arn:aws:health:us-east-1::event/EC2/AWS_EC2_1/1","service":"EC2","statusCode":"open"},{"arn":"arn:aws:health:us-east-1::event/RDS/AWS_RDS_1/1","service":"RDS","statusCode":"closed"}]}"#,
            ),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_events(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].arn(),
            Some("arn:aws:health:us-east-1::event/EC2/AWS_EC2_1/1")
        );
        assert_eq!(items[0].status_code(), Some(&EventStatusCode::Open));
        assert_eq!(items[1].service(), Some("RDS"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_events_forwards_status_and_service_filters() {
        // "bogus" isn't a recognized status code so it's filtered out before
        // reaching the filter builder.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"filter":{"eventStatusCodes":["open"],"services":["EC2"]}}"#,
            ),
            json_response(200, r#"{"events":[{"arn":"e1","service":"EC2"}]}"#),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .describe_events(
                Some(vec!["open".to_string(), "bogus".to_string()]),
                Some(vec!["EC2".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_events_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"filter":{},"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"events":[{"arn":"e3"}]}"#),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_events(None, None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_events_stops_at_limit_and_returns_resume_token() {
        // DescribeEvents forwards `maxResults` straight to AWS with no
        // client-side truncation, so the canned response must return
        // exactly the requested count, not more (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"filter":{},"maxResults":2}"#),
            json_response(
                200,
                r#"{"events":[{"arn":"e1"},{"arn":"e2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_events(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_events_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"filter":{},"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"events":[{"arn":"e1"},{"arn":"e2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"filter":{},"nextToken":"p2","maxResults":8}"#),
                json_response(200, r#"{"events":[{"arn":"e3"}]}"#),
            ),
        ]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_events(None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_events_propagates_errors() {
        // `BadRequestException`, not a throttling-classified code (durable
        // gotcha 1: those get retried and exhaust the single replay event).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"filter":{}}"#),
            json_error_response("BadRequestException", "bad request"),
        )]);
        let client = HealthClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_events(None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
