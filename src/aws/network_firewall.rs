use aws_config::SdkConfig;
use aws_sdk_networkfirewall::types::RuleGroupType;

use crate::error::VaporError;

pub struct NetworkFirewallClient {
    inner: aws_sdk_networkfirewall::Client,
}

impl NetworkFirewallClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_networkfirewall::Client::new(config),
        }
    }

    /// Lists firewalls, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListFirewalls` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-networkfirewall` 1.114.0), so `limit` is capped to the
    /// remaining budget on the request itself, matching kinesis/translate's
    /// server-side-capping pattern.
    pub async fn list_firewalls(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_networkfirewall::types::FirewallMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_firewalls();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            items.extend(output.firewalls.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn describe_firewall(
        &self,
        arn: &str,
    ) -> Result<
        aws_sdk_networkfirewall::operation::describe_firewall::DescribeFirewallOutput,
        VaporError,
    > {
        self.inner
            .describe_firewall()
            .firewall_arn(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Lists firewall policies, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListFirewallPolicies` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-networkfirewall` 1.114.0), same server-side-capping shape as
    /// `list_firewalls`.
    pub async fn list_firewall_policies(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_networkfirewall::types::FirewallPolicyMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_firewall_policies();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            items.extend(output.firewall_policies.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn describe_firewall_policy(
        &self,
        arn: &str,
    ) -> Result<
        aws_sdk_networkfirewall::operation::describe_firewall_policy::DescribeFirewallPolicyOutput,
        VaporError,
    > {
        self.inner
            .describe_firewall_policy()
            .firewall_policy_arn(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }

    /// Lists rule groups, optionally filtered by `rule_group_type` and capped
    /// at `limit` results (default unlimited), resumed from `next_token`.
    /// `ListRuleGroups` has both `max_results` and `next_token` (verified
    /// against pinned `aws-sdk-networkfirewall` 1.114.0), same
    /// server-side-capping shape as `list_firewalls`. The `type` filter is
    /// rebuilt on the request each loop iteration (fsx/transcribe/ses
    /// precedent).
    pub async fn list_rule_groups(
        &self,
        rule_group_type: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_networkfirewall::types::RuleGroupMetadata>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_rule_groups();
            if let Some(rgt) = rule_group_type {
                req = req.r#type(RuleGroupType::from(rgt));
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            token = output.next_token;
            items.extend(output.rule_groups.unwrap_or_default());

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn describe_rule_group(
        &self,
        arn: &str,
    ) -> Result<
        aws_sdk_networkfirewall::operation::describe_rule_group::DescribeRuleGroupOutput,
        VaporError,
    > {
        self.inner
            .describe_rule_group()
            .rule_group_arn(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://network-firewall.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_firewalls_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"Firewalls":[{"FirewallName":"fw1","FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw1"},{"FirewallName":"fw2","FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw2"}]}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_firewalls(None, None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].firewall_name(), Some("fw1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_firewalls_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(
                200,
                r#"{"Firewalls":[{"FirewallName":"fw3","FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw3"}]}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_firewalls(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_firewalls_stops_at_limit_and_returns_resume_token() {
        // ListFirewalls forwards `MaxResults` straight to AWS with no
        // client-side truncation, so the canned response must return
        // exactly the requested count, not more (durable gotcha 13).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Firewalls":[{"FirewallName":"fw1","FirewallArn":"arn:1"},{"FirewallName":"fw2","FirewallArn":"arn:2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_firewalls(Some(2), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_firewalls_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Firewalls":[{"FirewallName":"fw1","FirewallArn":"arn:1"},{"FirewallName":"fw2","FirewallArn":"arn:2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(
                    200,
                    r#"{"Firewalls":[{"FirewallName":"fw3","FirewallArn":"arn:3"}]}"#,
                ),
            ),
        ]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_firewalls(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_firewalls_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let err = client.list_firewalls(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_firewall_returns_details() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw1"}"#,
            ),
            json_response(
                200,
                r#"{"UpdateToken":"token1","Firewall":{"FirewallName":"fw1","FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw1","FirewallPolicyArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1","VpcId":"vpc-1","SubnetMappings":[]},"FirewallStatus":{"Status":"READY","ConfigurationSyncStateSummary":"IN_SYNC"}}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let output = client
            .describe_firewall("arn:aws:network-firewall:us-east-1:111111111111:firewall/fw1")
            .await
            .unwrap();

        assert_eq!(output.update_token(), Some("token1"));
        let firewall = output.firewall().unwrap();
        assert_eq!(firewall.firewall_name(), Some("fw1"));
        assert_eq!(
            firewall.firewall_policy_arn(),
            "arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"
        );
        assert_eq!(firewall.vpc_id(), "vpc-1");
        assert_eq!(
            output.firewall_status().unwrap().status(),
            &aws_sdk_networkfirewall::types::FirewallStatusValue::Ready
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_firewall_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"FirewallArn":"arn:missing"}"#),
            json_error_response("ResourceNotFoundException", "firewall not found"),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_firewall("arn:missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "firewall not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_firewall_policies_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"FirewallPolicies":[{"Name":"pol1","Arn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"}]}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_firewall_policies(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), Some("pol1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_firewall_policies_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"FirewallPolicies":[{"Name":"pol1","Arn":"arn:1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_firewall_policies(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_firewall_policies_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let err = client.list_firewall_policies(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_firewall_policy_returns_details() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"FirewallPolicyArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"}"#,
            ),
            json_response(
                200,
                r#"{"UpdateToken":"token1","FirewallPolicyResponse":{"FirewallPolicyName":"pol1","FirewallPolicyArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"}}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let output = client
            .describe_firewall_policy(
                "arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1",
            )
            .await
            .unwrap();

        assert_eq!(output.update_token(), "token1");
        let response = output.firewall_policy_response().unwrap();
        assert_eq!(response.firewall_policy_name(), "pol1");
        assert_eq!(
            response.firewall_policy_arn(),
            "arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_firewall_policy_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"FirewallPolicyArn":"arn:missing"}"#),
            json_error_response("ResourceNotFoundException", "policy not found"),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_firewall_policy("arn:missing")
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "policy not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"RuleGroups":[{"Name":"rg1","Arn":"arn:aws:network-firewall:us-east-1:111111111111:stateful-rulegroup/rg1"}]}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_rule_groups(None, None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), Some("rg1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_forwards_rule_group_type_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Type":"STATEFUL"}"#),
            json_response(200, r#"{"RuleGroups":[{"Name":"rg1","Arn":"arn:1"}]}"#),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, _token) = client
            .list_rule_groups(Some("STATEFUL"), None, None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"RuleGroups":[{"Name":"rg1","Arn":"arn:1"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":9}"#),
                json_response(200, r#"{"RuleGroups":[{"Name":"rg2","Arn":"arn:2"}]}"#),
            ),
        ]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_rule_groups(None, Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_rule_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "bad request"),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let err = client.list_rule_groups(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_rule_group_returns_details() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"RuleGroupArn":"arn:aws:network-firewall:us-east-1:111111111111:stateful-rulegroup/rg1"}"#,
            ),
            json_response(
                200,
                r#"{"UpdateToken":"token1","RuleGroupResponse":{"RuleGroupName":"rg1","RuleGroupArn":"arn:aws:network-firewall:us-east-1:111111111111:stateful-rulegroup/rg1","Type":"STATEFUL"}}"#,
            ),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let output = client
            .describe_rule_group(
                "arn:aws:network-firewall:us-east-1:111111111111:stateful-rulegroup/rg1",
            )
            .await
            .unwrap();

        assert_eq!(output.update_token(), "token1");
        let response = output.rule_group_response().unwrap();
        assert_eq!(response.rule_group_name(), "rg1");
        assert_eq!(response.r#type(), Some(&RuleGroupType::Stateful));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_rule_group_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"RuleGroupArn":"arn:missing"}"#),
            json_error_response("ResourceNotFoundException", "rule group not found"),
        )]);
        let client = NetworkFirewallClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_rule_group("arn:missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "rule group not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
