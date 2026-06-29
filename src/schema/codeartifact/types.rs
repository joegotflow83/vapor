use async_graphql::SimpleObject;

use crate::aws::codeartifact::{
    CodeArtifactDomainInfo, CodeArtifactPackageInfo, CodeArtifactRepositoryInfo,
    CodeArtifactUpstreamInfo,
};

#[derive(SimpleObject, Clone)]
pub struct CodeArtifactDomain {
    pub name: Option<String>,
    pub owner: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub created_time: Option<String>,
    pub encryption_key: Option<String>,
    pub repository_count: Option<i32>,
    pub asset_size_bytes: Option<i64>,
}

impl From<CodeArtifactDomainInfo> for CodeArtifactDomain {
    fn from(d: CodeArtifactDomainInfo) -> Self {
        Self {
            name: d.name,
            owner: d.owner,
            arn: d.arn,
            status: d.status,
            created_time: d.created_time,
            encryption_key: d.encryption_key,
            repository_count: d.repository_count,
            asset_size_bytes: d.asset_size_bytes,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CodeArtifactUpstream {
    pub repository_name: String,
}

impl From<CodeArtifactUpstreamInfo> for CodeArtifactUpstream {
    fn from(u: CodeArtifactUpstreamInfo) -> Self {
        Self {
            repository_name: u.repository_name,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CodeArtifactRepository {
    pub name: Option<String>,
    pub administrator_account: Option<String>,
    pub domain_name: Option<String>,
    pub domain_owner: Option<String>,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub upstreams: Vec<CodeArtifactUpstream>,
}

impl From<CodeArtifactRepositoryInfo> for CodeArtifactRepository {
    fn from(r: CodeArtifactRepositoryInfo) -> Self {
        Self {
            name: r.name,
            administrator_account: r.administrator_account,
            domain_name: r.domain_name,
            domain_owner: r.domain_owner,
            arn: r.arn,
            description: r.description,
            upstreams: r.upstreams.into_iter().map(CodeArtifactUpstream::from).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CodeArtifactPackage {
    pub format: Option<String>,
    pub namespace: Option<String>,
    pub package: Option<String>,
    pub origin_type: Option<String>,
}

impl From<CodeArtifactPackageInfo> for CodeArtifactPackage {
    fn from(p: CodeArtifactPackageInfo) -> Self {
        Self {
            format: p.format,
            namespace: p.namespace,
            package: p.package,
            origin_type: p.origin_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::codeartifact::{
        CodeArtifactDomainInfo, CodeArtifactPackageInfo, CodeArtifactRepositoryInfo,
        CodeArtifactUpstreamInfo,
    };

    #[test]
    fn test_domain_from_full() {
        let info = CodeArtifactDomainInfo {
            name: Some("my-domain".to_string()),
            owner: Some("123456789012".to_string()),
            arn: Some("arn:aws:codeartifact:us-east-1:123456789012:domain/my-domain".to_string()),
            status: Some("Active".to_string()),
            created_time: Some("2024-01-15T10:00:00Z".to_string()),
            encryption_key: Some("arn:aws:kms:us-east-1:123456789012:key/abc123".to_string()),
            repository_count: Some(3),
            asset_size_bytes: Some(1024 * 1024),
        };
        let result = CodeArtifactDomain::from(info);
        assert_eq!(result.name, Some("my-domain".to_string()));
        assert_eq!(result.owner, Some("123456789012".to_string()));
        assert_eq!(result.status, Some("Active".to_string()));
        assert_eq!(result.repository_count, Some(3));
        assert_eq!(result.asset_size_bytes, Some(1024 * 1024));
    }

    #[test]
    fn test_domain_from_minimal() {
        let info = CodeArtifactDomainInfo {
            name: None,
            owner: None,
            arn: None,
            status: None,
            created_time: None,
            encryption_key: None,
            repository_count: None,
            asset_size_bytes: None,
        };
        let result = CodeArtifactDomain::from(info);
        assert!(result.name.is_none());
        assert!(result.repository_count.is_none());
        assert!(result.asset_size_bytes.is_none());
    }

    #[test]
    fn test_upstream_from() {
        let info = CodeArtifactUpstreamInfo {
            repository_name: "upstream-repo".to_string(),
        };
        let result = CodeArtifactUpstream::from(info);
        assert_eq!(result.repository_name, "upstream-repo");
    }

    #[test]
    fn test_repository_from_full() {
        let info = CodeArtifactRepositoryInfo {
            name: Some("my-repo".to_string()),
            administrator_account: Some("123456789012".to_string()),
            domain_name: Some("my-domain".to_string()),
            domain_owner: Some("123456789012".to_string()),
            arn: Some(
                "arn:aws:codeartifact:us-east-1:123456789012:repository/my-domain/my-repo"
                    .to_string(),
            ),
            description: Some("My repository".to_string()),
            upstreams: vec![CodeArtifactUpstreamInfo {
                repository_name: "npm-store".to_string(),
            }],
        };
        let result = CodeArtifactRepository::from(info);
        assert_eq!(result.name, Some("my-repo".to_string()));
        assert_eq!(result.domain_name, Some("my-domain".to_string()));
        assert_eq!(result.description, Some("My repository".to_string()));
        assert_eq!(result.upstreams.len(), 1);
        assert_eq!(result.upstreams[0].repository_name, "npm-store");
    }

    #[test]
    fn test_repository_from_no_upstreams() {
        let info = CodeArtifactRepositoryInfo {
            name: Some("isolated-repo".to_string()),
            administrator_account: None,
            domain_name: None,
            domain_owner: None,
            arn: None,
            description: None,
            upstreams: vec![],
        };
        let result = CodeArtifactRepository::from(info);
        assert_eq!(result.name, Some("isolated-repo".to_string()));
        assert!(result.upstreams.is_empty());
        assert!(result.description.is_none());
    }

    #[test]
    fn test_package_from_full() {
        let info = CodeArtifactPackageInfo {
            format: Some("npm".to_string()),
            namespace: Some("@my-org".to_string()),
            package: Some("my-package".to_string()),
            origin_type: Some("INTERNAL".to_string()),
        };
        let result = CodeArtifactPackage::from(info);
        assert_eq!(result.format, Some("npm".to_string()));
        assert_eq!(result.namespace, Some("@my-org".to_string()));
        assert_eq!(result.package, Some("my-package".to_string()));
        assert_eq!(result.origin_type, Some("INTERNAL".to_string()));
    }

    #[test]
    fn test_package_from_minimal() {
        let info = CodeArtifactPackageInfo {
            format: Some("pypi".to_string()),
            namespace: None,
            package: Some("requests".to_string()),
            origin_type: None,
        };
        let result = CodeArtifactPackage::from(info);
        assert_eq!(result.format, Some("pypi".to_string()));
        assert!(result.namespace.is_none());
        assert_eq!(result.package, Some("requests".to_string()));
        assert!(result.origin_type.is_none());
    }
}
