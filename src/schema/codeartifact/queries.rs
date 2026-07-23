use async_graphql::{Context, Object, Result};

use crate::aws::codeartifact::CodeArtifactClient;
use crate::schema::codeartifact::types::{
    CodeArtifactDomain, CodeArtifactPackage, CodeArtifactRepository,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CodeArtifactQuery;

#[Object]
impl CodeArtifactQuery {
    /// Lists CodeArtifact domains, optionally capped at `limit` results and resumed via `next_token`.
    async fn code_artifact_domains(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CodeArtifactDomain>> {
        let client = ctx.data::<CodeArtifactClient>()?;
        let (items, next_token) = client.list_domains(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(CodeArtifactDomain::from).collect(),
            next_token,
        })
    }

    /// Lists repositories in a CodeArtifact domain, optionally capped at `limit` results and resumed via `next_token`.
    async fn code_artifact_repositories(
        &self,
        ctx: &Context<'_>,
        domain: String,
        domain_owner: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CodeArtifactRepository>> {
        let client = ctx.data::<CodeArtifactClient>()?;
        let (items, next_token) = client
            .list_repositories(domain, domain_owner, limit, next_token)
            .await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(CodeArtifactRepository::from)
                .collect(),
            next_token,
        })
    }

    /// Lists packages in a CodeArtifact repository, optionally capped at `limit` results and resumed via `next_token`.
    async fn code_artifact_packages(
        &self,
        ctx: &Context<'_>,
        domain: String,
        repository: String,
        format: Option<String>,
        namespace: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CodeArtifactPackage>> {
        let client = ctx.data::<CodeArtifactClient>()?;
        let (items, next_token) = client
            .list_packages(domain, repository, format, namespace, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(CodeArtifactPackage::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `CodeArtifactClient` method each (see `src/aws/codeartifact.rs`'s own test
// module for the pagination/limit/error-mapping behavior) — only light
// smoke tests are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::codeartifact::CodeArtifactClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::CodeArtifactQuery;

    const DOMAINS: &str = "https://codeartifact.us-east-1.amazonaws.com/v1/domains";
    const REPOS: &str = "https://codeartifact.us-east-1.amazonaws.com/v1/domain/repositories";
    const PACKAGES: &str = "https://codeartifact.us-east-1.amazonaws.com/v1/packages";

    #[tokio::test]
    async fn code_artifact_domains_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(DOMAINS, r#"{"maxResults":2}"#),
            json_response(
                200,
                r#"{"domains":[{"name":"d1"},{"name":"d2"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(CodeArtifactQuery)
            .data(CodeArtifactClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ codeArtifactDomains(limit: 2) { items { name } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["codeArtifactDomains"]["items"];
        assert_eq!(items[0]["name"], "d1");
        assert_eq!(items[1]["name"], "d2");
        assert_eq!(json["codeArtifactDomains"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn code_artifact_repositories_maps_items_for_given_domain() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{REPOS}?domain=my-domain&domain-owner=111122223333"),
                "",
            ),
            json_response(
                200,
                r#"{"repositories":[{"name":"repo1","domainName":"my-domain","domainOwner":"111122223333"},{"name":"repo2"}]}"#,
            ),
        )]);
        let schema = build_query_schema(CodeArtifactQuery)
            .data(CodeArtifactClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ codeArtifactRepositories(domain: "my-domain", domainOwner: "111122223333") { items { name domainName domainOwner upstreams { repositoryName } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["codeArtifactRepositories"]["items"];
        assert_eq!(items[0]["name"], "repo1");
        assert_eq!(items[0]["domainName"], "my-domain");
        assert_eq!(items[0]["domainOwner"], "111122223333");
        assert!(items[0]["upstreams"].as_array().unwrap().is_empty());
        assert_eq!(items[1]["name"], "repo2");
        assert!(json["codeArtifactRepositories"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn code_artifact_packages_maps_items_for_given_repository() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!(
                    "{PACKAGES}?domain=my-domain&repository=my-repo&format=npm&namespace=acme"
                ),
                "",
            ),
            json_response(
                200,
                r#"{"packages":[{"format":"npm","namespace":"acme","package":"left-pad"},{"format":"npm","namespace":"acme","package":"right-pad"}]}"#,
            ),
        )]);
        let schema = build_query_schema(CodeArtifactQuery)
            .data(CodeArtifactClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ codeArtifactPackages(domain: "my-domain", repository: "my-repo", format: "npm", namespace: "acme") { items { format namespace package originType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["codeArtifactPackages"]["items"];
        assert_eq!(items[0]["format"], "npm");
        assert_eq!(items[0]["namespace"], "acme");
        assert_eq!(items[0]["package"], "left-pad");
        assert!(items[0]["originType"].is_null());
        assert_eq!(items[1]["package"], "right-pad");
        assert!(json["codeArtifactPackages"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
