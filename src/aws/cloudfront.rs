use aws_config::SdkConfig;
use aws_sdk_cloudfront::types::{DistributionSummary, Tag};
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

use crate::error::VaporError;

pub struct CloudFrontClient {
    inner: aws_sdk_cloudfront::Client,
}

impl CloudFrontClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudfront::Client::new(config),
        }
    }

    /// Lists distributions, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListDistributions` has both
    /// `max_items` and `marker` (verified against pinned `aws-sdk-cloudfront`
    /// 1.123.0's `operation/list_distributions/_list_distributions_input.rs`)
    /// with no documented minimum on `max_items` (unlike elasticache/neptune's
    /// 20-100 floor), so `limit` is capped to the remaining budget on the
    /// request itself with no client-side truncation needed
    /// (`backup.rs::list_backup_vaults` pattern). Continuation uses
    /// `distribution_list.next_marker`, only present while
    /// `distribution_list.is_truncated()` is true.
    pub async fn list_distributions(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DistributionSummary>, Option<String>), VaporError> {
        let mut distributions: Vec<DistributionSummary> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.list_distributions();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(l) = limit {
                req = req.max_items((l - distributions.len() as i32).max(1));
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            let Some(dist_list) = output.distribution_list() else {
                marker = None;
                break;
            };
            for d in dist_list.items() {
                distributions.push(d.clone());
            }
            marker = if dist_list.is_truncated() {
                dist_list.next_marker().map(|s| s.to_string())
            } else {
                None
            };

            match (&marker, limit) {
                (None, _) => break,
                (_, Some(l)) if distributions.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((distributions, marker))
    }

    /// Fetch tags for a distribution by ARN.
    pub async fn list_tags_for_resource(&self, arn: &str) -> Result<Vec<Tag>, VaporError> {
        let output = self
            .inner
            .list_tags_for_resource()
            .resource(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.tags().map(|t| t.items().to_vec()).unwrap_or_default())
    }

    /// Fetch a single distribution by ID. Returns None if not found.
    pub async fn get_distribution(
        &self,
        id: &str,
    ) -> Result<Option<aws_sdk_cloudfront::types::Distribution>, VaporError> {
        match self.inner.get_distribution().id(id).send().await {
            Ok(output) => Ok(output.distribution().cloned()),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_no_such_distribution() {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk {
                        code: svc_err.code().map(String::from),
                        message: svc_err
                            .message()
                            .map(String::from)
                            .unwrap_or_else(|| svc_err.to_string()),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const DISTRIBUTIONS: &str = "https://cloudfront.amazonaws.com/2020-05-31/distribution";

    fn distribution_summary_xml(id: &str, domain_name: &str) -> String {
        format!(
            "<DistributionSummary>\
                <Id>{id}</Id>\
                <ARN>arn:aws:cloudfront::123456789012:distribution/{id}</ARN>\
                <Status>Deployed</Status>\
                <DomainName>{domain_name}</DomainName>\
                <Comment>test distribution</Comment>\
                <PriceClass>PriceClass_All</PriceClass>\
                <Enabled>true</Enabled>\
            </DistributionSummary>"
        )
    }

    fn distribution_list_xml(items: &[String], is_truncated: bool, next_marker: Option<&str>) -> String {
        let next_marker_el = next_marker
            .map(|m| format!("<NextMarker>{m}</NextMarker>"))
            .unwrap_or_default();
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
    async fn list_distributions_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(DISTRIBUTIONS, ""),
            xml_response(
                200,
                distribution_list_xml(
                    &[
                        distribution_summary_xml("E1ONE", "d1.cloudfront.net"),
                        distribution_summary_xml("E2TWO", "d2.cloudfront.net"),
                    ],
                    false,
                    None,
                ),
            ),
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let (distributions, token) = client.list_distributions(None, None).await.unwrap();

        assert_eq!(distributions.len(), 2);
        assert_eq!(distributions[0].id(), "E1ONE");
        assert_eq!(distributions[1].domain_name(), "d2.cloudfront.net");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_distributions_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{DISTRIBUTIONS}?Marker=cursor-a"), ""),
            xml_response(
                200,
                distribution_list_xml(&[distribution_summary_xml("E3THREE", "d3.cloudfront.net")], false, None),
            ),
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let (distributions, token) = client
            .list_distributions(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(distributions.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_distributions_stops_at_limit_and_returns_resume_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{DISTRIBUTIONS}?MaxItems=2"), ""),
            xml_response(
                200,
                distribution_list_xml(
                    &[
                        distribution_summary_xml("E1ONE", "d1.cloudfront.net"),
                        distribution_summary_xml("E2TWO", "d2.cloudfront.net"),
                    ],
                    true,
                    Some("page2"),
                ),
            ),
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let (distributions, token) = client.list_distributions(Some(2), None).await.unwrap();

        assert_eq!(distributions.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_distributions_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{DISTRIBUTIONS}?MaxItems=10"), ""),
                xml_response(
                    200,
                    distribution_list_xml(
                        &[
                            distribution_summary_xml("E1ONE", "d1.cloudfront.net"),
                            distribution_summary_xml("E2TWO", "d2.cloudfront.net"),
                        ],
                        true,
                        Some("p2"),
                    ),
                ),
            ),
            ReplayEvent::new(
                request(&format!("{DISTRIBUTIONS}?Marker=p2&MaxItems=8"), ""),
                xml_response(
                    200,
                    distribution_list_xml(&[distribution_summary_xml("E3THREE", "d3.cloudfront.net")], false, None),
                ),
            ),
        ]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let (distributions, token) = client.list_distributions(Some(10), None).await.unwrap();

        assert_eq!(distributions.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_distributions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(DISTRIBUTIONS, ""),
            xml_error_response("AccessDenied", "not authorized"),
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let err = client.list_distributions(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDenied".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_resource_returns_tags() {
        let arn = "arn:aws:cloudfront::123456789012:distribution/E1ONE";
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://cloudfront.amazonaws.com/2020-05-31/tagging?Resource=arn%3Aaws%3Acloudfront%3A%3A123456789012%3Adistribution%2FE1ONE",
                "",
            ),
            xml_response(
                200,
                "<Tags><Items><Tag><Key>env</Key><Value>prod</Value></Tag></Items></Tags>",
            ),
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let tags = client.list_tags_for_resource(arn).await.unwrap();

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), "env");
        assert_eq!(tags[0].value(), Some("prod"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_distribution_returns_some_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
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
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let distribution = client.get_distribution("E1ONE").await.unwrap();

        assert_eq!(distribution.map(|d| d.id().to_string()), Some("E1ONE".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_distribution_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{DISTRIBUTIONS}/missing"), ""),
            xml_error_response("NoSuchDistribution", "The specified distribution does not exist."),
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let distribution = client.get_distribution("missing").await.unwrap();

        assert!(distribution.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_distribution_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{DISTRIBUTIONS}/E1ONE"), ""),
            xml_error_response("AccessDenied", "not authorized"),
        )]);
        let client = CloudFrontClient::new(&sdk_config(http_client.clone()));

        let err = client.get_distribution("E1ONE").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDenied".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
