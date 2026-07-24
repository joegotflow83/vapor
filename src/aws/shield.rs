use aws_config::SdkConfig;
use aws_sdk_shield::error::SdkError;

use crate::error::VaporError;

pub struct ShieldClient {
    inner: aws_sdk_shield::Client,
}

impl ShieldClient {
    pub fn new(config: &SdkConfig) -> Self {
        let shield_config = aws_sdk_shield::config::Builder::from(config)
            .region(aws_sdk_shield::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_shield::Client::from_conf(shield_config),
        }
    }

    pub async fn describe_subscription(
        &self,
    ) -> Result<Option<aws_sdk_shield::types::Subscription>, VaporError> {
        match self.inner.describe_subscription().send().await {
            Ok(output) => Ok(output.subscription().cloned()),
            Err(SdkError::ServiceError(e)) if e.err().is_resource_not_found_exception() => Ok(None),
            Err(e) => Err(crate::error::sdk_err(e)),
        }
    }

    /// Lists Shield protections, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_protections(
        &self,
        resource_arn: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_shield::types::Protection>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_protections();
            if let Some(arn) = resource_arn {
                let filter = aws_sdk_shield::types::InclusionProtectionFilters::builder()
                    .resource_arns(arn)
                    .build();
                req = req.inclusion_filters(filter);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.protections.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists Shield protection groups, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_protection_groups(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_shield::types::ProtectionGroup>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_protection_groups();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.protection_groups);
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists Shield attacks, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_attacks(
        &self,
        resource_arns: Option<Vec<String>>,
        start_time: Option<String>,
        end_time: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_shield::types::AttackSummary>, Option<String>), VaporError> {
        let start_dt = start_time.as_deref().and_then(parse_datetime);
        let end_dt = end_time.as_deref().and_then(parse_datetime);

        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_attacks();

            if let Some(ref arns) = resource_arns {
                for arn in arns {
                    req = req.resource_arns(arn);
                }
            }

            if let Some(dt) = start_dt {
                let time_range = aws_sdk_shield::types::TimeRange::builder()
                    .from_inclusive(dt)
                    .build();
                req = req.start_time(time_range);
            }

            if let Some(dt) = end_dt {
                let time_range = aws_sdk_shield::types::TimeRange::builder()
                    .to_exclusive(dt)
                    .build();
                req = req.end_time(time_range);
            }

            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.attack_summaries.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

fn parse_datetime(s: &str) -> Option<aws_sdk_shield::primitives::DateTime> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| aws_sdk_shield::primitives::DateTime::from_secs(dt.timestamp()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://shield.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_subscription_returns_subscription_when_present() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"Subscription":{"SubscriptionArn":"arn:aws:shield::111111111111:subscription/1","TimeCommitmentInSeconds":31536000,"AutoRenew":"ENABLED"}}"#,
            ),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let sub = client.describe_subscription().await.unwrap().unwrap();

        assert_eq!(
            sub.subscription_arn(),
            Some("arn:aws:shield::111111111111:subscription/1")
        );
        assert_eq!(sub.time_commitment_in_seconds(), 31536000);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subscription_returns_none_when_not_subscribed() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("ResourceNotFoundException", "not subscribed"),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let sub = client.describe_subscription().await.unwrap();

        assert_eq!(sub, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_subscription_propagates_other_errors() {
        // `InternalErrorException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InternalErrorException", "internal failure"),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_subscription().await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InternalErrorException".to_string()));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protections_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"Protections":[{"Id":"p-1","Name":"my-protection","ResourceArn":"arn:aws:cloudfront::111111111111:distribution/E1"}]}"#,
            ),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (protections, token) = client.list_protections(None, None, None).await.unwrap();

        assert_eq!(protections.len(), 1);
        assert_eq!(protections[0].id(), Some("p-1"));
        assert_eq!(protections[0].name(), Some("my-protection"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protections_passes_through_resource_arn_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InclusionFilters":{"ResourceArns":["arn:aws:ec2:us-east-1:111111111111:eip-allocation/eipalloc-1"]}}"#,
            ),
            json_response(200, r#"{"Protections":[]}"#),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (protections, token) = client
            .list_protections(
                Some("arn:aws:ec2:us-east-1:111111111111:eip-allocation/eipalloc-1"),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(protections.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protections_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Protections":[{"Id":"p-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (protections, token) = client.list_protections(None, Some(1), None).await.unwrap();

        assert_eq!(protections.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protections_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Protections":[{"Id":"p-1"},{"Id":"p-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Protections":[{"Id":"p-3"}]}"#),
            ),
        ]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (protections, token) = client.list_protections(None, Some(10), None).await.unwrap();

        assert_eq!(protections.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protections_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InvalidPaginationTokenException", "bad token"),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let err = client.list_protections(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidPaginationTokenException".to_string()));
                assert_eq!(message, "bad token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protection_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"ProtectionGroups":[{"ProtectionGroupId":"pg-1","Aggregation":"SUM","Pattern":"ALL","Members":[]}]}"#,
            ),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client.list_protection_groups(None, None).await.unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].protection_group_id(), "pg-1");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protection_groups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"ProtectionGroups":[{"ProtectionGroupId":"pg-1","Aggregation":"SUM","Pattern":"ALL","Members":[]}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client.list_protection_groups(Some(1), None).await.unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_protection_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InternalErrorException", "internal failure"),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let err = client.list_protection_groups(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InternalErrorException".to_string()));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attacks_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"AttackSummaries":[{"AttackId":"a-1","ResourceArn":"arn:aws:cloudfront::111111111111:distribution/E1"}]}"#,
            ),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (attacks, token) = client
            .list_attacks(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(attacks.len(), 1);
        assert_eq!(attacks[0].attack_id(), Some("a-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attacks_passes_through_resource_arns_and_time_range_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"ResourceArns":["arn:aws:cloudfront::111111111111:distribution/E1"],"StartTime":{"FromInclusive":1700000000},"EndTime":{"ToExclusive":1700003600}}"#,
            ),
            json_response(200, r#"{"AttackSummaries":[]}"#),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (attacks, token) = client
            .list_attacks(
                Some(vec![
                    "arn:aws:cloudfront::111111111111:distribution/E1".to_string()
                ]),
                Some("2023-11-14T22:13:20Z".to_string()),
                Some("2023-11-14T23:13:20Z".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(attacks.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attacks_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"AttackSummaries":[{"AttackId":"a-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let (attacks, token) = client
            .list_attacks(None, None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(attacks.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attacks_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("InvalidParameterException", "bad arn"),
        )]);
        let client = ShieldClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_attacks(None, None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad arn");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
