use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use crate::schema::time::to_utc;

#[derive(SimpleObject, Clone)]
pub struct CodeArtifactDomain {
    pub name: Option<String>,
    pub owner: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub created_time: Option<DateTime<Utc>>,
    pub encryption_key: Option<String>,
    pub repository_count: Option<i32>,
    pub asset_size_bytes: Option<i64>,
}

impl From<aws_sdk_codeartifact::types::DomainSummary> for CodeArtifactDomain {
    fn from(d: aws_sdk_codeartifact::types::DomainSummary) -> Self {
        Self {
            name: d.name,
            owner: d.owner,
            arn: d.arn,
            status: d.status.map(|s| s.as_str().to_string()),
            created_time: to_utc(d.created_time.as_ref()),
            encryption_key: d.encryption_key,
            // repository_count/asset_size_bytes are only on DomainDescription
            // (describe_domain), not DomainSummary; omitted to avoid an N+1
            // fan-out over every listed domain.
            repository_count: None,
            asset_size_bytes: None,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CodeArtifactUpstream {
    pub repository_name: String,
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

impl From<aws_sdk_codeartifact::types::RepositorySummary> for CodeArtifactRepository {
    fn from(r: aws_sdk_codeartifact::types::RepositorySummary) -> Self {
        Self {
            name: r.name,
            administrator_account: r.administrator_account,
            domain_name: r.domain_name,
            domain_owner: r.domain_owner,
            arn: r.arn,
            description: r.description,
            // upstreams is only on RepositoryDescription (describe_repository),
            // not RepositorySummary; omitted to avoid an N+1 fan-out over
            // every listed repository.
            upstreams: Vec::new(),
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

impl From<aws_sdk_codeartifact::types::PackageSummary> for CodeArtifactPackage {
    fn from(p: aws_sdk_codeartifact::types::PackageSummary) -> Self {
        Self {
            format: p.format.map(|f| f.as_str().to_string()),
            namespace: p.namespace,
            package: p.package,
            // origin_type lives on PackageVersionOrigin (per version, via
            // describe_package_version), not on PackageSummary; no package-level
            // field exists without an N+1 fan-out over every version.
            origin_type: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_codeartifact::types::{DomainStatus, PackageFormat};
    use aws_smithy_types::DateTime as SmithyDateTime;

    #[test]
    fn test_domain_from_full() {
        let sdk = aws_sdk_codeartifact::types::DomainSummary::builder()
            .name("my-domain")
            .owner("123456789012")
            .arn("arn:aws:codeartifact:us-east-1:123456789012:domain/my-domain")
            .status(DomainStatus::Active)
            .created_time(SmithyDateTime::from_secs(1_700_000_000))
            .encryption_key("arn:aws:kms:us-east-1:123456789012:key/abc123")
            .build();
        let result = CodeArtifactDomain::from(sdk);
        assert_eq!(result.name, Some("my-domain".to_string()));
        assert_eq!(result.owner, Some("123456789012".to_string()));
        assert_eq!(result.status, Some("Active".to_string()));
        assert!(result.created_time.is_some());
        assert!(result.repository_count.is_none());
        assert!(result.asset_size_bytes.is_none());
    }

    #[test]
    fn test_domain_from_minimal() {
        let sdk = aws_sdk_codeartifact::types::DomainSummary::builder().build();
        let result = CodeArtifactDomain::from(sdk);
        assert!(result.name.is_none());
        assert!(result.created_time.is_none());
        assert!(result.repository_count.is_none());
        assert!(result.asset_size_bytes.is_none());
    }

    #[test]
    fn test_repository_from_full() {
        let sdk = aws_sdk_codeartifact::types::RepositorySummary::builder()
            .name("my-repo")
            .administrator_account("123456789012")
            .domain_name("my-domain")
            .domain_owner("123456789012")
            .arn("arn:aws:codeartifact:us-east-1:123456789012:repository/my-domain/my-repo")
            .description("My repository")
            .build();
        let result = CodeArtifactRepository::from(sdk);
        assert_eq!(result.name, Some("my-repo".to_string()));
        assert_eq!(result.domain_name, Some("my-domain".to_string()));
        assert_eq!(result.description, Some("My repository".to_string()));
        // upstreams is only populated by describe_repository, never by the
        // list_repositories_in_domain source of this From impl.
        assert!(result.upstreams.is_empty());
    }

    #[test]
    fn test_repository_from_minimal() {
        let sdk = aws_sdk_codeartifact::types::RepositorySummary::builder()
            .name("isolated-repo")
            .build();
        let result = CodeArtifactRepository::from(sdk);
        assert_eq!(result.name, Some("isolated-repo".to_string()));
        assert!(result.upstreams.is_empty());
        assert!(result.description.is_none());
    }

    #[test]
    fn test_package_from_full() {
        let sdk = aws_sdk_codeartifact::types::PackageSummary::builder()
            .format(PackageFormat::Npm)
            .namespace("@my-org")
            .package("my-package")
            .build();
        let result = CodeArtifactPackage::from(sdk);
        assert_eq!(result.format, Some("npm".to_string()));
        assert_eq!(result.namespace, Some("@my-org".to_string()));
        assert_eq!(result.package, Some("my-package".to_string()));
        assert!(result.origin_type.is_none());
    }

    #[test]
    fn test_package_from_minimal() {
        let sdk = aws_sdk_codeartifact::types::PackageSummary::builder()
            .format(PackageFormat::Pypi)
            .package("requests")
            .build();
        let result = CodeArtifactPackage::from(sdk);
        assert_eq!(result.format, Some("pypi".to_string()));
        assert!(result.namespace.is_none());
        assert_eq!(result.package, Some("requests".to_string()));
        assert!(result.origin_type.is_none());
    }
}
