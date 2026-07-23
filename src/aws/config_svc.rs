use aws_config::SdkConfig;
use aws_sdk_config as config_sdk;

use crate::error::VaporError;
use crate::aws::pagination::apply_limit;

pub struct AwsConfigClient {
    inner: config_sdk::Client,
}

impl AwsConfigClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: config_sdk::Client::new(config),
        }
    }

    /// Lists AWS Config rules, optionally filtered by name, capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `DescribeConfigRules` (verified against pinned `aws-sdk-config`
    /// 1.112.0's `_describe_config_rules_input.rs`) has no `limit`/
    /// `max_results` request field at all, only `next_token` — so `limit`
    /// can only be enforced via client-side `apply_limit` truncation
    /// (efs.rs `describe_mount_targets` pattern): when that trips mid-page
    /// the returned `next_token` is still AWS's *next*-page token,
    /// permanently skipping whatever was truncated off the current page.
    pub async fn describe_config_rules(
        &self,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<config_sdk::types::ConfigRule>, Option<String>), VaporError> {
        let mut rules = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_config_rules();
            if let Some(ref names) = names {
                req = req.set_config_rule_names(Some(names.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            rules.extend(output.config_rules.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut rules, limit) || token.is_none() {
                break;
            }
        }

        Ok((rules, token))
    }

    /// Lists compliance status per Config rule, optionally filtered by rule
    /// name and compliance type, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeComplianceByConfigRule`
    /// (verified against pinned `aws-sdk-config` 1.112.0's
    /// `_describe_compliance_by_config_rule_input.rs`) has no `limit`/
    /// `max_results` request field either, only `next_token` — same
    /// client-side-truncation caveat as `describe_config_rules` above.
    pub async fn describe_compliance_by_config_rule(
        &self,
        rule_names: Option<Vec<String>>,
        compliance_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<config_sdk::types::ComplianceByConfigRule>, Option<String>), VaporError> {
        let ct: Option<Vec<config_sdk::types::ComplianceType>> = compliance_types.map(|types| {
            types
                .iter()
                .filter_map(|s| s.parse::<config_sdk::types::ComplianceType>().ok())
                .collect()
        });

        let mut results = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_compliance_by_config_rule();
            if let Some(ref names) = rule_names {
                req = req.set_config_rule_names(Some(names.clone()));
            }
            if let Some(ref types) = ct {
                req = req.set_compliance_types(Some(types.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.compliance_by_config_rules.unwrap_or_default());
            token = output.next_token;

            if apply_limit(&mut results, limit) || token.is_none() {
                break;
            }
        }

        Ok((results, token))
    }

    /// Lists compliance status per resource, optionally filtered by resource
    /// type and compliance type, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeComplianceByResource`
    /// has both `limit` and `next_token` request fields (verified against
    /// pinned `aws-sdk-config` 1.112.0's
    /// `_describe_compliance_by_resource_input.rs`), so `limit` is capped to
    /// the remaining budget on the request itself (acm/efs `describe_access_points`
    /// pattern).
    pub async fn describe_compliance_by_resource(
        &self,
        resource_type: Option<String>,
        compliance_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<config_sdk::types::ComplianceByResource>, Option<String>), VaporError> {
        let ct: Option<Vec<config_sdk::types::ComplianceType>> = compliance_types.map(|types| {
            types
                .iter()
                .filter_map(|s| s.parse::<config_sdk::types::ComplianceType>().ok())
                .collect()
        });

        let mut results = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_compliance_by_resource();
            if let Some(ref rt) = resource_type {
                req = req.resource_type(rt);
            }
            if let Some(ref types) = ct {
                req = req.set_compliance_types(Some(types.clone()));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - results.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.compliance_by_resources.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if results.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((results, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::error::VaporError;

    const ENDPOINT: &str = "https://config.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_config_rules_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ConfigRules":[{"ConfigRuleName":"rule1"},{"ConfigRuleName":"rule2"}]}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client.describe_config_rules(None, None, None).await.unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].config_rule_name(), Some("rule1"));
        assert_eq!(rules[1].config_rule_name(), Some("rule2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_config_rules_filters_by_names_and_resumes_from_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ConfigRuleNames":["ruleA"],"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"ConfigRules":[{"ConfigRuleName":"ruleA"}]}"#),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client
            .describe_config_rules(Some(vec!["ruleA".to_string()]), None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].config_rule_name(), Some("ruleA"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_config_rules_stops_at_limit_via_client_side_truncation() {
        // DescribeConfigRules has no `limit`/`max_results` request field, so a
        // single page is fetched and truncated locally; the returned token is
        // still AWS's next-page token even though item 2 was dropped.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ConfigRules":[{"ConfigRuleName":"r1"},{"ConfigRuleName":"r2"}],"NextToken":"p2"}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client.describe_config_rules(None, Some(1), None).await.unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].config_rule_name(), Some("r1"));
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_config_rules_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(
                    200,
                    r#"{"ConfigRules":[{"ConfigRuleName":"r1"},{"ConfigRuleName":"r2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2"}"#),
                json_response(200, r#"{"ConfigRules":[{"ConfigRuleName":"r3"}]}"#),
            ),
        ]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (rules, token) = client.describe_config_rules(None, Some(10), None).await.unwrap();

        assert_eq!(rules.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_config_rules_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidParameterValueException", "bad rule name"),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        match client.describe_config_rules(None, None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad rule name");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_config_rule_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ComplianceByConfigRules":[{"ConfigRuleName":"rule1","Compliance":{"ComplianceType":"COMPLIANT"}}]}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .describe_compliance_by_config_rule(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].config_rule_name(), Some("rule1"));
        assert_eq!(
            results[0].compliance().and_then(|c| c.compliance_type()),
            Some(&config_sdk::types::ComplianceType::Compliant)
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_config_rule_filters_by_names_and_compliance_types() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"ConfigRuleNames":["ruleA"],"ComplianceTypes":["NON_COMPLIANT"]}"#,
            ),
            json_response(
                200,
                r#"{"ComplianceByConfigRules":[{"ConfigRuleName":"ruleA","Compliance":{"ComplianceType":"NON_COMPLIANT"}}]}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (results, _token) = client
            .describe_compliance_by_config_rule(
                Some(vec!["ruleA".to_string()]),
                Some(vec!["NON_COMPLIANT".to_string()]),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].compliance().and_then(|c| c.compliance_type()),
            Some(&config_sdk::types::ComplianceType::NonCompliant)
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_config_rule_stops_at_limit_via_client_side_truncation() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ComplianceByConfigRules":[{"ConfigRuleName":"r1"},{"ConfigRuleName":"r2"}],"NextToken":"p2"}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .describe_compliance_by_config_rule(None, None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_config_rule_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidParameterValueException", "bad compliance type"),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        match client
            .describe_compliance_by_config_rule(None, None, None, None)
            .await
        {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad compliance type");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_resource_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"ComplianceByResources":[{"ResourceType":"AWS::EC2::Instance","ResourceId":"i-1","Compliance":{"ComplianceType":"COMPLIANT"}}]}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .describe_compliance_by_resource(None, None, None, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource_type(), Some("AWS::EC2::Instance"));
        assert_eq!(results[0].resource_id(), Some("i-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_resource_sends_limit_and_compliance_types_on_the_request() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"ResourceType":"AWS::EC2::Instance","ComplianceTypes":["NON_COMPLIANT"],"Limit":5}"#,
            ),
            json_response(
                200,
                r#"{"ComplianceByResources":[{"ResourceType":"AWS::EC2::Instance","ResourceId":"i-2"}]}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (results, _token) = client
            .describe_compliance_by_resource(
                Some("AWS::EC2::Instance".to_string()),
                Some(vec!["NON_COMPLIANT".to_string()]),
                Some(5),
                None,
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_resource_pages_reducing_limit_by_items_already_collected() {
        // `limit` is a request field here (unlike the other two methods), so
        // each subsequent page's `Limit` shrinks by what's already collected.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Limit":10}"#),
                json_response(
                    200,
                    r#"{"ComplianceByResources":[{"ResourceId":"i-1"},{"ResourceId":"i-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","Limit":8}"#),
                json_response(200, r#"{"ComplianceByResources":[{"ResourceId":"i-3"}]}"#),
            ),
        ]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .describe_compliance_by_resource(None, None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_resource_stops_once_limit_reached() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":2}"#),
            json_response(
                200,
                r#"{"ComplianceByResources":[{"ResourceId":"i-1"},{"ResourceId":"i-2"}],"NextToken":"p2"}"#,
            ),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        let (results, token) = client
            .describe_compliance_by_resource(None, None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_compliance_by_resource_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidParameterValueException", "bad resource type"),
        )]);
        let client = AwsConfigClient::new(&sdk_config(http_client.clone()));

        match client.describe_compliance_by_resource(None, None, None, None).await {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad resource type");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

