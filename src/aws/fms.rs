use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct FmsClient {
    inner: aws_sdk_fms::Client,
}

impl FmsClient {
    pub fn new(config: &SdkConfig) -> Self {
        let fms_config = aws_sdk_fms::config::Builder::from(config)
            .region(aws_sdk_fms::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_fms::Client::from_conf(fms_config),
        }
    }

    /// Lists FMS policies, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `limit` is capped to the remaining budget on
    /// each request via `ListPoliciesInput::max_results` (verified against pinned
    /// `aws-sdk-fms` 1.106.0) so a capped page boundary always lines up with the
    /// token AWS returns.
    pub async fn list_policies(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_fms::types::PolicySummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_policies();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.policy_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists policy compliance statuses, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. Same server-side
    /// capping pattern as `list_policies` (verified `ListComplianceStatusInput`
    /// also has `max_results`).
    pub async fn list_compliance_status(
        &self,
        policy_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_fms::types::PolicyComplianceStatus>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_compliance_status().policy_id(policy_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.policy_compliance_status_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists FMS member accounts, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same server-side capping pattern
    /// as `list_policies` (verified `ListMemberAccountsInput` also has
    /// `max_results`).
    pub async fn list_member_accounts(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_member_accounts();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.member_accounts.unwrap_or_default());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://fms.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_policies_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"PolicyList":[{"PolicyId":"policy-a","PolicyName":"policy-a-name"},{"PolicyId":"policy-b","PolicyName":"policy-b-name"}]}"#,
            ),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (policies, token) = client.list_policies(None, None).await.unwrap();

        assert_eq!(policies.len(), 2);
        assert_eq!(policies[0].policy_id(), Some("policy-a"));
        assert_eq!(policies[1].policy_id(), Some("policy-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"tok-1"}"#),
            json_response(200, r#"{"PolicyList":[{"PolicyId":"policy-b"}]}"#),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (policies, token) = client
            .list_policies(None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].policy_id(), Some("policy-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"PolicyList":[{"PolicyId":"policy-a"},{"PolicyId":"policy-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (policies, token) = client.list_policies(Some(2), None).await.unwrap();

        assert_eq!(policies.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":5}"#),
                json_response(
                    200,
                    r#"{"PolicyList":[{"PolicyId":"policy-a"},{"PolicyId":"policy-b"},{"PolicyId":"policy-c"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"tok-3","MaxResults":2}"#),
                json_response(
                    200,
                    r#"{"PolicyList":[{"PolicyId":"policy-d"},{"PolicyId":"policy-e"}]}"#,
                ),
            ),
        ]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (policies, token) = client.list_policies(Some(5), None).await.unwrap();

        assert_eq!(policies.len(), 5);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("ResourceNotFoundException", "policy not found"),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_policies(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ResourceNotFoundException"));
                assert_eq!(message, "policy not found");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_compliance_status_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"PolicyId":"policy-1"}"#),
            json_response(
                200,
                r#"{"PolicyComplianceStatusList":[{"PolicyId":"policy-1","MemberAccount":"111111111111"},{"PolicyId":"policy-1","MemberAccount":"222222222222"}]}"#,
            ),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (statuses, token) = client
            .list_compliance_status("policy-1", None, None)
            .await
            .unwrap();

        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0].member_account(), Some("111111111111"));
        assert_eq!(statuses[1].member_account(), Some("222222222222"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_compliance_status_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"PolicyId":"policy-1","NextToken":"tok-1"}"#),
            json_response(
                200,
                r#"{"PolicyComplianceStatusList":[{"PolicyId":"policy-1","MemberAccount":"333333333333"}]}"#,
            ),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (statuses, token) = client
            .list_compliance_status("policy-1", None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].member_account(), Some("333333333333"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_compliance_status_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"PolicyId":"policy-1","MaxResults":2}"#),
            json_response(
                200,
                r#"{"PolicyComplianceStatusList":[{"PolicyId":"policy-1","MemberAccount":"111111111111"},{"PolicyId":"policy-1","MemberAccount":"222222222222"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (statuses, token) = client
            .list_compliance_status("policy-1", Some(2), None)
            .await
            .unwrap();

        assert_eq!(statuses.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_compliance_status_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"PolicyId":"policy-1","MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"PolicyComplianceStatusList":[{"PolicyId":"policy-1","MemberAccount":"111111111111"},{"PolicyId":"policy-1","MemberAccount":"222222222222"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"PolicyId":"policy-1","NextToken":"tok-3","MaxResults":1}"#,
                ),
                json_response(
                    200,
                    r#"{"PolicyComplianceStatusList":[{"PolicyId":"policy-1","MemberAccount":"333333333333"}]}"#,
                ),
            ),
        ]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (statuses, token) = client
            .list_compliance_status("policy-1", Some(3), None)
            .await
            .unwrap();

        assert_eq!(statuses.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_compliance_status_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"PolicyId":"policy-1"}"#),
            json_error_response("ResourceNotFoundException", "policy not found"),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_compliance_status("policy-1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ResourceNotFoundException"));
                assert_eq!(message, "policy not found");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_member_accounts_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"MemberAccounts":["111111111111","222222222222"]}"#,
            ),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (accounts, token) = client.list_member_accounts(None, None).await.unwrap();

        assert_eq!(
            accounts,
            vec!["111111111111".to_string(), "222222222222".to_string()]
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_member_accounts_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"tok-1"}"#),
            json_response(200, r#"{"MemberAccounts":["333333333333"]}"#),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (accounts, token) = client
            .list_member_accounts(None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(accounts, vec!["333333333333".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_member_accounts_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"MemberAccounts":["111111111111","222222222222"],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (accounts, token) = client.list_member_accounts(Some(2), None).await.unwrap();

        assert_eq!(accounts.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_member_accounts_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"MemberAccounts":["111111111111","222222222222"],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"tok-3","MaxResults":1}"#),
                json_response(200, r#"{"MemberAccounts":["333333333333"]}"#),
            ),
        ]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let (accounts, token) = client.list_member_accounts(Some(3), None).await.unwrap();

        assert_eq!(accounts.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_member_accounts_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("ResourceNotFoundException", "no member accounts"),
        )]);
        let client = FmsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_member_accounts(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ResourceNotFoundException"));
                assert_eq!(message, "no member accounts");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

