use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct UserPool {
    pub id: String,
    pub name: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
    pub last_modified_date: Option<String>,
    pub mfa_configuration: Option<String>,
    pub estimated_number_of_users: i32,
    pub deletion_protection: Option<String>,
}

impl UserPool {
    pub fn from_sdk(pool: &aws_sdk_cognitoidentityprovider::types::UserPoolType) -> Self {
        Self {
            id: pool.id().unwrap_or_default().to_string(),
            name: pool.name().map(|s| s.to_string()),
            arn: pool.arn().map(|s| s.to_string()),
            status: pool.status().map(|s| s.as_str().to_string()),
            creation_date: pool.creation_date().map(|t| t.to_string()),
            last_modified_date: pool.last_modified_date().map(|t| t.to_string()),
            mfa_configuration: pool.mfa_configuration().map(|m| m.as_str().to_string()),
            estimated_number_of_users: pool.estimated_number_of_users(),
            deletion_protection: pool.deletion_protection().map(|d| d.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct UserPoolClient {
    pub client_id: String,
    pub client_name: Option<String>,
    pub user_pool_id: String,
    pub explicit_auth_flows: Vec<String>,
    pub token_validity_units: Option<String>,
}

impl UserPoolClient {
    pub fn from_sdk(c: aws_sdk_cognitoidentityprovider::types::UserPoolClientType) -> Self {
        Self {
            client_id: c.client_id().unwrap_or_default().to_string(),
            client_name: c.client_name().map(|s| s.to_string()),
            user_pool_id: c.user_pool_id().unwrap_or_default().to_string(),
            explicit_auth_flows: c
                .explicit_auth_flows()
                .iter()
                .map(|f| f.as_str().to_string())
                .collect(),
            token_validity_units: c.token_validity_units().and_then(|t| {
                let mut parts = Vec::new();
                if let Some(u) = t.access_token() {
                    parts.push(format!("access={}", u.as_str()));
                }
                if let Some(u) = t.id_token() {
                    parts.push(format!("id={}", u.as_str()));
                }
                if let Some(u) = t.refresh_token() {
                    parts.push(format!("refresh={}", u.as_str()));
                }
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join(","))
                }
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_pool_from_sdk_minimal() {
        let pool = aws_sdk_cognitoidentityprovider::types::UserPoolType::builder().build();
        let result = UserPool::from_sdk(&pool);
        assert_eq!(result.id, "");
        assert!(result.name.is_none());
        assert!(result.arn.is_none());
        assert!(result.status.is_none());
        assert_eq!(result.estimated_number_of_users, 0);
        assert!(result.deletion_protection.is_none());
    }

    #[test]
    fn test_user_pool_from_sdk_full() {
        let pool = aws_sdk_cognitoidentityprovider::types::UserPoolType::builder()
            .id("us-east-1_abc123")
            .name("MyPool")
            .arn("arn:aws:cognito-idp:us-east-1:123456789012:userpool/us-east-1_abc123")
            .estimated_number_of_users(42)
            .build();
        let result = UserPool::from_sdk(&pool);
        assert_eq!(result.id, "us-east-1_abc123");
        assert_eq!(result.name, Some("MyPool".to_string()));
        assert_eq!(
            result.arn,
            Some("arn:aws:cognito-idp:us-east-1:123456789012:userpool/us-east-1_abc123".to_string())
        );
        assert_eq!(result.estimated_number_of_users, 42);
    }

    #[test]
    fn test_user_pool_client_from_sdk() {
        use aws_sdk_cognitoidentityprovider::types::ExplicitAuthFlowsType;
        let client = aws_sdk_cognitoidentityprovider::types::UserPoolClientType::builder()
            .client_id("client123")
            .client_name("MyApp")
            .user_pool_id("us-east-1_abc123")
            .explicit_auth_flows(ExplicitAuthFlowsType::AllowUserSrpAuth)
            .explicit_auth_flows(ExplicitAuthFlowsType::AllowRefreshTokenAuth)
            .build();
        let result = UserPoolClient::from_sdk(client);
        assert_eq!(result.client_id, "client123");
        assert_eq!(result.client_name, Some("MyApp".to_string()));
        assert_eq!(result.user_pool_id, "us-east-1_abc123");
        assert_eq!(result.explicit_auth_flows.len(), 2);
        assert!(result.explicit_auth_flows.contains(&"ALLOW_USER_SRP_AUTH".to_string()));
        assert!(result.token_validity_units.is_none());
    }

    #[test]
    fn test_user_pool_client_from_sdk_minimal() {
        let client = aws_sdk_cognitoidentityprovider::types::UserPoolClientType::builder().build();
        let result = UserPoolClient::from_sdk(client);
        assert_eq!(result.client_id, "");
        assert!(result.client_name.is_none());
        assert_eq!(result.user_pool_id, "");
        assert!(result.explicit_auth_flows.is_empty());
        assert!(result.token_validity_units.is_none());
    }
}
