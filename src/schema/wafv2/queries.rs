use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::wafv2::WafV2Client;
use crate::schema::pagination::Page;
use crate::schema::wafv2::types::{WafIpSet, WafRuleGroup, WafScope, WebAcl};

#[derive(Default)]
pub struct Wafv2Query;

#[Object]
impl Wafv2Query {
    /// Lists Web ACLs. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn waf_web_acls(
        &self,
        ctx: &Context<'_>,
        scope: WafScope,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<WebAcl>> {
        let client = ctx.data::<WafV2Client>()?;
        let sdk_scope = scope.to_sdk();
        let (summaries, next_token) = client.list_web_acls(sdk_scope.clone(), limit, next_token).await?;

        let futures: Vec<_> = summaries
            .iter()
            .map(|s| {
                let name = s.name().unwrap_or_default().to_string();
                let id = s.id().unwrap_or_default().to_string();
                let sc = sdk_scope.clone();
                async move { client.get_web_acl(&name, &id, sc).await }
            })
            .collect();

        let results = join_all(futures).await;
        let mut items = Vec::new();
        for result in results {
            if let Ok(output) = result {
                if let Some(acl) = output.web_acl() {
                    items.push(WebAcl::from_sdk(acl, &scope));
                }
            }
        }
        Ok(Page { items, next_token })
    }

    /// Lists IP sets. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn waf_ip_sets(
        &self,
        ctx: &Context<'_>,
        scope: WafScope,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<WafIpSet>> {
        let client = ctx.data::<WafV2Client>()?;
        let sdk_scope = scope.to_sdk();
        let (summaries, next_token) = client.list_ip_sets(sdk_scope.clone(), limit, next_token).await?;

        let futures: Vec<_> = summaries
            .iter()
            .map(|s| {
                let name = s.name().unwrap_or_default().to_string();
                let id = s.id().unwrap_or_default().to_string();
                let sc = sdk_scope.clone();
                async move { client.get_ip_set(&name, &id, sc).await }
            })
            .collect();

        let results = join_all(futures).await;
        let mut items = Vec::new();
        for result in results {
            if let Ok(output) = result {
                if let Some(ip_set) = output.ip_set() {
                    items.push(WafIpSet::from_sdk(ip_set, &scope));
                }
            }
        }
        Ok(Page { items, next_token })
    }

    /// Lists rule groups. `limit` caps the total number of results (default
    /// unlimited); pass `nextToken` from a prior page to resume.
    async fn waf_rule_groups(
        &self,
        ctx: &Context<'_>,
        scope: WafScope,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<WafRuleGroup>> {
        let client = ctx.data::<WafV2Client>()?;
        let sdk_scope = scope.to_sdk();
        let (summaries, next_token) = client.list_rule_groups(sdk_scope, limit, next_token).await?;
        let items = summaries
            .iter()
            .map(|s| WafRuleGroup::from_summary(s, &scope))
            .collect();
        Ok(Page { items, next_token })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::aws::wafv2::WafV2Client;
    use crate::schema::test_util::build_query_schema;

    use super::Wafv2Query;

    const BASE: &str = "https://wafv2.us-east-1.amazonaws.com/";

    // --- waf_web_acls (list + per-item get_web_acl fan-out) ---

    #[tokio::test]
    async fn waf_web_acls_fans_out_and_maps_details() {
        // Single ACL, so `join_all`'s fan-out resolves in the same declared
        // order under `StaticReplayClient`'s synchronous connector (kms
        // `kms_keys` precedent) — sidesteps multi-item ordering ambiguity.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL"}"#),
                json_response(200, r#"{"WebACLs":[{"Name":"acl1","Id":"id1","ARN":"arn:acl1"}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"acl1","Scope":"REGIONAL","Id":"id1"}"#),
                json_response(
                    200,
                    r#"{"WebACL":{"Name":"acl1","Id":"id1","ARN":"arn:acl1","Description":"desc1","Capacity":100,"DefaultAction":{"Allow":{}},"ManagedByFirewallManager":true},"LockToken":"lock1"}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(Wafv2Query)
            .data(WafV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ wafWebAcls(scope: REGIONAL) { items { name id arn scope description \
                 capacity defaultAction rulesCount managedByFirewallManager } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["wafWebAcls"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["name"], "acl1");
        assert_eq!(items[0]["id"], "id1");
        assert_eq!(items[0]["arn"], "arn:acl1");
        assert_eq!(items[0]["scope"], "REGIONAL");
        assert_eq!(items[0]["description"], "desc1");
        assert_eq!(items[0]["capacity"], 100);
        // `DefaultAction` is a required-but-nullable SDK field: smithy-rs's
        // `web_acl_correct_errors` fills a missing key with an empty
        // `DefaultAction{}` (both `allow`/`block` unset) rather than leaving
        // it `None`, which `WebAcl::from_sdk` would then map to "BLOCK" —
        // so the fixture sets `Allow` explicitly to assert the real branch.
        assert_eq!(items[0]["defaultAction"], "ALLOW");
        assert_eq!(items[0]["rulesCount"], 0);
        assert_eq!(items[0]["managedByFirewallManager"], true);
        assert!(json["wafWebAcls"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn waf_web_acls_skips_items_when_get_details_fails() {
        // The resolver silently drops items whose `get_web_acl` call errors
        // (`if let Ok(output) = result`) rather than failing the whole page.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"CLOUDFRONT"}"#),
                json_response(200, r#"{"WebACLs":[{"Name":"acl1","Id":"id1","ARN":"arn:acl1"}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"acl1","Scope":"CLOUDFRONT","Id":"id1"}"#),
                crate::aws::test_util::json_error_response(
                    "WAFNonexistentItemException",
                    "web acl not found",
                ),
            ),
        ]);
        let schema = build_query_schema(Wafv2Query)
            .data(WafV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ wafWebAcls(scope: CLOUDFRONT) { items { name } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["wafWebAcls"]["items"].as_array().unwrap().len(), 0);
        http_client.relaxed_requests_match();
    }

    // --- waf_ip_sets (list + per-item get_ip_set fan-out) ---

    #[tokio::test]
    async fn waf_ip_sets_fans_out_and_maps_details() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"Scope":"REGIONAL"}"#),
                json_response(
                    200,
                    r#"{"IPSets":[{"Name":"ipset1","Id":"id1","ARN":"arn:ipset1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"ipset1","Scope":"REGIONAL","Id":"id1"}"#),
                json_response(
                    200,
                    r#"{"IPSet":{"Name":"ipset1","Id":"id1","ARN":"arn:ipset1","IPAddressVersion":"IPV4","Addresses":["10.0.0.0/8"]},"LockToken":"lock1"}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(Wafv2Query)
            .data(WafV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ wafIpSets(scope: REGIONAL) { items { name id arn scope description \
                 ipAddressVersion addresses } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["wafIpSets"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["name"], "ipset1");
        assert_eq!(items[0]["arn"], "arn:ipset1");
        assert_eq!(items[0]["scope"], "REGIONAL");
        assert_eq!(items[0]["ipAddressVersion"], "IPV4");
        assert_eq!(items[0]["addresses"][0], "10.0.0.0/8");
        http_client.relaxed_requests_match();
    }

    // --- waf_rule_groups (bare passthrough) ---

    #[tokio::test]
    async fn waf_rule_groups_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Scope":"REGIONAL","Limit":1}"#),
            json_response(
                200,
                r#"{"RuleGroups":[{"Name":"rg1","Id":"id1","ARN":"arn:rg1","Description":"desc1"}],"NextMarker":"page2-token"}"#,
            ),
        )]);
        let schema = build_query_schema(Wafv2Query)
            .data(WafV2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ wafRuleGroups(scope: REGIONAL, limit: 1) { items { name id arn scope \
                 description capacity } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["wafRuleGroups"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["name"], "rg1");
        assert_eq!(items[0]["arn"], "arn:rg1");
        assert_eq!(items[0]["scope"], "REGIONAL");
        assert_eq!(items[0]["description"], "desc1");
        assert!(items[0]["capacity"].is_null());
        assert_eq!(json["wafRuleGroups"]["nextToken"], "page2-token");
        http_client.relaxed_requests_match();
    }
}
