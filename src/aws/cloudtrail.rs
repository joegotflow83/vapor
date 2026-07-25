use aws_config::SdkConfig;
use aws_sdk_cloudtrail::primitives::DateTime;
use aws_sdk_cloudtrail::types::LookupAttribute;

use crate::error::VaporError;

pub struct CloudTrailClient {
    inner: aws_sdk_cloudtrail::Client,
}

impl CloudTrailClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudtrail::Client::new(config),
        }
    }

    pub async fn describe_trails(
        &self,
    ) -> Result<Vec<aws_sdk_cloudtrail::types::Trail>, VaporError> {
        let output = self
            .inner
            .describe_trails()
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.trail_list().to_vec())
    }

    pub async fn get_trail_status(
        &self,
        name: &str,
    ) -> Result<aws_sdk_cloudtrail::operation::get_trail_status::GetTrailStatusOutput, VaporError>
    {
        self.inner
            .get_trail_status()
            .name(name)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Looks up CloudTrail events, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `LookupEventsInput::max_results` (this operation's `limit`-equivalent) so
    /// a capped page boundary lands exactly on the returned token, matching
    /// `specs/plan-2-schema-v2-pagination-timestamps.md`'s client-layer pattern
    /// (no generated paginator exists for `LookupEvents` in `aws-sdk-cloudtrail`
    /// 1.112.0 — `operation/lookup_events/` has no `paginator.rs` — so this loop
    /// is required, not just a style choice).
    pub async fn lookup_events(
        &self,
        start_time: DateTime,
        end_time: DateTime,
        attributes: Vec<LookupAttribute>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_cloudtrail::types::Event>, Option<String>), VaporError> {
        let mut events = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .lookup_events()
                .start_time(start_time)
                .end_time(end_time);

            for attr in &attributes {
                req = req.lookup_attributes(attr.clone());
            }
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
    use aws_sdk_cloudtrail::types::LookupAttributeKey;

    const ENDPOINT: &str = "https://cloudtrail.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_trails_returns_all_trails() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"trailList":[{"Name":"trail-1","S3BucketName":"bucket1","IsMultiRegionTrail":true},{"Name":"trail-2","S3BucketName":"bucket2"}]}"#,
            ),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let trails = client.describe_trails().await.unwrap();

        assert_eq!(trails.len(), 2);
        assert_eq!(trails[0].name(), Some("trail-1"));
        assert_eq!(trails[0].s3_bucket_name(), Some("bucket1"));
        assert_eq!(trails[0].is_multi_region_trail(), Some(true));
        assert_eq!(trails[1].name(), Some("trail-2"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_trails_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("OperationNotPermittedException", "not permitted"),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_trails().await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("OperationNotPermittedException".to_string()));
                assert_eq!(message, "not permitted");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_trail_status_returns_status() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Name":"my-trail"}"#),
            json_response(
                200,
                r#"{"IsLogging":true,"LatestDeliveryTime":1700000000,"StartLoggingTime":1699990000}"#,
            ),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let status = client.get_trail_status("my-trail").await.unwrap();

        assert_eq!(status.is_logging(), Some(true));
        assert_eq!(
            status.latest_delivery_time(),
            Some(&DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(
            status.start_logging_time(),
            Some(&DateTime::from_secs(1_699_990_000))
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_trail_status_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Name":"missing-trail"}"#),
            json_error_response("TrailNotFoundException", "trail not found"),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let err = client.get_trail_status("missing-trail").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("TrailNotFoundException".to_string()));
                assert_eq!(message, "trail not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lookup_events_returns_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StartTime":1700000000,"EndTime":1700003600}"#),
            json_response(
                200,
                r#"{"Events":[{"EventId":"e1","EventName":"ConsoleLogin","Username":"alice","EventTime":1700000100}]}"#,
            ),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let (events, token) = client
            .lookup_events(
                DateTime::from_secs(1_700_000_000),
                DateTime::from_secs(1_700_003_600),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id(), Some("e1"));
        assert_eq!(events[0].event_name(), Some("ConsoleLogin"));
        assert_eq!(events[0].username(), Some("alice"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lookup_events_passes_attributes_and_resumes_from_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"LookupAttributes":[{"AttributeKey":"EventName","AttributeValue":"ConsoleLogin"}],"StartTime":1700000000,"EndTime":1700003600,"NextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"Events":[{"EventId":"e2"}]}"#),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let attributes = vec![LookupAttribute::builder()
            .attribute_key(LookupAttributeKey::EventName)
            .attribute_value("ConsoleLogin")
            .build()
            .unwrap()];

        let (events, token) = client
            .lookup_events(
                DateTime::from_secs(1_700_000_000),
                DateTime::from_secs(1_700_003_600),
                attributes,
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id(), Some("e2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lookup_events_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"StartTime":1700000000,"EndTime":1700003600,"MaxResults":2}"#,
            ),
            json_response(
                200,
                r#"{"Events":[{"EventId":"e1"},{"EventId":"e2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let (events, token) = client
            .lookup_events(
                DateTime::from_secs(1_700_000_000),
                DateTime::from_secs(1_700_003_600),
                vec![],
                Some(2),
                None,
            )
            .await
            .unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lookup_events_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"StartTime":1700000000,"EndTime":1700003600,"MaxResults":10}"#,
                ),
                json_response(
                    200,
                    r#"{"Events":[{"EventId":"e1"},{"EventId":"e2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"StartTime":1700000000,"EndTime":1700003600,"NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(200, r#"{"Events":[{"EventId":"e3"}]}"#),
            ),
        ]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let (events, token) = client
            .lookup_events(
                DateTime::from_secs(1_700_000_000),
                DateTime::from_secs(1_700_003_600),
                vec![],
                Some(10),
                None,
            )
            .await
            .unwrap();

        assert_eq!(events.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn lookup_events_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"StartTime":1700000000,"EndTime":1700003600}"#),
            json_error_response("InvalidTimeRangeException", "start time after end time"),
        )]);
        let client = CloudTrailClient::new(&sdk_config(http_client.clone()));

        let err = client
            .lookup_events(
                DateTime::from_secs(1_700_000_000),
                DateTime::from_secs(1_700_003_600),
                vec![],
                None,
                None,
            )
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidTimeRangeException".to_string()));
                assert_eq!(message, "start time after end time");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
