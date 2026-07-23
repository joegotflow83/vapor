use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::network_firewall::NetworkFirewallClient;
use crate::schema::network_firewall::types::{Firewall, FirewallPolicy, RuleGroup};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct NetworkFirewallQuery;

#[Object]
impl NetworkFirewallQuery {
    /// Lists firewalls, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn network_firewalls(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Firewall>> {
        let client = ctx.data::<NetworkFirewallClient>()?;
        let (metadata_list, token) = client.list_firewalls(limit, next_token).await?;
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
        Ok(Page {
            items: firewalls,
            next_token: token,
        })
    }

    /// Lists firewall policies, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn network_firewall_policies(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<FirewallPolicy>> {
        let client = ctx.data::<NetworkFirewallClient>()?;
        let (metadata_list, token) = client.list_firewall_policies(limit, next_token).await?;
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
        Ok(Page {
            items: policies,
            next_token: token,
        })
    }

    /// Lists rule groups, optionally filtered by `rule_group_type` and capped
    /// at `limit` results (default unlimited), resumed from `next_token`.
    async fn network_firewall_rule_groups(
        &self,
        ctx: &Context<'_>,
        rule_group_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RuleGroup>> {
        let client = ctx.data::<NetworkFirewallClient>()?;
        let (metadata_list, token) = client
            .list_rule_groups(rule_group_type.as_deref(), limit, next_token)
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
        Ok(Page {
            items: rule_groups,
            next_token: token,
        })
    }
}

// All three resolvers share the same shape: a discovery `list_*` call
// followed by a per-arn `join_all` fan-out to `describe_*`, propagating any
// describe error via a custom "Failed to describe ..." message (not
// swallowed, unlike lambda/cloudfront's `unwrap_or_default` pattern) — real
// logic worth bespoke tests rather than a light passthrough smoke test.
#[cfg(test)]
mod tests {
    use crate::aws::network_firewall::NetworkFirewallClient;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::NetworkFirewallQuery;

    const ENDPOINT: &str = "https://network-firewall.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn network_firewalls_lists_and_describes_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Firewalls":[{"FirewallName":"fw1","FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw1"}],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw1"}"#,
                ),
                json_response(
                    200,
                    r#"{"UpdateToken":"token1","Firewall":{"FirewallName":"fw1","FirewallArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall/fw1","FirewallPolicyArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1","VpcId":"vpc-1","SubnetMappings":[]},"FirewallStatus":{"Status":"READY","ConfigurationSyncStateSummary":"IN_SYNC"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(NetworkFirewallQuery)
            .data(NetworkFirewallClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ networkFirewalls(limit: 1) { items { firewallName firewallArn firewallPolicyArn vpcId firewallStatus } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["networkFirewalls"]["items"];
        assert_eq!(items[0]["firewallName"], "fw1");
        assert_eq!(
            items[0]["firewallPolicyArn"],
            "arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"
        );
        assert_eq!(items[0]["vpcId"], "vpc-1");
        assert_eq!(items[0]["firewallStatus"], "READY");
        assert_eq!(json["networkFirewalls"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn network_firewalls_surfaces_describe_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(
                    200,
                    r#"{"Firewalls":[{"FirewallName":"fw1","FirewallArn":"arn:1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"FirewallArn":"arn:1"}"#),
                json_error_response("ResourceNotFoundException", "firewall not found"),
            ),
        ]);
        let schema = build_query_schema(NetworkFirewallQuery)
            .data(NetworkFirewallClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ networkFirewalls { items { firewallName } } }"#)
            .await;

        assert!(!res.errors.is_empty(), "expected an error, got none");
        assert!(
            res.errors[0].message.contains("Failed to describe firewall"),
            "unexpected error message: {}",
            res.errors[0].message
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn network_firewall_policies_lists_and_describes_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"FirewallPolicies":[{"Name":"pol1","Arn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"}],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"FirewallPolicyArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"}"#,
                ),
                json_response(
                    200,
                    r#"{"UpdateToken":"token1","FirewallPolicyResponse":{"FirewallPolicyName":"pol1","FirewallPolicyArn":"arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(NetworkFirewallQuery)
            .data(NetworkFirewallClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ networkFirewallPolicies(limit: 1) { items { name arn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["networkFirewallPolicies"]["items"];
        assert_eq!(items[0]["name"], "pol1");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:network-firewall:us-east-1:111111111111:firewall-policy/pol1"
        );
        assert_eq!(json["networkFirewallPolicies"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn network_firewall_policies_surfaces_describe_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(200, r#"{"FirewallPolicies":[{"Name":"pol1","Arn":"arn:1"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"FirewallPolicyArn":"arn:1"}"#),
                json_error_response("ResourceNotFoundException", "policy not found"),
            ),
        ]);
        let schema = build_query_schema(NetworkFirewallQuery)
            .data(NetworkFirewallClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ networkFirewallPolicies { items { name } } }"#)
            .await;

        assert!(!res.errors.is_empty(), "expected an error, got none");
        assert!(
            res.errors[0]
                .message
                .contains("Failed to describe firewall policy"),
            "unexpected error message: {}",
            res.errors[0].message
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn network_firewall_rule_groups_forwards_type_filter_and_lists_and_describes() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"Type":"STATEFUL","MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"RuleGroups":[{"Name":"rg1","Arn":"arn:aws:network-firewall:us-east-1:111111111111:stateful-rulegroup/rg1"}],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"RuleGroupArn":"arn:aws:network-firewall:us-east-1:111111111111:stateful-rulegroup/rg1"}"#,
                ),
                json_response(
                    200,
                    r#"{"UpdateToken":"token1","RuleGroupResponse":{"RuleGroupName":"rg1","RuleGroupArn":"arn:aws:network-firewall:us-east-1:111111111111:stateful-rulegroup/rg1","Type":"STATEFUL","Capacity":100}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(NetworkFirewallQuery)
            .data(NetworkFirewallClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ networkFirewallRuleGroups(ruleGroupType: "STATEFUL", limit: 1) { items { name arn ruleGroupType capacity } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["networkFirewallRuleGroups"]["items"];
        assert_eq!(items[0]["name"], "rg1");
        assert_eq!(items[0]["ruleGroupType"], "STATEFUL");
        assert_eq!(items[0]["capacity"], 100);
        assert_eq!(json["networkFirewallRuleGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn network_firewall_rule_groups_surfaces_describe_error() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{}"#),
                json_response(200, r#"{"RuleGroups":[{"Name":"rg1","Arn":"arn:1"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"RuleGroupArn":"arn:1"}"#),
                json_error_response("ResourceNotFoundException", "rule group not found"),
            ),
        ]);
        let schema = build_query_schema(NetworkFirewallQuery)
            .data(NetworkFirewallClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ networkFirewallRuleGroups { items { name } } }"#)
            .await;

        assert!(!res.errors.is_empty(), "expected an error, got none");
        assert!(
            res.errors[0]
                .message
                .contains("Failed to describe rule group"),
            "unexpected error message: {}",
            res.errors[0].message
        );
        http_client.relaxed_requests_match();
    }
}
