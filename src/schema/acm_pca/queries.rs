use async_graphql::{Context, Object, Result};

use crate::aws::acm_pca::AcmPcaClient;
use crate::schema::acm_pca::types::PrivateCa;
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct AcmPcaQuery;

#[Object]
impl AcmPcaQuery {
    /// Lists private certificate authorities. `limit` caps the number of
    /// results returned per page (default: unlimited); `nextToken` resumes
    /// from a previous page.
    async fn private_certificate_authorities(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<PrivateCa>> {
        let client = ctx.data::<AcmPcaClient>()?;
        let (cas, next_token) = client
            .list_certificate_authorities(limit, next_token)
            .await?;
        Ok(Page {
            items: cas.into_iter().map(PrivateCa::from).collect(),
            next_token,
        })
    }

    async fn private_certificate_authority(
        &self,
        ctx: &Context<'_>,
        certificate_authority_arn: String,
    ) -> Result<Option<PrivateCa>> {
        let client = ctx.data::<AcmPcaClient>()?;
        let ca = client
            .describe_certificate_authority(&certificate_authority_arn)
            .await?;
        Ok(ca.map(PrivateCa::from))
    }
}

// Both resolvers are 1:1 passthroughs to a single already-tested
// `AcmPcaClient` method (see `src/aws/acm_pca.rs`'s own test module for the
// fan-out/pagination/error-mapping behavior) — only light smoke tests are
// needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::acm_pca::AcmPcaClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::AcmPcaQuery;

    const ENDPOINT: &str = "https://acm-pca.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn private_certificate_authorities_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"cursor-a","MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"CertificateAuthorities":[{"Arn":"arn-1","Status":"ACTIVE"}],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-1"}"#),
                json_response(200, r#"{"Tags":[{"Key":"env","Value":"prod"}]}"#),
            ),
        ]);
        let schema = build_query_schema(AcmPcaQuery)
            .data(AcmPcaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ privateCertificateAuthorities(limit: 1, nextToken: "cursor-a") { items { arn status tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["privateCertificateAuthorities"]["items"];
        assert_eq!(items[0]["arn"], "arn-1");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(json["privateCertificateAuthorities"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn private_certificate_authority_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-1"}"#),
                json_response(
                    200,
                    r#"{"CertificateAuthority":{"Arn":"arn-1","Status":"ACTIVE"}}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-1"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
        ]);
        let schema = build_query_schema(AcmPcaQuery)
            .data(AcmPcaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ privateCertificateAuthority(certificateAuthorityArn: "arn-1") { arn status } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["privateCertificateAuthority"]["arn"], "arn-1");
        assert_eq!(json["privateCertificateAuthority"]["status"], "ACTIVE");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn private_certificate_authority_returns_none_when_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-missing"}"#),
            json_response(200, r#"{}"#),
        )]);
        let schema = build_query_schema(AcmPcaQuery)
            .data(AcmPcaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ privateCertificateAuthority(certificateAuthorityArn: "arn-missing") { arn } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["privateCertificateAuthority"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn private_certificate_authority_propagates_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-denied"}"#),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let schema = build_query_schema(AcmPcaQuery)
            .data(AcmPcaClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ privateCertificateAuthority(certificateAuthorityArn: "arn-denied") { arn } }"#)
            .await;

        assert_eq!(res.errors.len(), 1);
        http_client.relaxed_requests_match();
    }
}
