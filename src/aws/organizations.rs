use aws_config::SdkConfig;
use aws_sdk_organizations::types::PolicyType;

use crate::error::VaporError;

pub struct OrganizationsClient {
    inner: aws_sdk_organizations::Client,
}

impl OrganizationsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_organizations::Client::new(config),
        }
    }

    /// Lists accounts in the organization, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListAccounts` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-organizations` 1.118.0's
    /// `operation/list_accounts/_list_accounts_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `backup.rs`'s `list_backup_vaults` pattern.
    pub async fn list_accounts(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_organizations::types::Account>, Option<String>), VaporError> {
        let mut accounts = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_accounts();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - accounts.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            accounts.extend(output.accounts.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if accounts.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((accounts, token))
    }

    /// Lists organizational units under a parent, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `ListOrganizationalUnitsForParent` has both `max_results` and
    /// `next_token` (verified against pinned `aws-sdk-organizations`
    /// 1.118.0's
    /// `operation/list_organizational_units_for_parent/
    /// _list_organizational_units_for_parent_input.rs`), same pattern.
    pub async fn list_organizational_units_for_parent(
        &self,
        parent_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_organizations::types::OrganizationalUnit>, Option<String>), VaporError>
    {
        let mut ous = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_organizational_units_for_parent()
                .parent_id(parent_id);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - ous.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            ous.extend(output.organizational_units.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if ous.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((ous, token))
    }

    /// Lists policies of the given type, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListPolicies` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-organizations` 1.118.0's
    /// `operation/list_policies/_list_policies_input.rs`), same pattern.
    pub async fn list_policies(
        &self,
        policy_type: PolicyType,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_organizations::types::PolicySummary>, Option<String>), VaporError>
    {
        let mut policies = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_policies().filter(policy_type.clone());
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - policies.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            policies.extend(output.policies.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if policies.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((policies, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::error::VaporError;

    const ENDPOINT: &str = "https://organizations.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_accounts_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Accounts":[{"Id":"111111111111","Name":"account-a","Email":"a@example.com"},{"Id":"222222222222","Name":"account-b"}]}"#,
            ),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let result = client.list_accounts(None, None).await;

        match result {
            Ok((accounts, token)) => {
                assert_eq!(accounts.len(), 2);
                assert_eq!(accounts[0].id(), Some("111111111111"));
                assert_eq!(accounts[0].name(), Some("account-a"));
                assert_eq!(accounts[1].id(), Some("222222222222"));
                assert_eq!(token, None);
            }
            Err(e) => panic!("expected Ok, got {e:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accounts_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Accounts":[{"Id":"333333333333"}]}"#),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (accounts, token) = client
            .list_accounts(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id(), Some("333333333333"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accounts_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Accounts":[{"Id":"a"},{"Id":"b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (accounts, token) = client.list_accounts(Some(2), None).await.unwrap();

        assert_eq!(accounts.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accounts_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Accounts":[{"Id":"a"},{"Id":"b"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Accounts":[{"Id":"c"}]}"#),
            ),
        ]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (accounts, token) = client.list_accounts(Some(10), None).await.unwrap();

        assert_eq!(accounts.len(), 3);
        assert_eq!(accounts[2].id(), Some("c"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_accounts_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "bad input"),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_accounts(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidInputException"));
                assert_eq!(message, "bad input");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_organizational_units_for_parent_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ParentId":"r-root1"}"#),
            json_response(
                200,
                r#"{"OrganizationalUnits":[{"Id":"ou-1","Name":"ou-a","Arn":"arn:aws:organizations::1:ou/o-1/ou-1"},{"Id":"ou-2","Name":"ou-b"}]}"#,
            ),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (ous, token) = client
            .list_organizational_units_for_parent("r-root1", None, None)
            .await
            .unwrap();

        assert_eq!(ous.len(), 2);
        assert_eq!(ous[0].id(), Some("ou-1"));
        assert_eq!(ous[0].name(), Some("ou-a"));
        assert_eq!(ous[1].id(), Some("ou-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_organizational_units_for_parent_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ParentId":"r-root1","NextToken":"cursor-a"}"#),
            json_response(200, r#"{"OrganizationalUnits":[{"Id":"ou-3"}]}"#),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (ous, token) = client
            .list_organizational_units_for_parent(
                "r-root1",
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(ous.len(), 1);
        assert_eq!(ous[0].id(), Some("ou-3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_organizational_units_for_parent_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ParentId":"r-root1","MaxResults":2}"#),
            json_response(
                200,
                r#"{"OrganizationalUnits":[{"Id":"a"},{"Id":"b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (ous, token) = client
            .list_organizational_units_for_parent("r-root1", Some(2), None)
            .await
            .unwrap();

        assert_eq!(ous.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_organizational_units_for_parent_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ParentId":"r-root1"}"#),
            json_error_response("ServiceException", "internal error"),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_organizational_units_for_parent("r-root1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("ServiceException"));
                assert_eq!(message, "internal error");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Filter":"SERVICE_CONTROL_POLICY"}"#),
            json_response(
                200,
                r#"{"Policies":[{"Id":"p-1","Name":"policy-a","Type":"SERVICE_CONTROL_POLICY","AwsManaged":false},{"Id":"p-2","Name":"policy-b","AwsManaged":true}]}"#,
            ),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (policies, token) = client
            .list_policies(PolicyType::ServiceControlPolicy, None, None)
            .await
            .unwrap();

        assert_eq!(policies.len(), 2);
        assert_eq!(policies[0].id(), Some("p-1"));
        assert_eq!(
            policies[0].r#type(),
            Some(&PolicyType::ServiceControlPolicy)
        );
        assert!(!policies[0].aws_managed());
        assert!(policies[1].aws_managed());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"Filter":"SERVICE_CONTROL_POLICY","NextToken":"cursor-a"}"#,
            ),
            json_response(200, r#"{"Policies":[{"Id":"p-3"}]}"#),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (policies, token) = client
            .list_policies(
                PolicyType::ServiceControlPolicy,
                None,
                Some("cursor-a".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].id(), Some("p-3"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"Filter":"SERVICE_CONTROL_POLICY","MaxResults":2}"#,
            ),
            json_response(
                200,
                r#"{"Policies":[{"Id":"a"},{"Id":"b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let (policies, token) = client
            .list_policies(PolicyType::ServiceControlPolicy, Some(2), None)
            .await
            .unwrap();

        assert_eq!(policies.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Filter":"SERVICE_CONTROL_POLICY"}"#),
            json_error_response("InvalidInputException", "bad filter"),
        )]);
        let client = OrganizationsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_policies(PolicyType::ServiceControlPolicy, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidInputException"));
                assert_eq!(message, "bad filter");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

