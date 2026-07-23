use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct PrivateCaInfo {
    pub inner: aws_sdk_acmpca::types::CertificateAuthority,
    pub tags: Vec<(String, String)>,
}

pub struct AcmPcaClient {
    inner: aws_sdk_acmpca::Client,
}

impl AcmPcaClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_acmpca::Client::new(config),
        }
    }

    // Internal per-CA tag enrichment fan-out, not a top-level list query —
    // keeps draining all pages via `into_paginator()` (mq.rs
    // `describe_broker` precedent: only the caller-facing list op below
    // converts to the token-passthrough pattern).
    async fn fetch_tags(&self, arn: &str) -> Vec<(String, String)> {
        let mut tags = Vec::new();
        let mut pages = self
            .inner
            .list_tags()
            .certificate_authority_arn(arn)
            .into_paginator()
            .send();

        while let Some(page) = pages.try_next().await.unwrap_or(None) {
            for tag in page.tags.unwrap_or_default() {
                tags.push((tag.key, tag.value.unwrap_or_default()));
            }
        }
        tags
    }

    /// Lists private CAs, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListCertificateAuthorities`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-acmpca` 1.108.0's
    /// `operation/list_certificate_authorities/_list_certificate_authorities_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself
    /// (mq.rs `list_brokers` pattern). The N+1 `fetch_tags` fan-out only
    /// covers the single page of CAs collected this call, not the whole
    /// collection.
    pub async fn list_certificate_authorities(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<PrivateCaInfo>, Option<String>), VaporError> {
        let mut cas = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_certificate_authorities();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - cas.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            cas.extend(output.certificate_authorities.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if cas.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut result = Vec::with_capacity(cas.len());
        for ca in cas {
            let arn = ca.arn().unwrap_or_default().to_string();
            let tags = if arn.is_empty() {
                vec![]
            } else {
                self.fetch_tags(&arn).await
            };
            result.push(PrivateCaInfo { inner: ca, tags });
        }
        Ok((result, token))
    }

    pub async fn describe_certificate_authority(
        &self,
        certificate_authority_arn: &str,
    ) -> Result<Option<PrivateCaInfo>, VaporError> {
        let output = self
            .inner
            .describe_certificate_authority()
            .certificate_authority_arn(certificate_authority_arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        match output.certificate_authority().cloned() {
            Some(ca) => {
                let tags = self.fetch_tags(certificate_authority_arn).await;
                Ok(Some(PrivateCaInfo { inner: ca, tags }))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://acm-pca.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_certificate_authorities_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(
                    200,
                    r#"{"CertificateAuthorities":[{"Arn":"arn:aws:acm-pca:us-east-1:1:certificate-authority/abc","Status":"ACTIVE"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"CertificateAuthorityArn":"arn:aws:acm-pca:us-east-1:1:certificate-authority/abc"}"#,
                ),
                json_response(200, r#"{"Tags":[{"Key":"env","Value":"prod"}]}"#),
            ),
        ]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let (cas, token) = client.list_certificate_authorities(None, None).await.unwrap();

        assert_eq!(cas.len(), 1);
        assert_eq!(
            cas[0].inner.arn(),
            Some("arn:aws:acm-pca:us-east-1:1:certificate-authority/abc")
        );
        assert_eq!(cas[0].tags, vec![("env".to_string(), "prod".to_string())]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificate_authorities_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"CertificateAuthorities":[]}"#),
        )]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let (cas, token) = client
            .list_certificate_authorities(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(cas.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificate_authorities_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":2}"#),
                json_response(
                    200,
                    r#"{"CertificateAuthorities":[{"Arn":"arn-1"},{"Arn":"arn-2"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-1"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-2"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
        ]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let (cas, token) = client.list_certificate_authorities(Some(2), None).await.unwrap();

        assert_eq!(cas.len(), 2);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificate_authorities_pages_through_until_exhausted_when_limit_not_reached() {
        // Note the call order: the aws-layer loop above drains every AWS
        // page first (accumulating into `cas`), THEN fans out the N+1
        // `fetch_tags` calls over the whole accumulated set — the two
        // list-call events come before either tag-fetch event, not
        // interleaved per page.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"CertificateAuthorities":[{"Arn":"arn-1"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":9}"#),
                json_response(200, r#"{"CertificateAuthorities":[{"Arn":"arn-2"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-1"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-2"}"#),
                json_response(200, r#"{"Tags":[]}"#),
            ),
        ]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let (cas, token) = client.list_certificate_authorities(Some(10), None).await.unwrap();

        assert_eq!(cas.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_certificate_authorities_skips_tag_fetch_when_arn_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(200, r#"{"CertificateAuthorities":[{"Status":"ACTIVE"}]}"#),
        )]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let (cas, token) = client.list_certificate_authorities(None, None).await.unwrap();

        assert_eq!(cas.len(), 1);
        assert_eq!(cas[0].inner.arn(), None);
        assert!(cas[0].tags.is_empty());
        assert_eq!(token, None);
        // Only the list call should have been sent — no ListTags fan-out for
        // a CA with an empty/missing arn (relaxed_requests_match would panic
        // on an unconsumed extra event if fetch_tags fired anyway).
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_certificate_authority_returns_detail_when_found() {
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
                json_response(200, r#"{"Tags":[{"Key":"team","Value":"platform"}]}"#),
            ),
        ]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let ca = client.describe_certificate_authority("arn-1").await.unwrap().unwrap();

        assert_eq!(ca.inner.arn(), Some("arn-1"));
        assert_eq!(ca.tags, vec![("team".to_string(), "platform".to_string())]);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_certificate_authority_returns_none_when_certificate_authority_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-missing"}"#),
            json_response(200, r#"{}"#),
        )]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let ca = client.describe_certificate_authority("arn-missing").await.unwrap();

        assert!(ca.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_certificate_authority_propagates_not_found_error() {
        // Unlike acm::describe_certificate, this method has no special-case
        // for ResourceNotFoundException — it maps every send() error
        // (including not-found) straight through `sdk_err`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"CertificateAuthorityArn":"arn-missing"}"#),
            json_error_response("ResourceNotFoundException", "CA not found"),
        )]);
        let client = AcmPcaClient::new(&sdk_config(http_client.clone()));

        let result = client.describe_certificate_authority("arn-missing").await;

        match result {
            Ok(_) => panic!("expected an error, got Ok"),
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "CA not found");
            }
            Err(other) => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

