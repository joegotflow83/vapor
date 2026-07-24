use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct ServiceQuotasClient {
    inner: aws_sdk_servicequotas::Client,
}

impl ServiceQuotasClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_servicequotas::Client::new(config),
        }
    }

    /// Lists service quotas for a service, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListServiceQuotas` has both `max_results` (i32) and `next_token`
    /// (verified against pinned `aws-sdk-servicequotas` 1.104.0's
    /// `operation/list_service_quotas/_list_service_quotas_input.rs`); the
    /// limit is handed to AWS as the exact remaining budget each page so the
    /// returned token always lands on a real page boundary (spec-2 pattern).
    pub async fn list_service_quotas(
        &self,
        service_code: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_servicequotas::types::ServiceQuota>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_service_quotas().service_code(service_code);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.quotas.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists AWS services with quotas, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListServices` has
    /// both `max_results` (i32) and `next_token` (verified against pinned
    /// `aws-sdk-servicequotas` 1.104.0's
    /// `operation/list_services/_list_services_input.rs`).
    pub async fn list_services(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_servicequotas::types::ServiceInfo>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_services();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.services.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // awsJson1.1: POST JSON to a fixed `/` path, differentiated only by the
    // `x-amz-target` header (which `test_util::request` doesn't compare) —
    // same shape as `ram.rs`/`secrets_manager.rs`/`sagemaker.rs`. Crate name
    // (`aws-sdk-servicequotas`) matches the endpoint hostname
    // (`servicequotas.*`, verified against pinned `aws-sdk-servicequotas`
    // 1.104.0's `config/endpoint.rs`). Request/response bodies use
    // PascalCase keys throughout (`ServiceCode`, `NextToken`, `MaxResults`,
    // `Quotas`, `Services`, ...) per each op's
    // `ser_*_input_input`/`de_*` codegen. Neither `ServiceQuota` nor
    // `ServiceInfo` is touched by any `*_correct_errors` fn (grepped
    // `serde_util.rs`, only an unrelated `tag_correct_errors` hit) — every
    // `Option<T>` field genuinely stays `None` on a missing key. Both ops'
    // aws-layer loop was fixed in this session to actually page (previously
    // a single `.send()` per call, contradicting
    // `specs/plan-2-schema-v2-pagination-timestamps.md`'s hand-rolled-loop
    // pattern) — the capped/pages-through tests below exercise that fix.
    const BASE: &str = "https://servicequotas.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_service_quotas_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServiceCode":"ec2"}"#),
            json_response(
                200,
                r#"{"Quotas":[{"QuotaCode":"L-1234","QuotaName":"Instances","Value":20.0}]}"#,
            ),
        )]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let (quotas, token) = client.list_service_quotas("ec2", None, None).await.unwrap();

        assert_eq!(quotas.len(), 1);
        assert_eq!(quotas[0].quota_code(), Some("L-1234"));
        assert_eq!(quotas[0].quota_name(), Some("Instances"));
        assert_eq!(quotas[0].value(), Some(20.0));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_service_quotas_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServiceCode":"ec2","NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Quotas":[]}"#),
        )]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let (quotas, token) = client
            .list_service_quotas("ec2", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(quotas.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_service_quotas_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServiceCode":"ec2","MaxResults":1}"#),
            json_response(
                200,
                r#"{"Quotas":[{"QuotaCode":"L-1234"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let (quotas, token) = client
            .list_service_quotas("ec2", Some(1), None)
            .await
            .unwrap();

        assert_eq!(quotas.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_service_quotas_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"ServiceCode":"ec2","MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Quotas":[{"QuotaCode":"L-1"},{"QuotaCode":"L-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    BASE,
                    r#"{"ServiceCode":"ec2","NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(200, r#"{"Quotas":[{"QuotaCode":"L-3"}]}"#),
            ),
        ]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let (quotas, token) = client
            .list_service_quotas("ec2", Some(10), None)
            .await
            .unwrap();

        assert_eq!(quotas.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_service_quotas_propagates_errors() {
        // `NoSuchResourceException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"ServiceCode":"bogus"}"#),
            json_error_response("NoSuchResourceException", "no such service"),
        )]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_service_quotas("bogus", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NoSuchResourceException".to_string()));
                assert_eq!(message, "no such service");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Services":[{"ServiceCode":"ec2","ServiceName":"Amazon EC2"}]}"#,
            ),
        )]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client.list_services(None, None).await.unwrap();

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].service_code(), Some("ec2"));
        assert_eq!(services[0].service_name(), Some("Amazon EC2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Services":[{"ServiceCode":"ec2"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client.list_services(Some(1), None).await.unwrap();

        assert_eq!(services.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Services":[{"ServiceCode":"ec2"},{"ServiceCode":"s3"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"Services":[{"ServiceCode":"iam"}]}"#),
            ),
        ]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let (services, token) = client.list_services(Some(10), None).await.unwrap();

        assert_eq!(services.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_services_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("IllegalArgumentException", "bad request"),
        )]);
        let client = ServiceQuotasClient::new(&sdk_config(http_client.clone()));

        let err = client.list_services(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("IllegalArgumentException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
