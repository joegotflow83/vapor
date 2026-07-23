use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct CodeArtifactClient {
    inner: aws_sdk_codeartifact::Client,
}

impl CodeArtifactClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_codeartifact::Client::new(config),
        }
    }

    /// Lists CodeArtifact domains, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_domains(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_codeartifact::types::DomainSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_domains();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.domains.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists repositories in a CodeArtifact domain, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_repositories(
        &self,
        domain: String,
        domain_owner: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_codeartifact::types::RepositorySummary>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_repositories_in_domain().domain(&domain);
            if let Some(ref owner) = domain_owner {
                req = req.domain_owner(owner);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.repositories.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists packages in a CodeArtifact repository, optionally capped at `limit` results and resumed via `next_token`.
    pub async fn list_packages(
        &self,
        domain: String,
        repository: String,
        format: Option<String>,
        namespace: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_codeartifact::types::PackageSummary>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut token = next_token;

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
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.packages.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const DOMAINS: &str = "https://codeartifact.us-east-1.amazonaws.com/v1/domains";
    const REPOS: &str = "https://codeartifact.us-east-1.amazonaws.com/v1/domain/repositories";
    const PACKAGES: &str = "https://codeartifact.us-east-1.amazonaws.com/v1/packages";

    #[tokio::test]
    async fn list_domains_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(DOMAINS, "{}"),
            json_response(
                200,
                r#"{"domains":[{"name":"domain1","owner":"111122223333"},{"name":"domain2","owner":"111122223333"}]}"#,
            ),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (domains, token) = client.list_domains(None, None).await.unwrap();

        assert_eq!(domains.len(), 2);
        assert_eq!(domains[0].name(), Some("domain1"));
        assert_eq!(domains[1].owner(), Some("111122223333"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_domains_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(DOMAINS, r#"{"nextToken":"cursor-a"}"#),
            json_response(200, r#"{"domains":[{"name":"domain3"}]}"#),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (domains, token) = client
            .list_domains(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(domains.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_domains_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(DOMAINS, r#"{"maxResults":2}"#),
            json_response(
                200,
                r#"{"domains":[{"name":"d1"},{"name":"d2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (domains, token) = client.list_domains(Some(2), None).await.unwrap();

        assert_eq!(domains.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_domains_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(DOMAINS, r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"domains":[{"name":"d1"},{"name":"d2"}],"nextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(DOMAINS, r#"{"maxResults":8,"nextToken":"p2"}"#),
                json_response(200, r#"{"domains":[{"name":"d3"}]}"#),
            ),
        ]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (domains, token) = client.list_domains(Some(10), None).await.unwrap();

        assert_eq!(domains.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_domains_propagates_errors() {
        // `ValidationException` (not a throttling exception — see
        // apigateway.rs's precedent for why that would consume a second
        // replay event via the SDK's default retry strategy).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(DOMAINS, "{}"),
            json_error_response("ValidationException", "invalid request"),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let err = client.list_domains(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{REPOS}?domain=my-domain&domain-owner=111122223333"), ""),
            json_response(
                200,
                r#"{"repositories":[{"name":"repo1","domainName":"my-domain","domainOwner":"111122223333"},{"name":"repo2"}]}"#,
            ),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client
            .list_repositories(
                "my-domain".to_string(),
                Some("111122223333".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name(), Some("repo1"));
        assert_eq!(repos[0].domain_owner(), Some("111122223333"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{REPOS}?domain=my-domain&max-results=1"), ""),
            json_response(
                200,
                r#"{"repositories":[{"name":"repo1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (repos, token) = client
            .list_repositories("my-domain".to_string(), None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_repositories_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{REPOS}?domain=my-domain"), ""),
            json_error_response("ValidationException", "invalid domain"),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_repositories("my-domain".to_string(), None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "invalid domain");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_packages_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{PACKAGES}?domain=my-domain&repository=my-repo&format=npm&namespace=acme"),
                "",
            ),
            json_response(
                200,
                r#"{"packages":[{"format":"npm","namespace":"acme","package":"left-pad"},{"format":"npm","namespace":"acme","package":"right-pad"}]}"#,
            ),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (packages, token) = client
            .list_packages(
                "my-domain".to_string(),
                "my-repo".to_string(),
                Some("npm".to_string()),
                Some("acme".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].package(), Some("left-pad"));
        assert_eq!(packages[1].namespace(), Some("acme"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_packages_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{PACKAGES}?domain=my-domain&repository=my-repo&max-results=1"),
                "",
            ),
            json_response(
                200,
                r#"{"packages":[{"format":"npm","package":"left-pad"}],"nextToken":"page2"}"#,
            ),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let (packages, token) = client
            .list_packages(
                "my-domain".to_string(),
                "my-repo".to_string(),
                None,
                None,
                Some(1),
                None,
            )
            .await
            .unwrap();

        assert_eq!(packages.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_packages_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{PACKAGES}?domain=my-domain&repository=my-repo"),
                "",
            ),
            json_error_response("ValidationException", "invalid repository"),
        )]);
        let client = CodeArtifactClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_packages(
                "my-domain".to_string(),
                "my-repo".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "invalid repository");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
