use aws_config::SdkConfig;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use futures::future::join_all;

use crate::error::VaporError;

pub struct SsoAdminClient {
    inner: aws_sdk_ssoadmin::Client,
}

impl SsoAdminClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ssoadmin::Client::new(config),
        }
    }

    /// Lists IAM Identity Center instances, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListInstances` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-ssoadmin` 1.105.0's
    /// `operation/list_instances/_list_instances_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `ram.rs`'s `list_resource_shares` pattern.
    pub async fn list_instances(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ssoadmin::types::InstanceMetadata>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.list_instances();
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.instances.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }

    /// Lists permission sets for an instance, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListPermissionSets` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-ssoadmin` 1.105.0's
    /// `operation/list_permission_sets/_list_permission_sets_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself. The
    /// N+1 `describe_permission_set` fan-out only covers the single page of
    /// ARNs collected this call, not the whole collection — matching
    /// `mq.rs`'s `list_brokers` pattern. Fan-out errors (other than
    /// not-found, which `describe_permission_set` already maps to `None`)
    /// propagate via `?`, matching this file's pre-existing hard-error
    /// convention for list+describe fan-outs.
    pub async fn list_permission_sets(
        &self,
        instance_arn: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ssoadmin::types::PermissionSet>, Option<String>), VaporError> {
        let mut arns = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - arns.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self.inner.list_permission_sets().instance_arn(instance_arn);
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            arns.extend(output.permission_sets.unwrap_or_default());
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| arns.len() as i32 >= l) {
                break;
            }
        }

        let futures = arns
            .iter()
            .map(|arn| self.describe_permission_set(instance_arn, arn));
        let results = join_all(futures).await;

        let mut permission_sets = Vec::with_capacity(arns.len());
        for result in results {
            if let Some(ps) = result? {
                permission_sets.push(ps);
            }
        }

        Ok((permission_sets, token))
    }

    pub async fn describe_permission_set(
        &self,
        instance_arn: &str,
        permission_set_arn: &str,
    ) -> Result<Option<aws_sdk_ssoadmin::types::PermissionSet>, VaporError> {
        let output = match self
            .inner
            .describe_permission_set()
            .instance_arn(instance_arn)
            .permission_set_arn(permission_set_arn)
            .send()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                if matches!(e.code(), Some("ResourceNotFoundException")) {
                    return Ok(None);
                }
                return Err(crate::error::sdk_err(e));
            }
        };
        Ok(output.permission_set().cloned())
    }

    /// Lists account assignments, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListAccountAssignments` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-ssoadmin` 1.105.0's
    /// `operation/list_account_assignments/_list_account_assignments_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself.
    pub async fn list_account_assignments(
        &self,
        instance_arn: &str,
        account_id: &str,
        permission_set_arn: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_ssoadmin::types::AccountAssignment>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = self
                .inner
                .list_account_assignments()
                .instance_arn(instance_arn)
                .account_id(account_id)
                .permission_set_arn(permission_set_arn);
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.account_assignments.unwrap_or_default());
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
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const BASE: &str = "https://sso.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_instances_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"Instances":[{"InstanceArn":"arn:aws:sso:::instance/ssoins-1","IdentityStoreId":"d-1","OwnerAccountId":"111111111111","Name":"main","CreatedDate":1700000000,"Status":"ACTIVE"}]}"#,
            ),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (instances, token) = client.list_instances(None, None).await.unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(
            instances[0].instance_arn(),
            Some("arn:aws:sso:::instance/ssoins-1")
        );
        assert_eq!(instances[0].identity_store_id(), Some("d-1"));
        assert_eq!(instances[0].owner_account_id(), Some("111111111111"));
        assert_eq!(instances[0].name(), Some("main"));
        assert!(instances[0].created_date().is_some());
        assert_eq!(instances[0].status().map(|s| s.as_str()), Some("ACTIVE"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Instances":[]}"#),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (instances, token) = client
            .list_instances(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(instances.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Instances":[{"InstanceArn":"arn-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (instances, token) = client.list_instances(Some(1), None).await.unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Instances":[{"InstanceArn":"arn-1"},{"InstanceArn":"arn-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Instances":[{"InstanceArn":"arn-3"}]}"#),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (instances, token) = client.list_instances(Some(10), None).await.unwrap();

        assert_eq!(instances.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_instances_propagates_errors() {
        // `ValidationException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_error_response("ValidationException", "bad request"),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let err = client.list_instances(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permission_sets_lists_all_with_describe_fan_out() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"InstanceArn":"arn:instance"}"#),
                json_response(
                    200,
                    r#"{"PermissionSets":["ps-1","ps-2"]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-1"}"#,
                ),
                json_response(
                    200,
                    r#"{"PermissionSet":{"Name":"ps-one","PermissionSetArn":"ps-1","Description":"first","SessionDuration":"PT1H"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-2"}"#,
                ),
                json_response(200, r#"{"PermissionSet":{"Name":"ps-two"}}"#),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (permission_sets, token) = client
            .list_permission_sets("arn:instance", None, None)
            .await
            .unwrap();

        assert_eq!(permission_sets.len(), 2);
        assert_eq!(permission_sets[0].name(), Some("ps-one"));
        assert_eq!(permission_sets[0].permission_set_arn(), Some("ps-1"));
        assert_eq!(permission_sets[0].description(), Some("first"));
        assert_eq!(permission_sets[0].session_duration(), Some("PT1H"));
        assert_eq!(permission_sets[1].name(), Some("ps-two"));
        assert_eq!(permission_sets[1].permission_set_arn(), None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permission_sets_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","NextToken":"cursor-a"}"#,
                ),
                json_response(200, r#"{"PermissionSets":["ps-3"]}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-3"}"#,
                ),
                json_response(200, r#"{"PermissionSet":{"Name":"ps-three"}}"#),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (permission_sets, token) = client
            .list_permission_sets("arn:instance", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(permission_sets.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permission_sets_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"InstanceArn":"arn:instance","MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"PermissionSets":["ps-1"],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-1"}"#,
                ),
                json_response(200, r#"{"PermissionSet":{"Name":"ps-one"}}"#),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (permission_sets, token) = client
            .list_permission_sets("arn:instance", Some(1), None)
            .await
            .unwrap();

        assert_eq!(permission_sets.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permission_sets_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"InstanceArn":"arn:instance","MaxResults":10}"#),
                json_response(200, r#"{"PermissionSets":["ps-1","ps-2"],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(200, r#"{"PermissionSets":["ps-3"]}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-1"}"#,
                ),
                json_response(200, r#"{"PermissionSet":{"Name":"ps-one"}}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-2"}"#,
                ),
                json_response(200, r#"{"PermissionSet":{"Name":"ps-two"}}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-3"}"#,
                ),
                json_response(200, r#"{"PermissionSet":{"Name":"ps-three"}}"#),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (permission_sets, token) = client
            .list_permission_sets("arn:instance", Some(10), None)
            .await
            .unwrap();

        assert_eq!(permission_sets.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permission_sets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"InstanceArn":"arn:instance"}"#),
            json_error_response("ValidationException", "bad instance"),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_permission_sets("arn:instance", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad instance");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permission_sets_propagates_describe_permission_set_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"InstanceArn":"arn:instance"}"#),
                json_response(200, r#"{"PermissionSets":["ps-1"]}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-1"}"#,
                ),
                json_error_response("AccessDeniedException", "not allowed"),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_permission_sets("arn:instance", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not allowed");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_permission_sets_skips_permission_set_when_describe_permission_set_reports_not_found() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"InstanceArn":"arn:instance"}"#),
                json_response(200, r#"{"PermissionSets":["ps-1"]}"#),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-1"}"#,
                ),
                json_error_response("ResourceNotFoundException", "permission set not found"),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (permission_sets, token) = client
            .list_permission_sets("arn:instance", None, None)
            .await
            .unwrap();

        assert!(permission_sets.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_permission_set_returns_none_when_resource_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InstanceArn":"arn:instance","PermissionSetArn":"missing"}"#,
            ),
            json_error_response("ResourceNotFoundException", "permission set not found"),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let result = client
            .describe_permission_set("arn:instance", "missing")
            .await
            .unwrap();

        assert_eq!(result, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_permission_set_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InstanceArn":"arn:instance","PermissionSetArn":"ps-1"}"#,
            ),
            json_error_response("AccessDeniedException", "not allowed"),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_permission_set("arn:instance", "ps-1")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not allowed");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_account_assignments_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InstanceArn":"arn:instance","AccountId":"111111111111","PermissionSetArn":"ps-1"}"#,
            ),
            json_response(
                200,
                r#"{"AccountAssignments":[{"AccountId":"111111111111","PermissionSetArn":"ps-1","PrincipalType":"USER","PrincipalId":"principal-1"}]}"#,
            ),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (assignments, token) = client
            .list_account_assignments("arn:instance", "111111111111", "ps-1", None, None)
            .await
            .unwrap();

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].account_id(), Some("111111111111"));
        assert_eq!(assignments[0].permission_set_arn(), Some("ps-1"));
        assert_eq!(
            assignments[0].principal_type().map(|p| p.as_str()),
            Some("USER")
        );
        assert_eq!(assignments[0].principal_id(), Some("principal-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_account_assignments_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InstanceArn":"arn:instance","AccountId":"111111111111","PermissionSetArn":"ps-1","MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"AccountAssignments":[{"AccountId":"111111111111"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (assignments, token) = client
            .list_account_assignments("arn:instance", "111111111111", "ps-1", Some(1), None)
            .await
            .unwrap();

        assert_eq!(assignments.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_account_assignments_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","AccountId":"111111111111","PermissionSetArn":"ps-1","MaxResults":10}"#,
                ),
                json_response(
                    200,
                    r#"{"AccountAssignments":[{"AccountId":"111111111111"},{"AccountId":"111111111111"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"InstanceArn":"arn:instance","AccountId":"111111111111","PermissionSetArn":"ps-1","NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(200, r#"{"AccountAssignments":[{"AccountId":"111111111111"}]}"#),
            ),
        ]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let (assignments, token) = client
            .list_account_assignments("arn:instance", "111111111111", "ps-1", Some(10), None)
            .await
            .unwrap();

        assert_eq!(assignments.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_account_assignments_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InstanceArn":"arn:instance","AccountId":"111111111111","PermissionSetArn":"ps-1"}"#,
            ),
            json_error_response("ValidationException", "bad account"),
        )]);
        let client = SsoAdminClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_account_assignments("arn:instance", "111111111111", "ps-1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad account");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
