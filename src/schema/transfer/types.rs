use async_graphql::SimpleObject;
use aws_sdk_transfer::types::{ListedServer, ListedUser};

#[derive(SimpleObject, Clone)]
pub struct TransferServer {
    pub server_id: String,
    pub arn: Option<String>,
    pub state: Option<String>,
    pub protocols: Vec<String>,
    pub endpoint_type: Option<String>,
    pub identity_provider_type: Option<String>,
    pub domain: Option<String>,
    pub user_count: Option<i32>,
}

impl From<ListedServer> for TransferServer {
    fn from(s: ListedServer) -> Self {
        Self {
            server_id: s.server_id().unwrap_or_default().to_string(),
            arn: Some(s.arn().to_string()),
            state: s.state().map(|st| st.as_str().to_string()),
            // `ListedServer` summaries don't include the protocols list; left empty.
            protocols: vec![],
            endpoint_type: s.endpoint_type().map(|e| e.as_str().to_string()),
            identity_provider_type: s.identity_provider_type().map(|i| i.as_str().to_string()),
            domain: s.domain().map(|d| d.as_str().to_string()),
            user_count: s.user_count(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TransferUser {
    pub user_name: String,
    pub arn: Option<String>,
    pub server_id: String,
    pub role: Option<String>,
    pub home_directory: Option<String>,
    pub home_directory_type: Option<String>,
}

impl TransferUser {
    pub fn from_listed(user: ListedUser, server_id: String) -> Self {
        Self {
            user_name: user.user_name().unwrap_or_default().to_string(),
            arn: Some(user.arn().to_string()),
            server_id,
            role: user.role().map(|r| r.to_string()),
            home_directory: user.home_directory().map(|h| h.to_string()),
            home_directory_type: user.home_directory_type().map(|t| t.as_str().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_server_full() {
        let s = TransferServer {
            server_id: "s-1234567890abcdef0".to_string(),
            arn: Some("arn:aws:transfer:us-east-1:123456789012:server/s-1234567890abcdef0".to_string()),
            state: Some("ONLINE".to_string()),
            protocols: vec!["SFTP".to_string(), "FTPS".to_string()],
            endpoint_type: Some("PUBLIC".to_string()),
            identity_provider_type: Some("SERVICE_MANAGED".to_string()),
            domain: Some("S3".to_string()),
            user_count: Some(5),
        };

        assert_eq!(s.server_id, "s-1234567890abcdef0");
        assert_eq!(s.state, Some("ONLINE".to_string()));
        assert_eq!(s.protocols, vec!["SFTP".to_string(), "FTPS".to_string()]);
        assert_eq!(s.endpoint_type, Some("PUBLIC".to_string()));
        assert_eq!(s.identity_provider_type, Some("SERVICE_MANAGED".to_string()));
        assert_eq!(s.domain, Some("S3".to_string()));
        assert_eq!(s.user_count, Some(5));
    }

    #[test]
    fn test_transfer_server_minimal() {
        let s = TransferServer {
            server_id: "s-abc".to_string(),
            arn: None,
            state: None,
            protocols: vec![],
            endpoint_type: None,
            identity_provider_type: None,
            domain: None,
            user_count: None,
        };

        assert_eq!(s.server_id, "s-abc");
        assert!(s.arn.is_none());
        assert!(s.state.is_none());
        assert!(s.protocols.is_empty());
        assert!(s.endpoint_type.is_none());
        assert!(s.identity_provider_type.is_none());
        assert!(s.domain.is_none());
        assert!(s.user_count.is_none());
    }

    #[test]
    fn test_transfer_user_full() {
        let u = TransferUser {
            user_name: "alice".to_string(),
            arn: Some("arn:aws:transfer:us-east-1:123456789012:user/s-abc/alice".to_string()),
            server_id: "s-abc".to_string(),
            role: Some("arn:aws:iam::123456789012:role/transfer-role".to_string()),
            home_directory: Some("/mybucket/alice".to_string()),
            home_directory_type: Some("PATH".to_string()),
        };

        assert_eq!(u.user_name, "alice");
        assert_eq!(u.server_id, "s-abc");
        assert_eq!(u.role, Some("arn:aws:iam::123456789012:role/transfer-role".to_string()));
        assert_eq!(u.home_directory, Some("/mybucket/alice".to_string()));
        assert_eq!(u.home_directory_type, Some("PATH".to_string()));
    }

    #[test]
    fn test_transfer_user_minimal() {
        let u = TransferUser {
            user_name: "bob".to_string(),
            arn: None,
            server_id: "s-xyz".to_string(),
            role: None,
            home_directory: None,
            home_directory_type: None,
        };

        assert_eq!(u.user_name, "bob");
        assert_eq!(u.server_id, "s-xyz");
        assert!(u.arn.is_none());
        assert!(u.role.is_none());
        assert!(u.home_directory.is_none());
        assert!(u.home_directory_type.is_none());
    }
}
