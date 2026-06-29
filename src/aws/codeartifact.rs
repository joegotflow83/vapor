use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct CodeArtifactDomainInfo {
    pub name: Option<String>,
    pub owner: Option<String>,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub created_time: Option<String>,
    pub encryption_key: Option<String>,
    pub repository_count: Option<i32>,
    pub asset_size_bytes: Option<i64>,
}

pub struct CodeArtifactUpstreamInfo {
    pub repository_name: String,
}

pub struct CodeArtifactRepositoryInfo {
    pub name: Option<String>,
    pub administrator_account: Option<String>,
    pub domain_name: Option<String>,
    pub domain_owner: Option<String>,
    pub arn: Option<String>,
    pub description: Option<String>,
    pub upstreams: Vec<CodeArtifactUpstreamInfo>,
}

pub struct CodeArtifactPackageInfo {
    pub format: Option<String>,
    pub namespace: Option<String>,
    pub package: Option<String>,
    pub origin_type: Option<String>,
}

pub struct CodeArtifactClient {
    inner: aws_sdk_codeartifact::Client,
}

impl CodeArtifactClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codeartifact::Client::new(config),
        }
    }

    pub async fn list_domains(&self) -> Result<Vec<CodeArtifactDomainInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_domains();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for domain in output.domains() {
                items.push(CodeArtifactDomainInfo {
                    name: domain.name().map(|s| s.to_string()),
                    owner: domain.owner().map(|s| s.to_string()),
                    arn: domain.arn().map(|s| s.to_string()),
                    status: domain.status().map(|s| s.as_str().to_string()),
                    created_time: domain.created_time().map(|t| t.to_string()),
                    encryption_key: domain.encryption_key().map(|s| s.to_string()),
                    repository_count: None,
                    asset_size_bytes: None,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_repositories(
        &self,
        domain: String,
        domain_owner: Option<String>,
    ) -> Result<Vec<CodeArtifactRepositoryInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_repositories_in_domain()
                .domain(&domain);
            if let Some(ref owner) = domain_owner {
                req = req.domain_owner(owner);
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for repo in output.repositories() {
                items.push(CodeArtifactRepositoryInfo {
                    name: repo.name().map(|s| s.to_string()),
                    administrator_account: repo.administrator_account().map(|s| s.to_string()),
                    domain_name: repo.domain_name().map(|s| s.to_string()),
                    domain_owner: repo.domain_owner().map(|s| s.to_string()),
                    arn: repo.arn().map(|s| s.to_string()),
                    description: repo.description().map(|s| s.to_string()),
                    upstreams: Vec::new(),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_packages(
        &self,
        domain: String,
        repository: String,
        format: Option<String>,
        namespace: Option<String>,
    ) -> Result<Vec<CodeArtifactPackageInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_packages()
                .domain(&domain)
                .repository(&repository);
            if let Some(ref fmt) = format {
                req = req.format(aws_sdk_codeartifact::types::PackageFormat::from(
                    fmt.as_str(),
                ));
            }
            if let Some(ref ns) = namespace {
                req = req.namespace(ns);
            }
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for pkg in output.packages() {
                items.push(CodeArtifactPackageInfo {
                    format: pkg.format().map(|f| f.as_str().to_string()),
                    namespace: pkg.namespace().map(|s| s.to_string()),
                    package: pkg.package().map(|s| s.to_string()),
                    // origin_type requires N+1 describe_package; omitted from list
                    origin_type: None,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
