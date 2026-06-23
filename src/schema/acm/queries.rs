use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::acm::AcmClient;
use crate::schema::acm::types::AcmCertificate;

#[derive(Default)]
pub struct AcmQuery;

#[Object]
impl AcmQuery {
    /// List ACM certificates. Optionally filter by statuses (ISSUED, EXPIRED,
    /// PENDING_VALIDATION, etc.). Returns full detail including expiry and in-use resources.
    ///
    /// Note: Certificates for CloudFront must be in us-east-1; they will not appear
    /// when vapor targets another region.
    async fn acm_certificates(
        &self,
        ctx: &Context<'_>,
        statuses: Option<Vec<String>>,
    ) -> Result<Vec<AcmCertificate>> {
        let acm = ctx.data::<AcmClient>()?;
        let arns = acm.list_certificates(statuses.unwrap_or_default()).await?;

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

        Ok(certs)
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
