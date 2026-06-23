use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct OrgAccount {
    pub id: String,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
    pub joined_method: Option<String>,
    pub joined_timestamp: Option<String>,
}

impl From<&aws_sdk_organizations::types::Account> for OrgAccount {
    fn from(a: &aws_sdk_organizations::types::Account) -> Self {
        Self {
            id: a.id().unwrap_or_default().to_string(),
            arn: a.arn().map(|s| s.to_string()),
            name: a.name().map(|s| s.to_string()),
            email: a.email().map(|s| s.to_string()),
            status: a.status().map(|s| s.as_str().to_string()),
            joined_method: a.joined_method().map(|m| m.as_str().to_string()),
            joined_timestamp: a.joined_timestamp().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct OrganizationalUnit {
    pub id: String,
    pub arn: Option<String>,
    pub name: Option<String>,
}

impl From<&aws_sdk_organizations::types::OrganizationalUnit> for OrganizationalUnit {
    fn from(ou: &aws_sdk_organizations::types::OrganizationalUnit) -> Self {
        Self {
            id: ou.id().unwrap_or_default().to_string(),
            arn: ou.arn().map(|s| s.to_string()),
            name: ou.name().map(|s| s.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct OrgPolicy {
    pub id: String,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub policy_type: Option<String>,
    pub aws_managed: bool,
}

impl From<&aws_sdk_organizations::types::PolicySummary> for OrgPolicy {
    fn from(p: &aws_sdk_organizations::types::PolicySummary) -> Self {
        Self {
            id: p.id().unwrap_or_default().to_string(),
            arn: p.arn().map(|s| s.to_string()),
            name: p.name().map(|s| s.to_string()),
            description: p.description().map(|s| s.to_string()),
            policy_type: p.r#type().map(|t| t.as_str().to_string()),
            aws_managed: p.aws_managed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_org_account_from_sdk() {
        let account = aws_sdk_organizations::types::Account::builder()
            .id("123456789012")
            .arn("arn:aws:organizations::123456789012:account/o-abc/123456789012")
            .name("Production")
            .email("prod@example.com")
            .status(aws_sdk_organizations::types::AccountStatus::Active)
            .joined_method(aws_sdk_organizations::types::AccountJoinedMethod::Created)
            .joined_timestamp(aws_sdk_organizations::primitives::DateTime::from_secs(1_000_000))
            .build();

        let result = OrgAccount::from(&account);
        assert_eq!(result.id, "123456789012");
        assert_eq!(result.arn, Some("arn:aws:organizations::123456789012:account/o-abc/123456789012".to_string()));
        assert_eq!(result.name, Some("Production".to_string()));
        assert_eq!(result.email, Some("prod@example.com".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
        assert_eq!(result.joined_method, Some("CREATED".to_string()));
        assert!(result.joined_timestamp.is_some());
    }

    #[test]
    fn test_org_account_minimal() {
        let account = aws_sdk_organizations::types::Account::builder().build();
        let result = OrgAccount::from(&account);
        assert_eq!(result.id, "");
        assert!(result.arn.is_none());
        assert!(result.name.is_none());
        assert!(result.email.is_none());
        assert!(result.status.is_none());
        assert!(result.joined_method.is_none());
        assert!(result.joined_timestamp.is_none());
    }

    #[test]
    fn test_organizational_unit_from_sdk() {
        let ou = aws_sdk_organizations::types::OrganizationalUnit::builder()
            .id("ou-abc-12345")
            .arn("arn:aws:organizations::123456789012:ou/o-abc/ou-abc-12345")
            .name("Engineering")
            .build();

        let result = OrganizationalUnit::from(&ou);
        assert_eq!(result.id, "ou-abc-12345");
        assert_eq!(result.arn, Some("arn:aws:organizations::123456789012:ou/o-abc/ou-abc-12345".to_string()));
        assert_eq!(result.name, Some("Engineering".to_string()));
    }

    #[test]
    fn test_organizational_unit_minimal() {
        let ou = aws_sdk_organizations::types::OrganizationalUnit::builder().build();
        let result = OrganizationalUnit::from(&ou);
        assert_eq!(result.id, "");
        assert!(result.arn.is_none());
        assert!(result.name.is_none());
    }

    #[test]
    fn test_org_policy_from_sdk() {
        let policy = aws_sdk_organizations::types::PolicySummary::builder()
            .id("p-12345")
            .arn("arn:aws:organizations::123456789012:policy/o-abc/service_control_policy/p-12345")
            .name("DenyAll")
            .description("Deny all actions")
            .r#type(aws_sdk_organizations::types::PolicyType::ServiceControlPolicy)
            .aws_managed(false)
            .build();

        let result = OrgPolicy::from(&policy);
        assert_eq!(result.id, "p-12345");
        assert_eq!(result.arn, Some("arn:aws:organizations::123456789012:policy/o-abc/service_control_policy/p-12345".to_string()));
        assert_eq!(result.name, Some("DenyAll".to_string()));
        assert_eq!(result.description, Some("Deny all actions".to_string()));
        assert_eq!(result.policy_type, Some("SERVICE_CONTROL_POLICY".to_string()));
        assert!(!result.aws_managed);
    }

    #[test]
    fn test_org_policy_aws_managed() {
        let policy = aws_sdk_organizations::types::PolicySummary::builder()
            .id("p-FullAWSAccess")
            .name("FullAWSAccess")
            .r#type(aws_sdk_organizations::types::PolicyType::ServiceControlPolicy)
            .aws_managed(true)
            .build();

        let result = OrgPolicy::from(&policy);
        assert_eq!(result.id, "p-FullAWSAccess");
        assert!(result.aws_managed);
    }
}
