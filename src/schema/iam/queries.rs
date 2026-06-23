use async_graphql::{Context, Object, Result};

use crate::aws::iam::IamClient;
use crate::schema::iam::types::{
    IamAccessKey, IamAttachedPolicy, IamGroup, IamInlinePolicy, IamMfaDevice, IamPasswordPolicy,
    IamPolicy, IamPolicyDocument, IamRole, IamUser,
};

#[derive(Default)]
pub struct IamQuery;

#[Object]
impl IamQuery {
    /// List IAM roles. Optionally filter by path prefix (e.g. "/service-role/").
    async fn iam_roles(
        &self,
        ctx: &Context<'_>,
        path_prefix: Option<String>,
    ) -> Result<Vec<IamRole>> {
        let iam = ctx.data::<IamClient>()?;
        let roles = iam.list_roles(path_prefix.as_deref()).await?;
        Ok(roles.into_iter().map(IamRole::from).collect())
    }

    /// List IAM managed policies. scope accepts "Local" (default, customer-managed),
    /// "AWS" (AWS-managed), or "All". Optionally filter by path prefix.
    async fn iam_policies(
        &self,
        ctx: &Context<'_>,
        scope: Option<String>,
        path_prefix: Option<String>,
    ) -> Result<Vec<IamPolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let scope_str = scope.as_deref().unwrap_or("Local");
        let policies = iam.list_policies(scope_str, path_prefix.as_deref()).await?;
        Ok(policies.into_iter().map(IamPolicy::from).collect())
    }

    /// List IAM users. Optionally filter by path prefix.
    async fn iam_users(
        &self,
        ctx: &Context<'_>,
        path_prefix: Option<String>,
    ) -> Result<Vec<IamUser>> {
        let iam = ctx.data::<IamClient>()?;
        let users = iam.list_users(path_prefix.as_deref()).await?;
        Ok(users.into_iter().map(IamUser::from).collect())
    }

    /// List IAM groups. Optionally filter by path prefix.
    async fn iam_groups(
        &self,
        ctx: &Context<'_>,
        path_prefix: Option<String>,
    ) -> Result<Vec<IamGroup>> {
        let iam = ctx.data::<IamClient>()?;
        let groups = iam.list_groups(path_prefix.as_deref()).await?;
        Ok(groups.into_iter().map(IamGroup::from).collect())
    }

    /// List policies attached to an IAM role.
    async fn iam_attached_role_policies(
        &self,
        ctx: &Context<'_>,
        role_name: String,
    ) -> Result<Vec<IamAttachedPolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let policies = iam.list_attached_role_policies(&role_name).await?;
        Ok(policies.into_iter().map(IamAttachedPolicy::from).collect())
    }

    /// Fetch the JSON document for a managed IAM policy.
    /// Optionally specify a version_id (e.g. "v3"); defaults to the policy's current default version.
    async fn iam_policy_document(
        &self,
        ctx: &Context<'_>,
        policy_arn: String,
        version_id: Option<String>,
    ) -> Result<IamPolicyDocument> {
        let iam = ctx.data::<IamClient>()?;
        let version = iam
            .get_managed_policy_document(&policy_arn, version_id.as_deref())
            .await?;
        Ok(IamPolicyDocument::from((policy_arn, version)))
    }

    /// List all inline policies embedded directly in an IAM role, including their decoded JSON documents.
    async fn iam_role_inline_policies(
        &self,
        ctx: &Context<'_>,
        role_name: String,
    ) -> Result<Vec<IamInlinePolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let policies = iam.get_role_inline_policies(&role_name).await?;
        Ok(policies.into_iter().map(IamInlinePolicy::from).collect())
    }

    /// Fetch the account-wide IAM password policy.
    /// Returns null if no custom password policy has been configured —
    /// AWS then applies minimal defaults (8-char minimum, no complexity requirements).
    /// Use this to audit CIS AWS Benchmark 1.x controls: minimum length ≥14,
    /// all complexity flags enabled, max_password_age ≤90, reuse prevention ≥24,
    /// hard_expiry enabled.
    async fn iam_password_policy(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<IamPasswordPolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let policy = iam.get_account_password_policy().await?;
        Ok(policy.map(IamPasswordPolicy::from))
    }

    /// List MFA devices enrolled for an IAM user.
    /// Returns an empty list when the user has no MFA configured.
    /// Cross-reference with iamUsers to detect users lacking MFA (CIS AWS Benchmark 1.10).
    async fn iam_mfa_devices(
        &self,
        ctx: &Context<'_>,
        user_name: String,
    ) -> Result<Vec<IamMfaDevice>> {
        let iam = ctx.data::<IamClient>()?;
        let devices = iam.list_mfa_devices(&user_name).await?;
        Ok(devices.into_iter().map(IamMfaDevice::from).collect())
    }

    /// List access keys for an IAM user, enriched with last-used metadata.
    /// Use this to detect stale or inactive credentials (keys unused for 90+ days,
    /// keys that are Inactive, etc.).
    async fn iam_access_keys(
        &self,
        ctx: &Context<'_>,
        user_name: String,
    ) -> Result<Vec<IamAccessKey>> {
        let iam = ctx.data::<IamClient>()?;
        let keys = iam.list_access_keys(&user_name).await?;
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            let key_id = key.access_key_id().unwrap_or("").to_string();
            let last_used = iam.get_access_key_last_used(&key_id).await?;
            result.push(IamAccessKey::from((key, last_used)));
        }
        Ok(result)
    }
}
