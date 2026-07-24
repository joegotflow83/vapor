use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct RamClient {
    inner: aws_sdk_ram::Client,
}

impl RamClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ram::Client::new(config),
        }
    }

    /// Lists resource shares, optionally capped at `limit` results (default unlimited);
    /// resumable via `next_token`.
    pub async fn list_resource_shares(
        &self,
        resource_owner: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ram::types::ResourceShare>, Option<String>), VaporError> {
        let owner = aws_sdk_ram::types::ResourceOwner::from(resource_owner.unwrap_or("SELF"));
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self
                .inner
                .get_resource_shares()
                .resource_owner(owner.clone());
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.resource_shares.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }

    /// Lists resources, optionally capped at `limit` results (default unlimited);
    /// resumable via `next_token`.
    pub async fn list_resources(
        &self,
        resource_owner: &str,
        resource_share_arns: Option<Vec<String>>,
        resource_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ram::types::Resource>, Option<String>), VaporError> {
        let owner = aws_sdk_ram::types::ResourceOwner::from(resource_owner);
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.list_resources().resource_owner(owner.clone());
            if let Some(arns) = &resource_share_arns {
                req = req.set_resource_share_arns(Some(arns.clone()));
            }
            if let Some(rt) = &resource_type {
                req = req.resource_type(rt.clone());
            }
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.resources.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }

    /// Lists principals, optionally capped at `limit` results (default unlimited);
    /// resumable via `next_token`.
    pub async fn list_principals(
        &self,
        resource_owner: &str,
        resource_share_arns: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ram::types::Principal>, Option<String>), VaporError> {
        let owner = aws_sdk_ram::types::ResourceOwner::from(resource_owner);
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.list_principals().resource_owner(owner.clone());
            if let Some(arns) = &resource_share_arns {
                req = req.set_resource_share_arns(Some(arns.clone()));
            }
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.principals.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://ram.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_resource_shares_defaults_owner_to_self() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/getresourceshares"),
                r#"{"resourceOwner":"SELF"}"#,
            ),
            json_response(
                200,
                r#"{"resourceShares":[{"resourceShareArn":"arn:aws:ram:us-east-1:111111111111:resource-share/rs-1","name":"share-1"}]}"#,
            ),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client.list_resource_shares(None, None, None).await.unwrap();

        assert_eq!(shares.len(), 1);
        assert_eq!(
            shares[0].resource_share_arn(),
            Some("arn:aws:ram:us-east-1:111111111111:resource-share/rs-1")
        );
        assert_eq!(shares[0].name(), Some("share-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resource_shares_passes_through_custom_owner() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/getresourceshares"),
                r#"{"resourceOwner":"OTHER-ACCOUNTS"}"#,
            ),
            json_response(200, r#"{"resourceShares":[]}"#),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_resource_shares(Some("OTHER-ACCOUNTS"), None, None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resource_shares_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/getresourceshares"),
                r#"{"resourceOwner":"SELF","nextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"resourceShares":[]}"#),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_resource_shares(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(shares.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resource_shares_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/getresourceshares"),
                r#"{"resourceOwner":"SELF","maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"resourceShares":[{"resourceShareArn":"rs-1"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_resource_shares(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resource_shares_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/getresourceshares"),
                    r#"{"resourceOwner":"SELF","maxResults":10}"#,
                ),
                json_response(
                    200,
                    r#"{"resourceShares":[{"resourceShareArn":"rs-1"},{"resourceShareArn":"rs-2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/getresourceshares"),
                    r#"{"resourceOwner":"SELF","nextToken":"p2","maxResults":8}"#,
                ),
                json_response(200, r#"{"resourceShares":[{"resourceShareArn":"rs-3"}]}"#),
            ),
        ]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_resource_shares(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resource_shares_propagates_errors() {
        // `InvalidParameterException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/getresourceshares"),
                r#"{"resourceOwner":"SELF"}"#,
            ),
            json_error_response("InvalidParameterException", "bad owner"),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_resource_shares(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad owner");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resources_passes_through_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listresources"),
                r#"{"resourceOwner":"SELF","resourceShareArns":["rs-1"],"resourceType":"ec2:subnet"}"#,
            ),
            json_response(
                200,
                r#"{"resources":[{"arn":"arn:aws:ec2:us-east-1:111111111111:subnet/subnet-1","type":"ec2:subnet"}]}"#,
            ),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client
            .list_resources(
                "SELF",
                Some(vec!["rs-1".to_string()]),
                Some("ec2:subnet".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(
            resources[0].arn(),
            Some("arn:aws:ec2:us-east-1:111111111111:subnet/subnet-1")
        );
        assert_eq!(resources[0].r#type(), Some("ec2:subnet"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resources_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listresources"),
                r#"{"resourceOwner":"SELF","maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"resources":[{"arn":"arn-1"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (resources, token) = client
            .list_resources("SELF", None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_resources_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listresources"),
                r#"{"resourceOwner":"SELF"}"#,
            ),
            json_error_response("MalformedArnException", "bad arn"),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_resources("SELF", None, None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("MalformedArnException".to_string()));
                assert_eq!(message, "bad arn");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_principals_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listprincipals"),
                r#"{"resourceOwner":"SELF"}"#,
            ),
            json_response(
                200,
                r#"{"principals":[{"id":"111111111111","resourceShareArn":"rs-1","external":false}]}"#,
            ),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (principals, token) = client
            .list_principals("SELF", None, None, None)
            .await
            .unwrap();

        assert_eq!(principals.len(), 1);
        assert_eq!(principals[0].id(), Some("111111111111"));
        assert_eq!(principals[0].resource_share_arn(), Some("rs-1"));
        assert_eq!(principals[0].external(), Some(false));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_principals_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listprincipals"),
                r#"{"resourceOwner":"SELF","maxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"principals":[{"id":"111111111111"}],"nextToken":"page2-token"}"#,
            ),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let (principals, token) = client
            .list_principals("SELF", None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(principals.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_principals_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/listprincipals"),
                r#"{"resourceOwner":"SELF"}"#,
            ),
            json_error_response("UnknownResourceException", "no such resource"),
        )]);
        let client = RamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_principals("SELF", None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("UnknownResourceException".to_string()));
                assert_eq!(message, "no such resource");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
