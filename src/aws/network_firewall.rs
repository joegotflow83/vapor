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

    pub async fn list_firewalls(
        &self,
    ) -> Result<Vec<aws_sdk_networkfirewall::types::FirewallMetadata>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_firewalls();
            if let Some(ref token) = next_token {
                req = req.next_token(token.clone());
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.firewalls().iter().cloned());
            next_token = output.next_token().map(|t| t.to_string());
            if next_token.is_none() {
                break;
            }
        }
        Ok(items)
    }

    pub async fn describe_firewall(
        &self,
        arn: &str,
    ) -> Result<aws_sdk_networkfirewall::operation::describe_firewall::DescribeFirewallOutput, VaporError>
    {
        self.inner
            .describe_firewall()
            .firewall_arn(arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_firewall_policies(
        &self,
    ) -> Result<Vec<aws_sdk_networkfirewall::types::FirewallPolicyMetadata>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_firewall_policies();
            if let Some(ref token) = next_token {
                req = req.next_token(token.clone());
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.firewall_policies().iter().cloned());
            next_token = output.next_token().map(|t| t.to_string());
            if next_token.is_none() {
                break;
            }
        }
        Ok(items)
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_rule_groups(
        &self,
        rule_group_type: Option<&str>,
    ) -> Result<Vec<aws_sdk_networkfirewall::types::RuleGroupMetadata>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_rule_groups();
            if let Some(rgt) = rule_group_type {
                req = req.r#type(RuleGroupType::from(rgt));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token.clone());
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.rule_groups().iter().cloned());
            next_token = output.next_token().map(|t| t.to_string());
            if next_token.is_none() {
                break;
            }
        }
        Ok(items)
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }
}
