use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct SsoInstance {
    pub instance_arn: Option<String>,
    pub identity_store_id: Option<String>,
    pub owner_account_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
}

impl From<aws_sdk_ssoadmin::types::InstanceMetadata> for SsoInstance {
    fn from(i: aws_sdk_ssoadmin::types::InstanceMetadata) -> Self {
        Self {
            instance_arn: i.instance_arn().map(|s| s.to_string()),
            identity_store_id: i.identity_store_id().map(|s| s.to_string()),
            owner_account_id: i.owner_account_id().map(|s| s.to_string()),
            name: i.name().map(|s| s.to_string()),
            status: i.status().map(|s| s.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SsoPermissionSet {
    pub permission_set_arn: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_date: Option<String>,
    pub session_duration: Option<String>,
    pub relay_state: Option<String>,
}

impl From<aws_sdk_ssoadmin::types::PermissionSet> for SsoPermissionSet {
    fn from(p: aws_sdk_ssoadmin::types::PermissionSet) -> Self {
        Self {
            permission_set_arn: p.permission_set_arn().map(|s| s.to_string()),
            name: p.name().map(|s| s.to_string()),
            description: p.description().map(|s| s.to_string()),
            created_date: p.created_date().map(|d| d.to_string()),
            session_duration: p.session_duration().map(|s| s.to_string()),
            relay_state: p.relay_state().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SsoAccountAssignment {
    pub account_id: Option<String>,
    pub permission_set_arn: Option<String>,
    pub principal_type: Option<String>,
    pub principal_id: Option<String>,
}

impl From<aws_sdk_ssoadmin::types::AccountAssignment> for SsoAccountAssignment {
    fn from(a: aws_sdk_ssoadmin::types::AccountAssignment) -> Self {
        Self {
            account_id: a.account_id().map(|s| s.to_string()),
            permission_set_arn: a.permission_set_arn().map(|s| s.to_string()),
            principal_type: a.principal_type().map(|s| s.as_str().to_string()),
            principal_id: a.principal_id().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sso_instance_from_minimal() {
        let instance = aws_sdk_ssoadmin::types::InstanceMetadata::builder().build();
        let result = SsoInstance::from(instance);
        assert!(result.instance_arn.is_none());
        assert!(result.identity_store_id.is_none());
        assert!(result.status.is_none());
    }

    #[test]
    fn test_sso_instance_from_full() {
        let instance = aws_sdk_ssoadmin::types::InstanceMetadata::builder()
            .instance_arn("arn:aws:sso:::instance/ssoins-abc123")
            .identity_store_id("d-12345678")
            .owner_account_id("123456789012")
            .name("MyInstance")
            .status(aws_sdk_ssoadmin::types::InstanceStatus::Active)
            .build();
        let result = SsoInstance::from(instance);
        assert_eq!(
            result.instance_arn,
            Some("arn:aws:sso:::instance/ssoins-abc123".to_string())
        );
        assert_eq!(result.identity_store_id, Some("d-12345678".to_string()));
        assert_eq!(result.owner_account_id, Some("123456789012".to_string()));
        assert_eq!(result.name, Some("MyInstance".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
    }

    #[test]
    fn test_sso_permission_set_from_minimal() {
        let ps = aws_sdk_ssoadmin::types::PermissionSet::builder().build();
        let result = SsoPermissionSet::from(ps);
        assert!(result.permission_set_arn.is_none());
        assert!(result.name.is_none());
        assert!(result.description.is_none());
    }

    #[test]
    fn test_sso_permission_set_from_full() {
        let ps = aws_sdk_ssoadmin::types::PermissionSet::builder()
            .permission_set_arn(
                "arn:aws:sso:::permissionSet/ssoins-abc123/ps-xyz789",
            )
            .name("AdminAccess")
            .description("Full admin access")
            .session_duration("PT8H")
            .relay_state("https://console.aws.amazon.com/")
            .build();
        let result = SsoPermissionSet::from(ps);
        assert_eq!(
            result.permission_set_arn,
            Some("arn:aws:sso:::permissionSet/ssoins-abc123/ps-xyz789".to_string())
        );
        assert_eq!(result.name, Some("AdminAccess".to_string()));
        assert_eq!(result.description, Some("Full admin access".to_string()));
        assert_eq!(result.session_duration, Some("PT8H".to_string()));
        assert_eq!(
            result.relay_state,
            Some("https://console.aws.amazon.com/".to_string())
        );
    }

    #[test]
    fn test_sso_account_assignment_from() {
        let assignment = aws_sdk_ssoadmin::types::AccountAssignment::builder()
            .account_id("123456789012")
            .permission_set_arn("arn:aws:sso:::permissionSet/ssoins-abc123/ps-xyz789")
            .principal_type(aws_sdk_ssoadmin::types::PrincipalType::User)
            .principal_id("user-id-abc")
            .build();
        let result = SsoAccountAssignment::from(assignment);
        assert_eq!(result.account_id, Some("123456789012".to_string()));
        assert_eq!(
            result.permission_set_arn,
            Some("arn:aws:sso:::permissionSet/ssoins-abc123/ps-xyz789".to_string())
        );
        assert_eq!(result.principal_type, Some("USER".to_string()));
        assert_eq!(result.principal_id, Some("user-id-abc".to_string()));
    }

    #[test]
    fn test_sso_account_assignment_group_principal() {
        let assignment = aws_sdk_ssoadmin::types::AccountAssignment::builder()
            .principal_type(aws_sdk_ssoadmin::types::PrincipalType::Group)
            .principal_id("group-id-xyz")
            .build();
        let result = SsoAccountAssignment::from(assignment);
        assert_eq!(result.principal_type, Some("GROUP".to_string()));
        assert_eq!(result.principal_id, Some("group-id-xyz".to_string()));
        assert!(result.account_id.is_none());
    }
}
