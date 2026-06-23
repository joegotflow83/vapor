use async_graphql::SimpleObject;
use aws_sdk_sts::operation::get_caller_identity::GetCallerIdentityOutput;

#[derive(SimpleObject, Clone)]
pub struct CallerIdentity {
    pub account: Option<String>,
    pub arn: Option<String>,
    pub user_id: Option<String>,
}

impl From<GetCallerIdentityOutput> for CallerIdentity {
    fn from(output: GetCallerIdentityOutput) -> Self {
        Self {
            account: output.account().map(|s| s.to_string()),
            arn: output.arn().map(|s| s.to_string()),
            user_id: output.user_id().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caller_identity_from_output() {
        let output = GetCallerIdentityOutput::builder()
            .account("123456789012")
            .arn("arn:aws:iam::123456789012:user/testuser")
            .user_id("AIDAJEXAMPLE")
            .build();
        let identity = CallerIdentity::from(output);
        assert_eq!(identity.account.as_deref(), Some("123456789012"));
        assert_eq!(identity.arn.as_deref(), Some("arn:aws:iam::123456789012:user/testuser"));
        assert_eq!(identity.user_id.as_deref(), Some("AIDAJEXAMPLE"));
    }

    #[test]
    fn test_caller_identity_from_empty_output() {
        let output = GetCallerIdentityOutput::builder().build();
        let identity = CallerIdentity::from(output);
        assert!(identity.account.is_none());
        assert!(identity.arn.is_none());
        assert!(identity.user_id.is_none());
    }
}
