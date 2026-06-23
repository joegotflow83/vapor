use async_graphql::{Context, Object, Result};

use crate::aws::opensearch::OpenSearchClient;
use crate::schema::ec2::types::Tag;
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
    async fn opensearch_domain_tags(
        &self,
        ctx: &Context<'_>,
        arn: String,
    ) -> Result<Vec<Tag>> {
        let client = ctx.data::<OpenSearchClient>()?;
        let sdk_tags = client.list_tags(&arn).await?;
        Ok(sdk_tags.iter().map(convert_opensearch_tag).collect())
    }
}
