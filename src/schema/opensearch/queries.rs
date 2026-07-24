use async_graphql::{Context, Object, Result};

use crate::aws::opensearch::OpenSearchClient;
use crate::schema::common::types::Tag;
use crate::schema::opensearch::types::{convert_opensearch_tag, OpenSearchDomain};

#[derive(Default)]
pub struct OpenSearchQuery;

#[Object]
impl OpenSearchQuery {
    /// List all OpenSearch Service domains with full configuration.
    /// Tags are intentionally omitted — use `opensearchDomainTags(arn)` to fetch
    /// tags per domain without triggering N+1 API calls.
    async fn opensearch_domains(&self, ctx: &Context<'_>) -> Result<Vec<OpenSearchDomain>> {
        let client = ctx.data::<OpenSearchClient>()?;
        let names = client.list_domain_names().await?;
        if names.is_empty() {
            return Ok(vec![]);
        }
        let statuses = client.describe_domains(&names).await?;
        Ok(statuses.into_iter().map(OpenSearchDomain::from).collect())
    }

    /// Describe a single OpenSearch Service domain by name.
    async fn opensearch_domain(
        &self,
        ctx: &Context<'_>,
        domain_name: String,
    ) -> Result<Option<OpenSearchDomain>> {
        let client = ctx.data::<OpenSearchClient>()?;
        let statuses = client.describe_domains(&[domain_name]).await?;
        Ok(statuses.into_iter().next().map(OpenSearchDomain::from))
    }

    /// Fetch tags for an OpenSearch domain by ARN.
    async fn opensearch_domain_tags(&self, ctx: &Context<'_>, arn: String) -> Result<Vec<Tag>> {
        let client = ctx.data::<OpenSearchClient>()?;
        let sdk_tags = client.list_tags(&arn).await?;
        Ok(sdk_tags.iter().map(convert_opensearch_tag).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::opensearch::OpenSearchClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::OpenSearchQuery;

    const BASE: &str = "https://es.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn opensearch_domains_lists_then_describes_and_maps_fields() {
        // `list_domain_names` (discovery) feeds its names straight into a
        // single `describe_domains` call — unlike network_firewall/eks's
        // per-item fan-out, this is one batched describe for the whole list
        // (the aws-layer's own 5-per-request chunking isn't exercised here
        // since there's only one name).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/2021-01-01/domain"), ""),
                json_response(
                    200,
                    r#"{"DomainNames":[{"DomainName":"domain-1","EngineType":"OpenSearch"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/2021-01-01/opensearch/domain-info"),
                    r#"{"DomainNames":["domain-1"]}"#,
                ),
                json_response(
                    200,
                    r#"{"DomainStatusList":[{"DomainId":"111122223333/domain-1","DomainName":"domain-1","ARN":"arn:aws:es:us-east-1:111122223333:domain/domain-1","Created":true,"Deleted":false,"EngineVersion":"OpenSearch_2.11","Endpoints":{"vpc":"vpc-endpoint.es.amazonaws.com"}}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(OpenSearchQuery)
            .data(OpenSearchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ opensearchDomains { domainId domainName arn created deleted engineVersion endpoints { key value } tags { key value } } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let domains = json["opensearchDomains"].as_array().unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0]["domainId"], "111122223333/domain-1");
        assert_eq!(domains[0]["domainName"], "domain-1");
        assert_eq!(
            domains[0]["arn"],
            "arn:aws:es:us-east-1:111122223333:domain/domain-1"
        );
        assert_eq!(domains[0]["created"], true);
        assert_eq!(domains[0]["deleted"], false);
        assert_eq!(domains[0]["engineVersion"], "OpenSearch_2.11");
        assert_eq!(domains[0]["endpoints"][0]["key"], "vpc");
        assert_eq!(
            domains[0]["endpoints"][0]["value"],
            "vpc-endpoint.es.amazonaws.com"
        );
        // Always empty per the resolver's doc comment — use
        // `opensearchDomainTags(arn)` instead.
        assert_eq!(domains[0]["tags"].as_array().unwrap().len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn opensearch_domains_skips_describe_call_when_no_domains_exist() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2021-01-01/domain"), ""),
            json_response(200, r#"{"DomainNames":[]}"#),
        )]);
        let schema = build_query_schema(OpenSearchQuery)
            .data(OpenSearchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute("{ opensearchDomains { domainId } }").await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["opensearchDomains"].as_array().unwrap().len(), 0);
        // A single queued `list_domain_names` event with no matching second
        // event left over proves `describe_domains` was never called.
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn opensearch_domain_returns_domain_when_describe_finds_it() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2021-01-01/opensearch/domain-info"),
                r#"{"DomainNames":["domain-1"]}"#,
            ),
            json_response(
                200,
                r#"{"DomainStatusList":[{"DomainId":"1/domain-1","DomainName":"domain-1","ARN":"arn:d1"}]}"#,
            ),
        )]);
        let schema = build_query_schema(OpenSearchQuery)
            .data(OpenSearchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ opensearchDomain(domainName: "domain-1") { domainId domainName } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["opensearchDomain"]["domainId"], "1/domain-1");
        assert_eq!(json["opensearchDomain"]["domainName"], "domain-1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn opensearch_domain_returns_none_when_describe_list_is_empty() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2021-01-01/opensearch/domain-info"),
                r#"{"DomainNames":["missing"]}"#,
            ),
            json_response(200, r#"{"DomainStatusList":[]}"#),
        )]);
        let schema = build_query_schema(OpenSearchQuery)
            .data(OpenSearchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ opensearchDomain(domainName: "missing") { domainId } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["opensearchDomain"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn opensearch_domain_propagates_describe_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2021-01-01/opensearch/domain-info"),
                r#"{"DomainNames":["domain-1"]}"#,
            ),
            json_error_response("ValidationException", "invalid domain name"),
        )]);
        let schema = build_query_schema(OpenSearchQuery)
            .data(OpenSearchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ opensearchDomain(domainName: "domain-1") { domainId } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("invalid domain name"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn opensearch_domain_tags_maps_sdk_tags() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!(
                    "{BASE}/2021-01-01/tags?arn=arn%3Aaws%3Aes%3Aus-east-1%3A123456789012%3Adomain%2Fdomain-1"
                ),
                "",
            ),
            json_response(200, r#"{"TagList":[{"Key":"env","Value":"prod"}]}"#),
        )]);
        let schema = build_query_schema(OpenSearchQuery)
            .data(OpenSearchClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ opensearchDomainTags(arn: "arn:aws:es:us-east-1:123456789012:domain/domain-1") { key value } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["opensearchDomainTags"][0]["key"], "env");
        assert_eq!(json["opensearchDomainTags"][0]["value"], "prod");
        http_client.relaxed_requests_match();
    }
}
