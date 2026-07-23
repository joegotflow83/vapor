#[cfg(feature = "acm")]
use aws_config::SdkConfig;
#[cfg(feature = "acm")]
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
#[cfg(feature = "acm")]
use aws_sdk_acm::types::{CertificateDetail, CertificateStatus, Tag as AcmTag};

#[cfg(feature = "acm")]
use crate::error::VaporError;

#[cfg(feature = "acm")]
pub struct AcmClient {
    inner: aws_sdk_acm::Client,
}

impl AcmClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_acm::Client::new(config),
        }
    }

    /// Lists certificate ARNs, optionally filtered by `statuses`, capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `limit` is handed to AWS via `ListCertificatesInput::max_items` (this
    /// operation's `max_results`-equivalent field name, verified against
    /// pinned `aws-sdk-acm` 1.106.0's `_list_certificates_input.rs`) so a
    /// capped page boundary lands exactly on the returned token
    /// (kinesis/mq pattern).
    pub async fn list_certificates(
        &self,
        statuses: Vec<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let status_enums: Vec<CertificateStatus> = statuses
            .iter()
            .map(|s| CertificateStatus::from(s.as_str()))
            .collect();

        let mut arns: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_certificates();
            if !status_enums.is_empty() {
                req = req.set_certificate_statuses(Some(status_enums.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - arns.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for summary in output.certificate_summary_list.unwrap_or_default() {
                if let Some(arn) = summary.certificate_arn {
                    arns.push(arn);
                }
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if arns.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((arns, token))
    }

    /// Fetch full certificate metadata. Returns None if the certificate does not exist.
    pub async fn describe_certificate(&self, arn: &str) -> Result<Option<CertificateDetail>, VaporError> {
        match self.inner.describe_certificate().certificate_arn(arn).send().await {
            Ok(output) => Ok(output.certificate),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_resource_not_found_exception() {
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

    /// Fetch tags for a certificate.
    pub async fn list_tags_for_certificate(&self, arn: &str) -> Result<Vec<AcmTag>, VaporError> {
        let output = self
            .inner
            .list_tags_for_certificate()
            .certificate_arn(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.tags().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://acm.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_certificates_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateStatuses":["EXPIRED"]}"#),
            json_response(
                200,
                r#"{"CertificateSummaryList":[{"CertificateArn":"arn1"},{"CertificateArn":"arn2"}]}"#,
            ),
        )]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let (arns, token) = client
            .list_certificates(vec!["EXPIRED".to_string()], None, None)
            .await
            .unwrap();

        assert_eq!(arns, vec!["arn1".to_string(), "arn2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificates_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"CertificateSummaryList":[{"CertificateArn":"arn3"}]}"#),
        )]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let (arns, token) = client
            .list_certificates(vec![], None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(arns, vec!["arn3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificates_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxItems":2}"#),
            json_response(
                200,
                r#"{"CertificateSummaryList":[{"CertificateArn":"a1"},{"CertificateArn":"a2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let (arns, token) = client.list_certificates(vec![], Some(2), None).await.unwrap();

        assert_eq!(arns, vec!["a1".to_string(), "a2".to_string()]);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificates_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxItems":10}"#),
                json_response(
                    200,
                    r#"{"CertificateSummaryList":[{"CertificateArn":"a1"},{"CertificateArn":"a2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxItems":8}"#),
                json_response(200, r#"{"CertificateSummaryList":[{"CertificateArn":"a3"}]}"#),
            ),
        ]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let (arns, token) = client.list_certificates(vec![], Some(10), None).await.unwrap();

        assert_eq!(arns, vec!["a1".to_string(), "a2".to_string(), "a3".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_certificate_returns_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateArn":"arn:aws:acm:us-east-1:1:certificate/abc"}"#),
            json_response(
                200,
                r#"{"Certificate":{"CertificateArn":"arn:aws:acm:us-east-1:1:certificate/abc","DomainName":"example.com","Status":"ISSUED"}}"#,
            ),
        )]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let detail = client
            .describe_certificate("arn:aws:acm:us-east-1:1:certificate/abc")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(detail.domain_name(), Some("example.com"));
        assert_eq!(detail.status(), Some(&CertificateStatus::Issued));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_certificate_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateArn":"arn:missing"}"#),
            json_error_response("ResourceNotFoundException", "certificate not found"),
        )]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let detail = client.describe_certificate("arn:missing").await.unwrap();

        assert_eq!(detail, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_certificate_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateArn":"arn:denied"}"#),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_certificate("arn:denied").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_for_certificate_returns_tags() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateArn":"arn:aws:acm:us-east-1:1:certificate/abc"}"#),
            json_response(
                200,
                r#"{"Tags":[{"Key":"env","Value":"prod"},{"Key":"team","Value":"platform"}]}"#,
            ),
        )]);
        let client = AcmClient::new(&sdk_config(http_client.clone()));

        let tags = client
            .list_tags_for_certificate("arn:aws:acm:us-east-1:1:certificate/abc")
            .await
            .unwrap();

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].key(), "env");
        assert_eq!(tags[0].value(), Some("prod"));
        http_client.relaxed_requests_match();
    }
}
