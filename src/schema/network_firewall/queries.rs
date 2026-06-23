use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::network_firewall::NetworkFirewallClient;
use crate::schema::network_firewall::types::{Firewall, FirewallPolicy, RuleGroup};

#[derive(Default)]
pub struct NetworkFirewallQuery;

#[Object]
impl NetworkFirewallQuery {
    async fn network_firewalls(&self, ctx: &Context<'_>) -> Result<Vec<Firewall>> {
        let client = ctx.data::<NetworkFirewallClient>()?;
        let metadata_list = client.list_firewalls().await?;
        let arns: Vec<String> = metadata_list
            .iter()
            .filter_map(|m| m.firewall_arn().map(|s| s.to_string()))
            .collect();
        let futures: Vec<_> = arns
            .into_iter()
            .map(|arn| async move { client.describe_firewall(&arn).await })
            .collect();
        let results = join_all(futures).await;
        let mut firewalls = Vec::new();
        for result in results {
            match result {
                Ok(output) => firewalls.push(Firewall::from(output)),
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to describe firewall: {e}"
                    )));
                }
            }
        }
        Ok(firewalls)
    }

    async fn network_firewall_policies(&self, ctx: &Context<'_>) -> Result<Vec<FirewallPolicy>> {
        let client = ctx.data::<NetworkFirewallClient>()?;
        let metadata_list = client.list_firewall_policies().await?;
        let arns: Vec<String> = metadata_list
            .iter()
            .filter_map(|m| m.arn().map(|s| s.to_string()))
            .collect();
        let futures: Vec<_> = arns
            .into_iter()
            .map(|arn| async move { client.describe_firewall_policy(&arn).await })
            .collect();
        let results = join_all(futures).await;
        let mut policies = Vec::new();
        for result in results {
            match result {
                Ok(output) => policies.push(FirewallPolicy::from(output)),
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to describe firewall policy: {e}"
                    )));
                }
            }
        }
        Ok(policies)
    }

    async fn network_firewall_rule_groups(
        &self,
        ctx: &Context<'_>,
        rule_group_type: Option<String>,
    ) -> Result<Vec<RuleGroup>> {
        let client = ctx.data::<NetworkFirewallClient>()?;
        let metadata_list = client
            .list_rule_groups(rule_group_type.as_deref())
            .await?;
        let arns: Vec<String> = metadata_list
            .iter()
            .filter_map(|m| m.arn().map(|s| s.to_string()))
            .collect();
        let futures: Vec<_> = arns
            .into_iter()
            .map(|arn| async move { client.describe_rule_group(&arn).await })
            .collect();
        let results = join_all(futures).await;
        let mut rule_groups = Vec::new();
        for result in results {
            match result {
                Ok(output) => rule_groups.push(RuleGroup::from(output)),
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to describe rule group: {e}"
                    )));
                }
            }
        }
        Ok(rule_groups)
    }
}
