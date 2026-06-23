#[cfg(feature = "iam")]
use aws_config::SdkConfig;
#[cfg(feature = "iam")]
use aws_sdk_iam::types::{AccessKeyLastUsed, AccessKeyMetadata, AttachedPolicy, Group, MfaDevice, PasswordPolicy, Policy, PolicyVersion, Role, User};

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

    pub async fn list_roles(
        &self,
        path_prefix: Option<&str>,
    ) -> Result<Vec<Role>, VaporError> {
        let mut roles = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_roles();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for r in output.roles() {
                roles.push(r.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(roles)
    }

    pub async fn list_policies(
        &self,
        scope: &str,
        path_prefix: Option<&str>,
    ) -> Result<Vec<Policy>, VaporError> {
        use aws_sdk_iam::types::PolicyScopeType;

        let scope_val = match scope {
            "AWS" => PolicyScopeType::Aws,
            "All" => PolicyScopeType::All,
            _ => PolicyScopeType::Local,
        };

        let mut policies = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_policies().scope(scope_val.clone());
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for p in output.policies() {
                policies.push(p.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(policies)
    }

    pub async fn list_users(
        &self,
        path_prefix: Option<&str>,
    ) -> Result<Vec<User>, VaporError> {
        let mut users = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_users();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for u in output.users() {
                users.push(u.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(users)
    }

    pub async fn list_groups(
        &self,
        path_prefix: Option<&str>,
    ) -> Result<Vec<Group>, VaporError> {
        let mut groups = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_groups();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(prefix) = path_prefix {
                req = req.path_prefix(prefix);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for g in output.groups() {
                groups.push(g.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(groups)
    }

    pub async fn list_attached_role_policies(
        &self,
        role_name: &str,
    ) -> Result<Vec<AttachedPolicy>, VaporError> {
        let mut policies = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_attached_role_policies()
                .role_name(role_name);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for p in output.attached_policies() {
                policies.push(p.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(policies)
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
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        output.policy_version().cloned().ok_or_else(|| {
            VaporError::AwsSdk(format!("No policy version found for {policy_arn}"))
        })
    }

    /// List all access keys for an IAM user.
    pub async fn list_access_keys(
        &self,
        user_name: &str,
    ) -> Result<Vec<AccessKeyMetadata>, VaporError> {
        let mut keys = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_access_keys().user_name(user_name);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for k in output.access_key_metadata() {
                keys.push(k.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(keys)
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.access_key_last_used().cloned())
    }

    /// Fetch the account-wide IAM password policy.
    /// Returns `None` if no custom password policy has been configured
    /// (AWS then applies its own minimum defaults).
    pub async fn get_account_password_policy(
        &self,
    ) -> Result<Option<PasswordPolicy>, VaporError> {
        match self.inner.get_account_password_policy().send().await {
            Ok(output) => Ok(output.password_policy().cloned()),
            Err(e) => {
                if e.as_service_error()
                    .map(|se| se.is_no_such_entity_exception())
                    .unwrap_or(false)
                {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(e.to_string()))
                }
            }
        }
    }

    /// List MFA devices enrolled for an IAM user. Returns an empty vec if the
    /// user has no devices configured — enables detection of users lacking MFA
    /// (CIS AWS Benchmark 1.10).
    pub async fn list_mfa_devices(
        &self,
        user_name: &str,
    ) -> Result<Vec<MfaDevice>, VaporError> {
        let mut devices = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_mfa_devices().user_name(user_name);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for d in output.mfa_devices() {
                devices.push(d.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(devices)
    }

    /// List all inline policies embedded directly in an IAM role, returning
    /// a list of (policy_name, url_encoded_document) pairs.
    pub async fn get_role_inline_policies(
        &self,
        role_name: &str,
    ) -> Result<Vec<(String, String)>, VaporError> {
        let mut names = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_role_policies().role_name(role_name);
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for name in output.policy_names() {
                names.push(name.clone());
            }
            if output.is_truncated() {
                marker = output.marker().map(|s| s.to_string());
            } else {
                break;
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
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.push((name.clone(), output.policy_document().to_string()));
        }

        Ok(results)
    }
}
