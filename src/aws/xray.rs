use aws_config::SdkConfig;

use crate::error::VaporError;
use crate::aws::pagination::apply_limit;

#[derive(Debug, Clone, PartialEq)]
pub struct XRayGroupInsightsConfig {
    pub insights_enabled: Option<bool>,
    pub notifications_enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XRayGroupInfo {
    pub group_name: Option<String>,
    pub group_arn: Option<String>,
    pub filter_expression: Option<String>,
    pub insights_configuration: Option<XRayGroupInsightsConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XRaySamplingRuleInfo {
    pub rule_name: Option<String>,
    pub rule_arn: Option<String>,
    pub priority: Option<i32>,
    pub fixed_rate: Option<f64>,
    pub reservoir_size: Option<i32>,
    pub service_name: Option<String>,
    pub service_type: Option<String>,
    pub host: Option<String>,
    pub http_method: Option<String>,
    pub url_path: Option<String>,
    pub resource_arn: Option<String>,
    pub version: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XRayEncryptionConfigInfo {
    pub key_id: Option<String>,
    pub status: Option<String>,
    pub type_: Option<String>,
}

pub struct XRayClient {
    inner: aws_sdk_xray::Client,
}

impl XRayClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_xray::Client::new(config),
        }
    }

    /// Lists X-Ray groups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `GetGroups` has no
    /// `max_results`-equivalent input field (only a bare `next_token`,
    /// confirmed against pinned `aws-sdk-xray` 1.103.0's
    /// `operation/get_groups/_get_groups_input.rs`), so `limit` can only be
    /// enforced via client-side `apply_limit` truncation — same caveat class
    /// as `lightsail.rs::get_instances`: when truncation trips mid-page, the
    /// returned `next_token` is still AWS's *next*-page token, permanently
    /// skipping whatever was truncated off the current page.
    pub async fn get_groups(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<XRayGroupInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_groups();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for g in output.groups.unwrap_or_default() {
                items.push(XRayGroupInfo {
                    group_name: g.group_name,
                    group_arn: g.group_arn,
                    filter_expression: g.filter_expression,
                    insights_configuration: g.insights_configuration.map(|ic| {
                        XRayGroupInsightsConfig {
                            insights_enabled: ic.insights_enabled,
                            notifications_enabled: ic.notifications_enabled,
                        }
                    }),
                });
            }

            token = match output.next_token {
                Some(tok) if !tok.is_empty() => Some(tok),
                _ => None,
            };

            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    /// Lists X-Ray sampling rules, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `GetSamplingRules`
    /// has no `max_results`-equivalent input field (same caveat class as
    /// `get_groups` above).
    pub async fn list_sampling_rules(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<XRaySamplingRuleInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_sampling_rules();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            for record in output.sampling_rule_records.unwrap_or_default() {
                if let Some(rule) = record.sampling_rule {
                    items.push(XRaySamplingRuleInfo {
                        rule_name: rule.rule_name,
                        rule_arn: rule.rule_arn,
                        priority: Some(rule.priority),
                        fixed_rate: Some(rule.fixed_rate),
                        reservoir_size: Some(rule.reservoir_size),
                        service_name: Some(rule.service_name),
                        service_type: Some(rule.service_type),
                        host: Some(rule.host),
                        http_method: Some(rule.http_method),
                        url_path: Some(rule.url_path),
                        resource_arn: Some(rule.resource_arn),
                        version: Some(rule.version),
                    });
                }
            }

            token = match output.next_token {
                Some(tok) if !tok.is_empty() => Some(tok),
                _ => None,
            };

            if apply_limit(&mut items, limit) || token.is_none() {
                break;
            }
        }

        Ok((items, token))
    }

    pub async fn get_encryption_config(
        &self,
    ) -> Result<Option<XRayEncryptionConfigInfo>, VaporError> {
        let output = self
            .inner
            .get_encryption_config()
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output.encryption_config().map(|ec| XRayEncryptionConfigInfo {
            key_id: ec.key_id().map(|s| s.to_string()),
            status: ec.status().map(|s| s.as_str().to_string()),
            type_: ec.r#type().map(|t| t.as_str().to_string()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const BASE: &str = "https://xray.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn get_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/Groups"), "{}"),
            json_response(
                200,
                r#"{"Groups":[{"GroupName":"g1","GroupARN":"arn:g1","FilterExpression":"service(\"api\")","InsightsConfiguration":{"InsightsEnabled":true,"NotificationsEnabled":false}},{"GroupName":"g2"}]}"#,
            ),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_groups(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let g1 = &items[0];
        assert_eq!(g1.group_name, Some("g1".to_string()));
        assert_eq!(g1.group_arn, Some("arn:g1".to_string()));
        assert_eq!(g1.filter_expression, Some("service(\"api\")".to_string()));
        assert_eq!(
            g1.insights_configuration,
            Some(XRayGroupInsightsConfig {
                insights_enabled: Some(true),
                notifications_enabled: Some(false),
            })
        );

        let g2 = &items[1];
        assert_eq!(g2.group_name, Some("g2".to_string()));
        assert_eq!(g2.group_arn, None);
        assert_eq!(g2.filter_expression, None);
        assert_eq!(g2.insights_configuration, None);

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_groups_maps_minimal_group_with_no_optional_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/Groups"), "{}"),
            json_response(200, r#"{"Groups":[{}]}"#),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client.get_groups(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        let g = &items[0];
        assert_eq!(g.group_name, None);
        assert_eq!(g.group_arn, None);
        assert_eq!(g.filter_expression, None);
        assert_eq!(g.insights_configuration, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_groups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/Groups"), r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Groups":[{"GroupName":"g3"}]}"#),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .get_groups(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].group_name, Some("g3".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_groups_stops_at_limit_and_returns_resume_token() {
        // `GetGroups` has no `MaxResults`-equivalent input field at all, so
        // `limit` is enforced purely client-side (durable gotcha 13's
        // client-truncate category) — the canned response must return more
        // than `limit` items to prove truncation actually happens.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/Groups"), "{}"),
            json_response(
                200,
                r#"{"Groups":[{"GroupName":"g1"},{"GroupName":"g2"},{"GroupName":"g3"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_groups(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].group_name, Some("g1".to_string()));
        assert_eq!(items[1].group_name, Some("g2".to_string()));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_groups_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/Groups"), "{}"),
                json_response(200, r#"{"Groups":[{"GroupName":"g1"}],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/Groups"), r#"{"NextToken":"p2"}"#),
                json_response(200, r#"{"Groups":[{"GroupName":"g2"}]}"#),
            ),
        ]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.get_groups(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].group_name, Some("g1".to_string()));
        assert_eq!(items[1].group_name, Some("g2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_groups_propagates_errors() {
        // `InvalidRequestException`, not a throttling-classified code (see
        // memory gotcha 1: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/Groups"), "{}"),
            json_error_response("InvalidRequestException", "malformed request"),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let err = client.get_groups(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "malformed request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_sampling_rules_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetSamplingRules"), "{}"),
            json_response(
                200,
                r#"{"SamplingRuleRecords":[{"SamplingRule":{"RuleName":"r1","RuleARN":"arn:r1","ResourceARN":"*","Priority":1,"FixedRate":0.05,"ReservoirSize":1,"ServiceName":"svc","ServiceType":"AWS::EC2::Instance","Host":"*","HTTPMethod":"GET","URLPath":"/*","Version":1},"CreatedAt":1700000000,"ModifiedAt":1700000001},{"SamplingRule":{"ResourceARN":"*","Priority":2,"FixedRate":0.1,"ReservoirSize":2,"ServiceName":"svc2","ServiceType":"type2","Host":"h2","HTTPMethod":"POST","URLPath":"/p","Version":2}}]}"#,
            ),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_sampling_rules(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        let r1 = &items[0];
        assert_eq!(r1.rule_name, Some("r1".to_string()));
        assert_eq!(r1.rule_arn, Some("arn:r1".to_string()));
        assert_eq!(r1.priority, Some(1));
        assert_eq!(r1.fixed_rate, Some(0.05));
        assert_eq!(r1.reservoir_size, Some(1));
        assert_eq!(r1.service_name, Some("svc".to_string()));
        assert_eq!(r1.service_type, Some("AWS::EC2::Instance".to_string()));
        assert_eq!(r1.host, Some("*".to_string()));
        assert_eq!(r1.http_method, Some("GET".to_string()));
        assert_eq!(r1.url_path, Some("/*".to_string()));
        assert_eq!(r1.resource_arn, Some("*".to_string()));
        assert_eq!(r1.version, Some(1));

        let r2 = &items[1];
        assert_eq!(r2.rule_name, None);
        assert_eq!(r2.rule_arn, None);
        assert_eq!(r2.service_name, Some("svc2".to_string()));

        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_sampling_rules_record_without_sampling_rule_is_skipped() {
        // `SamplingRuleRecord.sampling_rule` is itself optional — a record
        // missing that field is silently dropped by the `if let Some(rule)`
        // guard in `list_sampling_rules` rather than mapped to a
        // partially-filled `XRaySamplingRuleInfo`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetSamplingRules"), "{}"),
            json_response(200, r#"{"SamplingRuleRecords":[{"CreatedAt":1700000000}]}"#),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_sampling_rules(None, None).await.unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_sampling_rules_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetSamplingRules"), r#"{"NextToken":"cursor-a"}"#),
            json_response(
                200,
                r#"{"SamplingRuleRecords":[{"SamplingRule":{"ResourceARN":"*","Priority":3,"FixedRate":0.2,"ReservoirSize":3,"ServiceName":"svc3","ServiceType":"type3","Host":"h3","HTTPMethod":"PUT","URLPath":"/q","Version":3}}]}"#,
            ),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_sampling_rules(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].service_name, Some("svc3".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_sampling_rules_stops_at_limit_and_returns_resume_token() {
        // `GetSamplingRules` has no `MaxResults`-equivalent input field
        // either (same client-truncate caveat class as `get_groups`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetSamplingRules"), "{}"),
            json_response(
                200,
                r#"{"SamplingRuleRecords":[{"SamplingRule":{"ResourceARN":"*","Priority":1,"FixedRate":0.1,"ReservoirSize":1,"ServiceName":"svc1","ServiceType":"t","Host":"h","HTTPMethod":"GET","URLPath":"/","Version":1}},{"SamplingRule":{"ResourceARN":"*","Priority":2,"FixedRate":0.1,"ReservoirSize":1,"ServiceName":"svc2","ServiceType":"t","Host":"h","HTTPMethod":"GET","URLPath":"/","Version":1}},{"SamplingRule":{"ResourceARN":"*","Priority":3,"FixedRate":0.1,"ReservoirSize":1,"ServiceName":"svc3","ServiceType":"t","Host":"h","HTTPMethod":"GET","URLPath":"/","Version":1}}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_sampling_rules(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].service_name, Some("svc1".to_string()));
        assert_eq!(items[1].service_name, Some("svc2".to_string()));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_sampling_rules_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/GetSamplingRules"), "{}"),
                json_response(
                    200,
                    r#"{"SamplingRuleRecords":[{"SamplingRule":{"ResourceARN":"*","Priority":1,"FixedRate":0.1,"ReservoirSize":1,"ServiceName":"svc1","ServiceType":"t","Host":"h","HTTPMethod":"GET","URLPath":"/","Version":1}}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/GetSamplingRules"), r#"{"NextToken":"p2"}"#),
                json_response(
                    200,
                    r#"{"SamplingRuleRecords":[{"SamplingRule":{"ResourceARN":"*","Priority":2,"FixedRate":0.1,"ReservoirSize":1,"ServiceName":"svc2","ServiceType":"t","Host":"h","HTTPMethod":"GET","URLPath":"/","Version":1}}]}"#,
                ),
            ),
        ]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_sampling_rules(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].service_name, Some("svc1".to_string()));
        assert_eq!(items[1].service_name, Some("svc2".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_sampling_rules_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetSamplingRules"), "{}"),
            json_error_response("InvalidRequestException", "malformed request"),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let err = client.list_sampling_rules(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "malformed request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_encryption_config_returns_populated_config() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/EncryptionConfig"), ""),
            json_response(
                200,
                r#"{"EncryptionConfig":{"KeyId":"arn:kms:key1","Status":"ACTIVE","Type":"KMS"}}"#,
            ),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let config = client.get_encryption_config().await.unwrap();

        assert_eq!(
            config,
            Some(XRayEncryptionConfigInfo {
                key_id: Some("arn:kms:key1".to_string()),
                status: Some("ACTIVE".to_string()),
                type_: Some("KMS".to_string()),
            })
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_encryption_config_returns_none_when_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/EncryptionConfig"), ""),
            json_response(200, r#"{}"#),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let config = client.get_encryption_config().await.unwrap();

        assert_eq!(config, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_encryption_config_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/EncryptionConfig"), ""),
            json_error_response("InvalidRequestException", "malformed request"),
        )]);
        let client = XRayClient::new(&sdk_config(http_client.clone()));

        let err = client.get_encryption_config().await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "malformed request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
