use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::cloudfront::CloudFrontClient;
use crate::schema::cloudfront::types::{
    CfDistribution, distribution_from_get, distribution_from_summary, map_tags,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct CloudFrontQuery;

#[Object]
impl CloudFrontQuery {
    /// List CloudFront distributions with tags fetched concurrently.
    async fn cloudfront_distributions(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<CfDistribution>> {
        let cf = ctx.data::<CloudFrontClient>()?;
        let (summaries, next_token) = cf.list_distributions(limit, next_token).await?;

        let futures: Vec<_> = summaries
            .iter()
            .map(|d| {
                let arn = d.arn().to_string();
                async move {
                    let tags = cf
                        .list_tags_for_resource(&arn)
                        .await
                        .unwrap_or_default();
                    let tags = map_tags(tags);
                    distribution_from_summary(d, tags)
                }
            })
            .collect();

        Ok(Page {
            items: join_all(futures).await,
            next_token,
        })
    }

    /// Fetch a single CloudFront distribution by ID.
    async fn cloudfront_distribution(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<CfDistribution>> {
        let cf = ctx.data::<CloudFrontClient>()?;
        let dist = match cf.get_distribution(&id).await? {
            Some(d) => d,
            None => return Ok(None),
        };
        let arn = dist.arn().to_string();
        let tags = cf.list_tags_for_resource(&arn).await.unwrap_or_default();
        let tags = map_tags(tags);
        Ok(Some(distribution_from_get(&dist, tags)))
    }
}

// Both resolvers have real logic beyond a bare passthrough: `cloudfront_distributions`
// fans out a `list_tags_for_resource` call per discovered distribution via `join_all`
// and silently swallows tag-fetch errors (`unwrap_or_default`); `cloudfront_distribution`
// has an early-return branch (no tag fetch at all) when `get_distribution` finds nothing.
// Per the resolver-layer sweep's stated scope this gets bespoke coverage rather than a
// single light smoke test.
#[cfg(test)]
mod tests {
    use crate::aws::cloudfront::CloudFrontClient;
    use crate::aws::test_util::{
        request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::CloudFrontQuery;

    const DISTRIBUTIONS: &str = "https://cloudfront.amazonaws.com/2020-05-31/distribution";

    fn distribution_summary_xml(id: &str) -> String {
        format!(
            "<DistributionSummary>\
                <Id>{id}</Id>\
                <ARN>arn:aws:cloudfront::123456789012:distribution/{id}</ARN>\
                <Status>Deployed</Status>\
                <DomainName>{id}.cloudfront.net</DomainName>\
                <Comment></Comment>\
                <PriceClass>PriceClass_All</PriceClass>\
                <Enabled>true</Enabled>\
            </DistributionSummary>"
        )
    }

    fn distribution_list_xml(items: &[String], next_marker: Option<&str>) -> String {
        let (is_truncated, next_marker_el) = match next_marker {
            Some(m) => (true, format!("<NextMarker>{m}</NextMarker>")),
            None => (false, String::new()),
        };
        format!(
            "<DistributionList>\
                <Marker></Marker>\
                <MaxItems>100</MaxItems>\
                <IsTruncated>{is_truncated}</IsTruncated>\
                {next_marker_el}\
                <Quantity>{quantity}</Quantity>\
                <Items>{items}</Items>\
            </DistributionList>",
            quantity = items.len(),
            items = items.join(""),
        )
    }

    #[tokio::test]
    async fn cloudfront_distributions_maps_items_fetches_tags_and_forwards_next_token() {
        // Two sequential calls per the resolver's own code: `list_distributions`
        // (discovery), then a per-distribution `list_tags_for_resource` fan-out
        // (acm.rs precedent — fan-out order is deterministic under
        // `StaticReplayClient` since nothing here truly suspends the executor).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{DISTRIBUTIONS}?MaxItems=1"), ""),
                xml_response(
                    200,
                    distribution_list_xml(&[distribution_summary_xml("E1ONE")], Some("page2")),
                ),
            ),
            ReplayEvent::new(
                request(
                    "https://cloudfront.amazonaws.com/2020-05-31/tagging?Resource=arn%3Aaws%3Acloudfront%3A%3A123456789012%3Adistribution%2FE1ONE",
                    "",
                ),
                xml_response(
                    200,
                    "<Tags><Items><Tag><Key>env</Key><Value>prod</Value></Tag></Items></Tags>",
                ),
            ),
        ]);
        let schema = build_query_schema(CloudFrontQuery)
            .data(CloudFrontClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ cloudfrontDistributions(limit: 1) { items { id arn domainName
                 status tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cloudfrontDistributions"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["id"], "E1ONE");
        assert_eq!(items[0]["domainName"], "E1ONE.cloudfront.net");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["cloudfrontDistributions"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cloudfront_distributions_swallows_tag_fetch_errors() {
        // `list_tags_for_resource`'s error is discarded via `unwrap_or_default()`
        // in the resolver — the distribution still comes back, just with no tags,
        // rather than failing the whole query.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(DISTRIBUTIONS, ""),
                xml_response(200, distribution_list_xml(&[distribution_summary_xml("E1ONE")], None)),
            ),
            ReplayEvent::new(
                request(
                    "https://cloudfront.amazonaws.com/2020-05-31/tagging?Resource=arn%3Aaws%3Acloudfront%3A%3A123456789012%3Adistribution%2FE1ONE",
                    "",
                ),
                xml_error_response("AccessDenied", "not authorized"),
            ),
        ]);
        let schema = build_query_schema(CloudFrontQuery)
            .data(CloudFrontClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ cloudfrontDistributions { items { id tags { key value } } } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["cloudfrontDistributions"]["items"];
        assert_eq!(items[0]["id"], "E1ONE");
        assert!(items[0]["tags"].as_array().unwrap().is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cloudfront_distribution_returns_full_detail_with_tags() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{DISTRIBUTIONS}/E1ONE"), ""),
                xml_response(
                    200,
                    "<Distribution>\
                        <Id>E1ONE</Id>\
                        <ARN>arn:aws:cloudfront::123456789012:distribution/E1ONE</ARN>\
                        <Status>Deployed</Status>\
                        <DomainName>d1.cloudfront.net</DomainName>\
                    </Distribution>",
                ),
            ),
            ReplayEvent::new(
                request(
                    "https://cloudfront.amazonaws.com/2020-05-31/tagging?Resource=arn%3Aaws%3Acloudfront%3A%3A123456789012%3Adistribution%2FE1ONE",
                    "",
                ),
                xml_response(
                    200,
                    "<Tags><Items><Tag><Key>env</Key><Value>prod</Value></Tag></Items></Tags>",
                ),
            ),
        ]);
        let schema = build_query_schema(CloudFrontQuery)
            .data(CloudFrontClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ cloudfrontDistribution(id: "E1ONE") { id domainName tags { key value } } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["cloudfrontDistribution"]["id"], "E1ONE");
        assert_eq!(json["cloudfrontDistribution"]["domainName"], "d1.cloudfront.net");
        assert_eq!(json["cloudfrontDistribution"]["tags"][0]["key"], "env");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn cloudfront_distribution_returns_none_when_not_found_and_skips_tag_fetch() {
        // Only one queued event: if the resolver didn't early-return on `None`
        // and tried to fetch tags anyway, `StaticReplayClient` would fail the
        // test with "no more test data available".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{DISTRIBUTIONS}/missing"), ""),
            xml_error_response("NoSuchDistribution", "The specified distribution does not exist."),
        )]);
        let schema = build_query_schema(CloudFrontQuery)
            .data(CloudFrontClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ cloudfrontDistribution(id: "missing") { id } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["cloudfrontDistribution"].is_null());
        http_client.relaxed_requests_match();
    }
}
