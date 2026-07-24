use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::acm::AcmClient;
use crate::schema::acm::types::AcmCertificate;
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AcmQuery;

#[Object]
impl AcmQuery {
    /// List ACM certificates. Optionally filter by statuses (ISSUED, EXPIRED,
    /// PENDING_VALIDATION, etc.). Returns full detail including expiry and in-use resources.
    /// `limit` caps the total number of results across pages (default: unlimited).
    ///
    /// Note: Certificates for CloudFront must be in us-east-1; they will not appear
    /// when vapor targets another region.
    async fn acm_certificates(
        &self,
        ctx: &Context<'_>,
        statuses: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AcmCertificate>> {
        let acm = ctx.data::<AcmClient>()?;
        let (arns, next_token) = acm
            .list_certificates(statuses.unwrap_or_default(), limit, next_token)
            .await?;

        let futures: Vec<_> = arns
            .iter()
            .map(|arn| {
                let arn = arn.clone();
                async move {
                    let detail = acm.describe_certificate(&arn).await;
                    let tags = acm.list_tags_for_certificate(&arn).await;
                    (arn, detail, tags)
                }
            })
            .collect();

        let results = join_all(futures).await;

        let mut certs = Vec::new();
        for (arn, detail_result, tags_result) in results {
            match detail_result {
                Ok(Some(detail)) => {
                    let tags = tags_result.unwrap_or_default();
                    certs.push(AcmCertificate::from_detail_and_tags(detail, tags));
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to describe certificate {arn}: {e}"
                    )));
                }
            }
        }

        Ok(Page {
            items: certs,
            next_token,
        })
    }

    /// Fetch a single ACM certificate by ARN.
    async fn acm_certificate(
        &self,
        ctx: &Context<'_>,
        arn: String,
    ) -> Result<Option<AcmCertificate>> {
        let acm = ctx.data::<AcmClient>()?;
        let (detail_result, tags_result) = tokio::join!(
            acm.describe_certificate(&arn),
            acm.list_tags_for_certificate(&arn),
        );
        let tags = tags_result.unwrap_or_default();
        match detail_result? {
            Some(detail) => Ok(Some(AcmCertificate::from_detail_and_tags(detail, tags))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::acm::AcmClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::AcmQuery;

    const ENDPOINT: &str = "https://acm.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn acm_certificates_maps_fan_out_detail_and_tags() {
        // Three sequential calls per the resolver's own code:
        // `list_certificates` (discovery), then per-arn `describe_certificate`
        // + `list_tags_for_certificate` (fan-out) — the mock connector serves
        // responses strictly in the order requests are sent, so this list
        // must mirror that order exactly (memory gotcha: fan-out order is
        // deterministic under `StaticReplayClient` since nothing here truly
        // suspends the executor).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"CertificateSummaryList":[{"CertificateArn":"arn1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn1"}"#),
                json_response(
                    200,
                    r#"{"Certificate":{"CertificateArn":"arn1","DomainName":"example.com","Status":"ISSUED"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn1"}"#),
                json_response(200, r#"{"Tags":[{"Key":"env","Value":"prod"}]}"#),
            ),
        ]);
        let schema = build_query_schema(AcmQuery)
            .data(AcmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ acmCertificates { items { certificateArn domainName status \
                 tags { key value } } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["acmCertificates"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["certificateArn"], "arn1");
        assert_eq!(items[0]["domainName"], "example.com");
        assert_eq!(items[0]["status"], "ISSUED");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert!(json["acmCertificates"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn acm_certificates_drops_arn_whose_describe_call_returns_not_found() {
        // `describe_certificate` mapping `ResourceNotFoundException` to
        // `Ok(None)` (see `src/aws/acm.rs`) makes the resolver silently skip
        // that arn from the result — but `list_tags_for_certificate` is
        // still awaited unconditionally in the same fan-out future (no
        // early-return on `None`), so a matching event is still required.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"CertificateSummaryList":[{"CertificateArn":"arn-missing"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn-missing"}"#),
                json_error_response("ResourceNotFoundException", "certificate not found"),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn-missing"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
        ]);
        let schema = build_query_schema(AcmQuery)
            .data(AcmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ acmCertificates { items { certificateArn } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["acmCertificates"]["items"].as_array().unwrap().len(),
            0
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn acm_certificates_propagates_non_not_found_describe_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(
                    200,
                    r#"{"CertificateSummaryList":[{"CertificateArn":"arn-denied"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn-denied"}"#),
                json_error_response("AccessDeniedException", "not authorized"),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn-denied"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
        ]);
        let schema = build_query_schema(AcmQuery)
            .data(AcmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ acmCertificates { items { certificateArn } nextToken } }")
            .await;

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("arn-denied"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn acm_certificates_passes_statuses_limit_and_next_token_to_discovery_call() {
        // No certificates returned, so no fan-out calls follow — isolates
        // the discovery-call argument-passthrough behavior.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"CertificateStatuses":["EXPIRED"],"NextToken":"cursor-a","MaxItems":5}"#,
            ),
            json_response(200, r#"{"CertificateSummaryList":[]}"#),
        )]);
        let schema = build_query_schema(AcmQuery)
            .data(AcmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ acmCertificates(statuses: ["EXPIRED"], limit: 5, nextToken: "cursor-a") { items { certificateArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn acm_certificate_returns_full_detail_with_tags() {
        // `acm_certificate` fires `describe_certificate`/`list_tags_for_certificate`
        // concurrently via `tokio::join!` rather than sequentially — under
        // `StaticReplayClient`'s synchronous mock connector this still
        // resolves in the futures' declaration order (describe, then tags).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn1"}"#),
                json_response(
                    200,
                    r#"{"Certificate":{"CertificateArn":"arn1","DomainName":"example.com","Status":"ISSUED"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn1"}"#),
                json_response(200, r#"{"Tags":[{"Key":"env","Value":"prod"}]}"#),
            ),
        ]);
        let schema = build_query_schema(AcmQuery)
            .data(AcmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ acmCertificate(arn: "arn1") { certificateArn domainName tags { key value } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["acmCertificate"]["certificateArn"], "arn1");
        assert_eq!(json["acmCertificate"]["domainName"], "example.com");
        assert_eq!(json["acmCertificate"]["tags"][0]["key"], "env");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn acm_certificate_returns_none_when_certificate_missing() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn-missing"}"#),
                json_error_response("ResourceNotFoundException", "certificate not found"),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateArn":"arn-missing"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
        ]);
        let schema = build_query_schema(AcmQuery)
            .data(AcmClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ acmCertificate(arn: "arn-missing") { certificateArn } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["acmCertificate"].is_null());
        http_client.relaxed_requests_match();
    }
}
