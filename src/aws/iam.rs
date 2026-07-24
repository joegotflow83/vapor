#[cfg(feature = "iam")]
use aws_config::SdkConfig;
#[cfg(feature = "iam")]
use aws_sdk_iam::types::{
    AccessKeyLastUsed, AccessKeyMetadata, AttachedPolicy, Group, MfaDevice, PasswordPolicy, Policy,
    PolicyVersion, Role, User,
};

#[cfg(feature = "iam")]
use crate::error::VaporError;

#[cfg(feature = "iam")]
pub struct IamClient {
    inner: aws_sdk_iam::Client,
}

impl IamClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_iam::Client::new(config),
        }
    }

    /// Lists IAM roles, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListRolesInput::max_items`
    /// (verified against pinned `aws-sdk-iam` 1.113.0) caps a request page
    /// directly; the continuation field is `marker`, gated on `is_truncated`.
    pub async fn list_roles(
        &self,
        path_prefix: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Role>, Option<String>), VaporError> {
        let mut roles = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_roles();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            if let Some(l) = limit {
                req = req.max_items(l - roles.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            roles.extend(output.roles().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if roles.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((roles, token))
    }

    /// Lists managed IAM policies, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    pub async fn list_policies(
        &self,
        scope: &str,
        path_prefix: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Policy>, Option<String>), VaporError> {
        use aws_sdk_iam::types::PolicyScopeType;

        let scope_val = match scope {
            "AWS" => PolicyScopeType::Aws,
            "All" => PolicyScopeType::All,
            _ => PolicyScopeType::Local,
        };

        let mut policies = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_policies().scope(scope_val.clone());
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            if let Some(l) = limit {
                req = req.max_items(l - policies.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            policies.extend(output.policies().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if policies.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((policies, token))
    }

    /// Lists IAM users, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    pub async fn list_users(
        &self,
        path_prefix: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<User>, Option<String>), VaporError> {
        let mut users = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_users();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            if let Some(l) = limit {
                req = req.max_items(l - users.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            users.extend(output.users().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if users.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((users, token))
    }

    /// Lists IAM groups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    pub async fn list_groups(
        &self,
        path_prefix: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Group>, Option<String>), VaporError> {
        let mut groups = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_groups();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            if let Some(l) = limit {
                req = req.max_items(l - groups.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            groups.extend(output.groups().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if groups.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((groups, token))
    }

    /// Lists policies attached to an IAM role, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    pub async fn list_attached_role_policies(
        &self,
        role_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AttachedPolicy>, Option<String>), VaporError> {
        let mut policies = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_attached_role_policies()
                .role_name(role_name);
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - policies.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            policies.extend(output.attached_policies().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if policies.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((policies, token))
    }

    /// Fetch a specific version of a managed IAM policy document.
    /// If `version_id` is None, the policy's current default version is fetched automatically.
    pub async fn get_managed_policy_document(
        &self,
        policy_arn: &str,
        version_id: Option<&str>,
    ) -> Result<PolicyVersion, VaporError> {
        let vid = match version_id {
            Some(v) => v.to_string(),
            None => {
                let output = self
                    .inner
                    .get_policy()
                    .policy_arn(policy_arn)
                    .send()
                    .await
                    .map_err(crate::error::sdk_err)?;
                output
                    .policy()
                    .and_then(|p| p.default_version_id())
                    .unwrap_or("v1")
                    .to_string()
            }
        };

        let output = self
            .inner
            .get_policy_version()
            .policy_arn(policy_arn)
            .version_id(&vid)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        output
            .policy_version()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk {
                code: None,
                message: format!("No policy version found for {policy_arn}"),
            })
    }

    /// List access keys for an IAM user, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    pub async fn list_access_keys(
        &self,
        user_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AccessKeyMetadata>, Option<String>), VaporError> {
        let mut keys = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_access_keys().user_name(user_name);
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - keys.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            keys.extend(output.access_key_metadata().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if keys.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((keys, token))
    }

    /// Fetch the last-used metadata for an access key. Returns None if the key
    /// has never been used.
    pub async fn get_access_key_last_used(
        &self,
        access_key_id: &str,
    ) -> Result<Option<AccessKeyLastUsed>, VaporError> {
        let output = self
            .inner
            .get_access_key_last_used()
            .access_key_id(access_key_id)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.access_key_last_used().cloned())
    }

    /// Fetch the account-wide IAM password policy.
    /// Returns `None` if no custom password policy has been configured
    /// (AWS then applies its own minimum defaults).
    pub async fn get_account_password_policy(&self) -> Result<Option<PasswordPolicy>, VaporError> {
        match self.inner.get_account_password_policy().send().await {
            Ok(output) => Ok(output.password_policy().cloned()),
            Err(e) => {
                if e.as_service_error()
                    .map(|se| se.is_no_such_entity_exception())
                    .unwrap_or(false)
                {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }

    /// List MFA devices enrolled for an IAM user. Returns an empty vec if the
    /// user has no devices configured — enables detection of users lacking MFA
    /// (CIS AWS Benchmark 1.10). Optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    pub async fn list_mfa_devices(
        &self,
        user_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<MfaDevice>, Option<String>), VaporError> {
        let mut devices = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_mfa_devices().user_name(user_name);
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - devices.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            devices.extend(output.mfa_devices().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if devices.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((devices, token))
    }

    /// List all inline policies embedded directly in an IAM role, returning
    /// a list of (policy_name, url_encoded_document) pairs. `limit`/
    /// `next_token` paginate the policy-name discovery list (`ListRolePolicies`)
    /// before the `GetRolePolicy` fan-out over the discovered page.
    pub async fn get_role_inline_policies(
        &self,
        role_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<(String, String)>, Option<String>), VaporError> {
        let mut names = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_role_policies().role_name(role_name);
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - names.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            names.extend(output.policy_names().iter().cloned());
            token = if output.is_truncated() {
                output.marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut results = Vec::new();
        for name in &names {
            let output = self
                .inner
                .get_role_policy()
                .role_name(role_name)
                .policy_name(name)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            results.push((name.clone(), output.policy_document().to_string()));
        }

        Ok((results, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient,
    };

    // IAM is a global service: a single endpoint regardless of configured
    // region (verified against pinned `aws-sdk-iam` 1.113.0's
    // `config/endpoint.rs`, which hardcodes `https://iam.amazonaws.com`).
    const ENDPOINT: &str = "https://iam.amazonaws.com/";
    const POLICY_ARN: &str = "arn:aws:iam::123456789012:policy/test-policy";
    const POLICY_ARN_ENC: &str = "arn%3Aaws%3Aiam%3A%3A123456789012%3Apolicy%2Ftest-policy";

    #[tokio::test]
    async fn list_roles_happy_path_with_path_prefix() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListRoles&Version=2010-05-08&PathPrefix=%2Fapp%2F",
            ),
            xml_response(
                200,
                "<ListRolesResponse><ListRolesResult><Roles><member>\
                 <RoleName>my-role</RoleName><RoleId>AROAEXAMPLE</RoleId>\
                 <Arn>arn:aws:iam::123456789012:role/my-role</Arn><Path>/app/</Path>\
                 <CreateDate>2024-01-01T00:00:00Z</CreateDate></member></Roles>\
                 <IsTruncated>false</IsTruncated></ListRolesResult></ListRolesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (roles, marker) = client.list_roles(Some("/app/"), None, None).await.unwrap();

        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].role_name(), "my-role");
        assert_eq!(roles[0].arn(), "arn:aws:iam::123456789012:role/my-role");
        assert_eq!(roles[0].path(), "/app/");
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_roles_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListRoles&Version=2010-05-08&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<ListRolesResponse><ListRolesResult><Roles></Roles>\
                 <IsTruncated>false</IsTruncated></ListRolesResult></ListRolesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (roles, marker) = client
            .list_roles(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(roles.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_roles_capped_at_limit_aws_side_enforced() {
        // `list_roles` forwards `MaxItems` straight to AWS with no
        // client-side truncate, so the canned response must return exactly
        // `limit` items (gotcha 13's AWS-side-enforcement category).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListRoles&Version=2010-05-08&MaxItems=2"),
            xml_response(
                200,
                "<ListRolesResponse><ListRolesResult><Roles>\
                 <member><RoleName>a</RoleName></member>\
                 <member><RoleName>b</RoleName></member>\
                 </Roles><IsTruncated>true</IsTruncated><Marker>page2</Marker>\
                 </ListRolesResult></ListRolesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (roles, marker) = client.list_roles(None, Some(2), None).await.unwrap();

        assert_eq!(roles.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_roles_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListRoles&Version=2010-05-08&MaxItems=100"),
                xml_response(
                    200,
                    "<ListRolesResponse><ListRolesResult><Roles>\
                     <member><RoleName>a</RoleName></member></Roles>\
                     <IsTruncated>true</IsTruncated><Marker>p2</Marker>\
                     </ListRolesResult></ListRolesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=ListRoles&Version=2010-05-08&Marker=p2&MaxItems=99",
                ),
                xml_response(
                    200,
                    "<ListRolesResponse><ListRolesResult><Roles>\
                     <member><RoleName>b</RoleName></member></Roles>\
                     <IsTruncated>false</IsTruncated></ListRolesResult></ListRolesResponse>",
                ),
            ),
        ]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (roles, marker) = client.list_roles(None, Some(100), None).await.unwrap();

        assert_eq!(roles.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_roles_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListRoles&Version=2010-05-08"),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client.list_roles(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ServiceFailure".to_string()));
                assert_eq!(message, "internal error");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_happy_path_scope_aws() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListPolicies&Version=2010-05-08&Scope=AWS"),
            xml_response(
                200,
                "<ListPoliciesResponse><ListPoliciesResult><Policies><member>\
                 <PolicyName>AdministratorAccess</PolicyName><PolicyId>ANPAEXAMPLE</PolicyId>\
                 <Arn>arn:aws:iam::aws:policy/AdministratorAccess</Arn><Path>/</Path>\
                 <DefaultVersionId>v1</DefaultVersionId></member></Policies>\
                 <IsTruncated>false</IsTruncated></ListPoliciesResult></ListPoliciesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (policies, marker) = client.list_policies("AWS", None, None, None).await.unwrap();

        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].policy_name(), Some("AdministratorAccess"));
        assert_eq!(policies[0].default_version_id(), Some("v1"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_unrecognized_scope_defaults_to_local() {
        // `scope` maps any value other than the literal "AWS"/"All" strings
        // to `PolicyScopeType::Local` (see the wrapper's `match scope`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListPolicies&Version=2010-05-08&Scope=Local",
            ),
            xml_response(
                200,
                "<ListPoliciesResponse><ListPoliciesResult><Policies></Policies>\
                 <IsTruncated>false</IsTruncated></ListPoliciesResult></ListPoliciesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (policies, _marker) = client
            .list_policies("Local", None, None, None)
            .await
            .unwrap();

        assert_eq!(policies.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_capped_at_limit_aws_side_enforced() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListPolicies&Version=2010-05-08&Scope=All&MaxItems=1",
            ),
            xml_response(
                200,
                "<ListPoliciesResponse><ListPoliciesResult><Policies><member>\
                 <PolicyName>a</PolicyName></member></Policies>\
                 <IsTruncated>true</IsTruncated><Marker>p2</Marker>\
                 </ListPoliciesResult></ListPoliciesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (policies, marker) = client
            .list_policies("All", None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(policies.len(), 1);
        assert_eq!(marker, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_policies_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListPolicies&Version=2010-05-08&Scope=AWS"),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_policies("AWS", None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListUsers&Version=2010-05-08"),
            xml_response(
                200,
                "<ListUsersResponse><ListUsersResult><Users><member>\
                 <UserName>alice</UserName><UserId>AIDAEXAMPLE</UserId>\
                 <Arn>arn:aws:iam::123456789012:user/alice</Arn><Path>/</Path>\
                 <CreateDate>2024-01-01T00:00:00Z</CreateDate></member></Users>\
                 <IsTruncated>false</IsTruncated></ListUsersResult></ListUsersResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (users, marker) = client.list_users(None, None, None).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].user_name(), "alice");
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListUsers&Version=2010-05-08&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<ListUsersResponse><ListUsersResult><Users></Users>\
                 <IsTruncated>false</IsTruncated></ListUsersResult></ListUsersResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (users, marker) = client
            .list_users(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(users.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_capped_at_limit_aws_side_enforced() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListUsers&Version=2010-05-08&MaxItems=2"),
            xml_response(
                200,
                "<ListUsersResponse><ListUsersResult><Users>\
                 <member><UserName>a</UserName></member>\
                 <member><UserName>b</UserName></member></Users>\
                 <IsTruncated>true</IsTruncated><Marker>page2</Marker>\
                 </ListUsersResult></ListUsersResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (users, marker) = client.list_users(None, Some(2), None).await.unwrap();

        assert_eq!(users.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_users_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListUsers&Version=2010-05-08"),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client.list_users(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_groups_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListGroups&Version=2010-05-08"),
            xml_response(
                200,
                "<ListGroupsResponse><ListGroupsResult><Groups><member>\
                 <GroupName>admins</GroupName><GroupId>AGPAEXAMPLE</GroupId>\
                 <Arn>arn:aws:iam::123456789012:group/admins</Arn><Path>/</Path>\
                 <CreateDate>2024-01-01T00:00:00Z</CreateDate></member></Groups>\
                 <IsTruncated>false</IsTruncated></ListGroupsResult></ListGroupsResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.list_groups(None, None, None).await.unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].group_name(), "admins");
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_groups_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListGroups&Version=2010-05-08&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<ListGroupsResponse><ListGroupsResult><Groups></Groups>\
                 <IsTruncated>false</IsTruncated></ListGroupsResult></ListGroupsResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client
            .list_groups(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(groups.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_groups_capped_at_limit_aws_side_enforced() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListGroups&Version=2010-05-08&MaxItems=1"),
            xml_response(
                200,
                "<ListGroupsResponse><ListGroupsResult><Groups>\
                 <member><GroupName>a</GroupName></member></Groups>\
                 <IsTruncated>true</IsTruncated><Marker>page2</Marker>\
                 </ListGroupsResult></ListGroupsResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.list_groups(None, Some(1), None).await.unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListGroups&Version=2010-05-08"),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client.list_groups(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attached_role_policies_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAttachedRolePolicies&Version=2010-05-08&RoleName=my-role",
            ),
            xml_response(
                200,
                "<ListAttachedRolePoliciesResponse><ListAttachedRolePoliciesResult>\
                 <AttachedPolicies><member><PolicyName>AdministratorAccess</PolicyName>\
                 <PolicyArn>arn:aws:iam::aws:policy/AdministratorAccess</PolicyArn></member>\
                 </AttachedPolicies><IsTruncated>false</IsTruncated>\
                 </ListAttachedRolePoliciesResult></ListAttachedRolePoliciesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (policies, marker) = client
            .list_attached_role_policies("my-role", None, None)
            .await
            .unwrap();

        assert_eq!(policies.len(), 1);
        assert_eq!(
            policies[0].policy_arn(),
            Some("arn:aws:iam::aws:policy/AdministratorAccess")
        );
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attached_role_policies_capped_at_limit_aws_side_enforced() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAttachedRolePolicies&Version=2010-05-08&RoleName=my-role&MaxItems=1",
            ),
            xml_response(
                200,
                "<ListAttachedRolePoliciesResponse><ListAttachedRolePoliciesResult>\
                 <AttachedPolicies><member><PolicyName>a</PolicyName></member></AttachedPolicies>\
                 <IsTruncated>true</IsTruncated><Marker>p2</Marker>\
                 </ListAttachedRolePoliciesResult></ListAttachedRolePoliciesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (policies, marker) = client
            .list_attached_role_policies("my-role", Some(1), None)
            .await
            .unwrap();

        assert_eq!(policies.len(), 1);
        assert_eq!(marker, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attached_role_policies_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=ListAttachedRolePolicies&Version=2010-05-08&RoleName=my-role&MaxItems=100",
                ),
                xml_response(
                    200,
                    "<ListAttachedRolePoliciesResponse><ListAttachedRolePoliciesResult>\
                     <AttachedPolicies><member><PolicyName>a</PolicyName></member></AttachedPolicies>\
                     <IsTruncated>true</IsTruncated><Marker>p2</Marker>\
                     </ListAttachedRolePoliciesResult></ListAttachedRolePoliciesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=ListAttachedRolePolicies&Version=2010-05-08&RoleName=my-role&Marker=p2&MaxItems=99",
                ),
                xml_response(
                    200,
                    "<ListAttachedRolePoliciesResponse><ListAttachedRolePoliciesResult>\
                     <AttachedPolicies><member><PolicyName>b</PolicyName></member></AttachedPolicies>\
                     <IsTruncated>false</IsTruncated>\
                     </ListAttachedRolePoliciesResult></ListAttachedRolePoliciesResponse>",
                ),
            ),
        ]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (policies, marker) = client
            .list_attached_role_policies("my-role", Some(100), None)
            .await
            .unwrap();

        assert_eq!(policies.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_attached_role_policies_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAttachedRolePolicies&Version=2010-05-08&RoleName=my-role",
            ),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_attached_role_policies("my-role", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_managed_policy_document_with_explicit_version_skips_get_policy() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetPolicyVersion&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}&VersionId=v2"),
            ),
            xml_response(
                200,
                "<GetPolicyVersionResponse><GetPolicyVersionResult><PolicyVersion>\
                 <Document>doc-body</Document><VersionId>v2</VersionId>\
                 <IsDefaultVersion>false</IsDefaultVersion></PolicyVersion>\
                 </GetPolicyVersionResult></GetPolicyVersionResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let version = client
            .get_managed_policy_document(POLICY_ARN, Some("v2"))
            .await
            .unwrap();

        assert_eq!(version.version_id(), Some("v2"));
        assert_eq!(version.document(), Some("doc-body"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_managed_policy_document_without_version_fetches_default() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, format!("Action=GetPolicy&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}")),
                xml_response(
                    200,
                    "<GetPolicyResponse><GetPolicyResult><Policy>\
                     <PolicyName>test-policy</PolicyName><DefaultVersionId>v3</DefaultVersionId>\
                     </Policy></GetPolicyResult></GetPolicyResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    format!("Action=GetPolicyVersion&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}&VersionId=v3"),
                ),
                xml_response(
                    200,
                    "<GetPolicyVersionResponse><GetPolicyVersionResult><PolicyVersion>\
                     <Document>doc-body</Document><VersionId>v3</VersionId>\
                     </PolicyVersion></GetPolicyVersionResult></GetPolicyVersionResponse>",
                ),
            ),
        ]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let version = client
            .get_managed_policy_document(POLICY_ARN, None)
            .await
            .unwrap();

        assert_eq!(version.version_id(), Some("v3"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_managed_policy_document_falls_back_to_v1_when_no_default_version_id() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, format!("Action=GetPolicy&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}")),
                xml_response(
                    200,
                    "<GetPolicyResponse><GetPolicyResult><Policy>\
                     <PolicyName>test-policy</PolicyName></Policy>\
                     </GetPolicyResult></GetPolicyResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    format!("Action=GetPolicyVersion&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}&VersionId=v1"),
                ),
                xml_response(
                    200,
                    "<GetPolicyVersionResponse><GetPolicyVersionResult><PolicyVersion>\
                     <VersionId>v1</VersionId></PolicyVersion>\
                     </GetPolicyVersionResult></GetPolicyVersionResponse>",
                ),
            ),
        ]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let version = client
            .get_managed_policy_document(POLICY_ARN, None)
            .await
            .unwrap();

        assert_eq!(version.version_id(), Some("v1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_managed_policy_document_missing_policy_version_in_response_returns_error() {
        // `aws-sdk-iam` has no `*_correct_errors` fill-in for
        // `GetPolicyVersionOutput::policy_version` (unlike e.g.
        // `get_account_password_policy`'s output), so an empty result really
        // does deserialize to `None` here and the wrapper's defensive
        // `ok_or_else` branch is reachable.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetPolicyVersion&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}&VersionId=v2"),
            ),
            xml_response(
                200,
                "<GetPolicyVersionResponse><GetPolicyVersionResult>\
                 </GetPolicyVersionResult></GetPolicyVersionResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_managed_policy_document(POLICY_ARN, Some("v2"))
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, None);
                assert!(message.contains(POLICY_ARN));
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_managed_policy_document_error_from_get_policy_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetPolicy&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}"),
            ),
            xml_error_response("NoSuchEntity", "policy not found"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_managed_policy_document(POLICY_ARN, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("NoSuchEntity".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_managed_policy_document_error_from_get_policy_version_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetPolicyVersion&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}&VersionId=v2"),
            ),
            xml_error_response("NoSuchEntity", "version not found"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_managed_policy_document(POLICY_ARN, Some("v2"))
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("NoSuchEntity".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_access_keys_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAccessKeys&Version=2010-05-08&UserName=alice",
            ),
            xml_response(
                200,
                "<ListAccessKeysResponse><ListAccessKeysResult><AccessKeyMetadata><member>\
                 <UserName>alice</UserName><AccessKeyId>AKIAEXAMPLE</AccessKeyId>\
                 <Status>Active</Status><CreateDate>2024-01-01T00:00:00Z</CreateDate>\
                 </member></AccessKeyMetadata><IsTruncated>false</IsTruncated>\
                 </ListAccessKeysResult></ListAccessKeysResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (keys, marker) = client.list_access_keys("alice", None, None).await.unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].access_key_id(), Some("AKIAEXAMPLE"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_access_keys_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAccessKeys&Version=2010-05-08&UserName=alice&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<ListAccessKeysResponse><ListAccessKeysResult><AccessKeyMetadata></AccessKeyMetadata>\
                 <IsTruncated>false</IsTruncated></ListAccessKeysResult></ListAccessKeysResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (keys, marker) = client
            .list_access_keys("alice", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(keys.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_access_keys_capped_at_limit_aws_side_enforced() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAccessKeys&Version=2010-05-08&UserName=alice&MaxItems=1",
            ),
            xml_response(
                200,
                "<ListAccessKeysResponse><ListAccessKeysResult><AccessKeyMetadata><member>\
                 <AccessKeyId>a</AccessKeyId></member></AccessKeyMetadata>\
                 <IsTruncated>true</IsTruncated><Marker>p2</Marker>\
                 </ListAccessKeysResult></ListAccessKeysResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (keys, marker) = client
            .list_access_keys("alice", Some(1), None)
            .await
            .unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(marker, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_access_keys_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAccessKeys&Version=2010-05-08&UserName=alice",
            ),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_access_keys("alice", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_access_key_last_used_happy_path_with_data() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=GetAccessKeyLastUsed&Version=2010-05-08&AccessKeyId=AKIAEXAMPLE",
            ),
            xml_response(
                200,
                "<GetAccessKeyLastUsedResponse><GetAccessKeyLastUsedResult>\
                 <UserName>alice</UserName><AccessKeyLastUsed>\
                 <LastUsedDate>2024-01-01T00:00:00Z</LastUsedDate><ServiceName>s3</ServiceName>\
                 <Region>us-east-1</Region></AccessKeyLastUsed>\
                 </GetAccessKeyLastUsedResult></GetAccessKeyLastUsedResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let last_used = client
            .get_access_key_last_used("AKIAEXAMPLE")
            .await
            .unwrap()
            .expect("expected Some(AccessKeyLastUsed)");

        assert_eq!(last_used.service_name(), "s3");
        assert_eq!(last_used.region(), "us-east-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_access_key_last_used_none_when_never_used() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=GetAccessKeyLastUsed&Version=2010-05-08&AccessKeyId=AKIAEXAMPLE",
            ),
            xml_response(
                200,
                "<GetAccessKeyLastUsedResponse><GetAccessKeyLastUsedResult>\
                 <UserName>alice</UserName>\
                 </GetAccessKeyLastUsedResult></GetAccessKeyLastUsedResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let last_used = client
            .get_access_key_last_used("AKIAEXAMPLE")
            .await
            .unwrap();

        assert_eq!(last_used, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_access_key_last_used_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=GetAccessKeyLastUsed&Version=2010-05-08&AccessKeyId=AKIAEXAMPLE",
            ),
            xml_error_response("NoSuchEntity", "no such access key"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_access_key_last_used("AKIAEXAMPLE")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("NoSuchEntity".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_account_password_policy_happy_path_configured() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=GetAccountPasswordPolicy&Version=2010-05-08",
            ),
            xml_response(
                200,
                "<GetAccountPasswordPolicyResponse><GetAccountPasswordPolicyResult>\
                 <PasswordPolicy><MinimumPasswordLength>14</MinimumPasswordLength>\
                 <RequireSymbols>true</RequireSymbols></PasswordPolicy>\
                 </GetAccountPasswordPolicyResult></GetAccountPasswordPolicyResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let policy = client
            .get_account_password_policy()
            .await
            .unwrap()
            .expect("expected Some(PasswordPolicy)");

        assert_eq!(policy.minimum_password_length(), Some(14));
        assert!(policy.require_symbols());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_account_password_policy_none_when_not_configured() {
        // Unlike most `Option<T>` output fields in this SDK,
        // `GetAccountPasswordPolicyOutput::password_policy` has a
        // `serde_util::get_account_password_policy_output_output_correct_errors`
        // that fills in an all-default `PasswordPolicy` whenever the
        // response omits it — so an empty-body response can NOT be used to
        // exercise the wrapper's `Ok(None)` path (gotcha 14). The only way
        // to reach `Ok(None)` is the explicit `NoSuchEntityException` catch.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=GetAccountPasswordPolicy&Version=2010-05-08",
            ),
            xml_error_response("NoSuchEntity", "no custom password policy"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let policy = client.get_account_password_policy().await.unwrap();

        assert_eq!(policy, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_account_password_policy_other_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=GetAccountPasswordPolicy&Version=2010-05-08",
            ),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client.get_account_password_policy().await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_mfa_devices_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListMFADevices&Version=2010-05-08&UserName=alice"),
            xml_response(
                200,
                "<ListMFADevicesResponse><ListMFADevicesResult><MFADevices><member>\
                 <UserName>alice</UserName><SerialNumber>arn:aws:iam::123456789012:mfa/alice</SerialNumber>\
                 <EnableDate>2024-01-01T00:00:00Z</EnableDate></member></MFADevices>\
                 <IsTruncated>false</IsTruncated></ListMFADevicesResult></ListMFADevicesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (devices, marker) = client.list_mfa_devices("alice", None, None).await.unwrap();

        assert_eq!(devices.len(), 1);
        assert_eq!(
            devices[0].serial_number(),
            "arn:aws:iam::123456789012:mfa/alice"
        );
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_mfa_devices_returns_empty_vec_for_user_with_no_devices() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListMFADevices&Version=2010-05-08&UserName=bob",
            ),
            xml_response(
                200,
                "<ListMFADevicesResponse><ListMFADevicesResult><MFADevices></MFADevices>\
                 <IsTruncated>false</IsTruncated></ListMFADevicesResult></ListMFADevicesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (devices, _marker) = client.list_mfa_devices("bob", None, None).await.unwrap();

        assert!(devices.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_mfa_devices_capped_at_limit_aws_side_enforced() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListMFADevices&Version=2010-05-08&UserName=alice&MaxItems=1",
            ),
            xml_response(
                200,
                "<ListMFADevicesResponse><ListMFADevicesResult><MFADevices><member>\
                 <SerialNumber>a</SerialNumber></member></MFADevices>\
                 <IsTruncated>true</IsTruncated><Marker>p2</Marker>\
                 </ListMFADevicesResult></ListMFADevicesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (devices, marker) = client
            .list_mfa_devices("alice", Some(1), None)
            .await
            .unwrap();

        assert_eq!(devices.len(), 1);
        assert_eq!(marker, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_mfa_devices_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListMFADevices&Version=2010-05-08&UserName=alice",
            ),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_mfa_devices("alice", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_role_inline_policies_happy_path_single_policy_fanout() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListRolePolicies&Version=2010-05-08&RoleName=my-role"),
                xml_response(
                    200,
                    "<ListRolePoliciesResponse><ListRolePoliciesResult><PolicyNames>\
                     <member>inline-policy-1</member></PolicyNames><IsTruncated>false</IsTruncated>\
                     </ListRolePoliciesResult></ListRolePoliciesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetRolePolicy&Version=2010-05-08&RoleName=my-role&PolicyName=inline-policy-1",
                ),
                xml_response(
                    200,
                    "<GetRolePolicyResponse><GetRolePolicyResult><RoleName>my-role</RoleName>\
                     <PolicyName>inline-policy-1</PolicyName><PolicyDocument>doc-body</PolicyDocument>\
                     </GetRolePolicyResult></GetRolePolicyResponse>",
                ),
            ),
        ]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (results, marker) = client
            .get_role_inline_policies("my-role", None, None)
            .await
            .unwrap();

        assert_eq!(
            results,
            vec![("inline-policy-1".to_string(), "doc-body".to_string())]
        );
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_role_inline_policies_no_policies_skips_fanout() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListRolePolicies&Version=2010-05-08&RoleName=my-role"),
            xml_response(
                200,
                "<ListRolePoliciesResponse><ListRolePoliciesResult><PolicyNames></PolicyNames>\
                 <IsTruncated>false</IsTruncated></ListRolePoliciesResult></ListRolePoliciesResponse>",
            ),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (results, marker) = client
            .get_role_inline_policies("my-role", None, None)
            .await
            .unwrap();

        assert!(results.is_empty());
        assert_eq!(marker, None);
        // A single-event replay client: if the fan-out incorrectly ran for
        // an empty discovery page, the second `GetRolePolicy` call would
        // find no queued response and fail here.
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_role_inline_policies_capped_fanout_only_over_discovered_page() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=ListRolePolicies&Version=2010-05-08&RoleName=my-role&MaxItems=1",
                ),
                xml_response(
                    200,
                    "<ListRolePoliciesResponse><ListRolePoliciesResult><PolicyNames>\
                     <member>policy-a</member></PolicyNames><IsTruncated>true</IsTruncated>\
                     <Marker>p2</Marker></ListRolePoliciesResult></ListRolePoliciesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetRolePolicy&Version=2010-05-08&RoleName=my-role&PolicyName=policy-a",
                ),
                xml_response(
                    200,
                    "<GetRolePolicyResponse><GetRolePolicyResult><RoleName>my-role</RoleName>\
                     <PolicyName>policy-a</PolicyName><PolicyDocument>doc-a</PolicyDocument>\
                     </GetRolePolicyResult></GetRolePolicyResponse>",
                ),
            ),
        ]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let (results, marker) = client
            .get_role_inline_policies("my-role", Some(1), None)
            .await
            .unwrap();

        assert_eq!(results, vec![("policy-a".to_string(), "doc-a".to_string())]);
        assert_eq!(marker, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_role_inline_policies_error_in_discovery_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListRolePolicies&Version=2010-05-08&RoleName=my-role",
            ),
            xml_error_response("ServiceFailure", "internal error"),
        )]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_role_inline_policies("my-role", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("ServiceFailure".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_role_inline_policies_error_in_fanout_propagates() {
        // Unlike `connect.rs`/`elasticache.rs`'s per-item fan-out (which
        // fold errors into `None`/empty via `.ok()`/`.unwrap_or_default()`,
        // gotcha 10), this file's fan-out propagates via plain `?`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListRolePolicies&Version=2010-05-08&RoleName=my-role"),
                xml_response(
                    200,
                    "<ListRolePoliciesResponse><ListRolePoliciesResult><PolicyNames>\
                     <member>inline-policy-1</member></PolicyNames><IsTruncated>false</IsTruncated>\
                     </ListRolePoliciesResult></ListRolePoliciesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetRolePolicy&Version=2010-05-08&RoleName=my-role&PolicyName=inline-policy-1",
                ),
                xml_error_response("NoSuchEntity", "policy not found"),
            ),
        ]);
        let client = IamClient::new(&sdk_config(http_client.clone()));

        let err = client
            .get_role_inline_policies("my-role", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, .. } => assert_eq!(code, Some("NoSuchEntity".to_string())),
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
