use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::wafv2::WafV2Client;
use crate::schema::wafv2::types::{WafIpSet, WafRuleGroup, WafScope, WebAcl};

#[derive(Default)]
pub struct Wafv2Query;

#[Object]
impl Wafv2Query {
    async fn waf_web_acls(&self, ctx: &Context<'_>, scope: WafScope) -> Result<Vec<WebAcl>> {
        let client = ctx.data::<WafV2Client>()?;
        let sdk_scope = scope.to_sdk();
        let summaries = client.list_web_acls(sdk_scope.clone()).await?;

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
        let mut out = Vec::new();
        for result in results {
            if let Ok(output) = result {
                if let Some(acl) = output.web_acl() {
                    out.push(WebAcl::from_sdk(acl, &scope));
                }
            }
        }
        Ok(out)
    }

    async fn waf_ip_sets(&self, ctx: &Context<'_>, scope: WafScope) -> Result<Vec<WafIpSet>> {
        let client = ctx.data::<WafV2Client>()?;
        let sdk_scope = scope.to_sdk();
        let summaries = client.list_ip_sets(sdk_scope.clone()).await?;

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
        let mut out = Vec::new();
        for result in results {
            if let Ok(output) = result {
                if let Some(ip_set) = output.ip_set() {
                    out.push(WafIpSet::from_sdk(ip_set, &scope));
                }
            }
        }
        Ok(out)
    }

    async fn waf_rule_groups(
        &self,
        ctx: &Context<'_>,
        scope: WafScope,
    ) -> Result<Vec<WafRuleGroup>> {
        let client = ctx.data::<WafV2Client>()?;
        let sdk_scope = scope.to_sdk();
        let summaries = client.list_rule_groups(sdk_scope).await?;
        Ok(summaries
            .iter()
            .map(|s| WafRuleGroup::from_summary(s, &scope))
            .collect())
    }
}
