use async_graphql::{Context, Object, Result};

use crate::aws::xray::XRayClient;
use crate::schema::pagination::Page;
use crate::schema::xray::types::{XRayEncryptionConfig, XRayGroup, XRaySamplingRule};

#[derive(Default)]
pub struct XRayQuery;

#[Object]
impl XRayQuery {
    /// Lists X-Ray groups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn xray_groups(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<XRayGroup>> {
        let client = ctx.data::<XRayClient>()?;
        let (items, next_token) = client.get_groups(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(XRayGroup::from).collect(),
            next_token,
        })
    }

    /// Lists X-Ray sampling rules, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn xray_sampling_rules(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<XRaySamplingRule>> {
        let client = ctx.data::<XRayClient>()?;
        let (items, next_token) = client.list_sampling_rules(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(XRaySamplingRule::from).collect(),
            next_token,
        })
    }

    async fn xray_encryption_config(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<XRayEncryptionConfig>> {
        let client = ctx.data::<XRayClient>()?;
        let config = client.get_encryption_config().await?;
        Ok(config.map(XRayEncryptionConfig::from))
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::aws::xray::XRayClient;
    use crate::schema::test_util::build_query_schema;

    use super::XRayQuery;

    const BASE: &str = "https://xray.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn xray_groups_maps_fields_and_next_token() {
        // `limit: 2` against a 3-item response, matching the `aws/xray.rs`
        // `get_groups_stops_at_limit_and_returns_resume_token` precedent —
        // `GetGroups` has no `MaxResults` input field, so `limit` is enforced
        // purely client-side and a `NextToken` in the canned response would
        // otherwise trigger an unbounded-fetch second request past the single
        // mocked `ReplayEvent` (gotcha: only pass `NextToken` back when
        // `limit` is set, or the query must actually be unbounded).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/Groups"), "{}"),
            json_response(
                200,
                r#"{"Groups":[{"GroupName":"g1","GroupARN":"arn:g1","FilterExpression":"service(\"api\")","InsightsConfiguration":{"InsightsEnabled":true,"NotificationsEnabled":false}},{"GroupName":"g2"},{"GroupName":"g3"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(XRayQuery)
            .data(XRayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ xrayGroups(limit: 2) { items { groupName groupArn filterExpression insightsConfiguration { insightsEnabled notificationsEnabled } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["xrayGroups"]["items"];
        assert_eq!(items[0]["groupName"], "g1");
        assert_eq!(items[0]["groupArn"], "arn:g1");
        assert_eq!(items[0]["filterExpression"], "service(\"api\")");
        assert_eq!(items[0]["insightsConfiguration"]["insightsEnabled"], true);
        assert_eq!(items[0]["insightsConfiguration"]["notificationsEnabled"], false);

        assert_eq!(items[1]["groupName"], "g2");
        assert_eq!(items[1]["groupArn"], serde_json::Value::Null);
        assert_eq!(items[1]["insightsConfiguration"], serde_json::Value::Null);

        assert_eq!(json["xrayGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn xray_groups_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/Groups"), r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"Groups":[{"GroupName":"g3"}]}"#),
        )]);
        let schema = build_query_schema(XRayQuery)
            .data(XRayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ xrayGroups(nextToken: "cursor-a") { items { groupName } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["xrayGroups"]["items"][0]["groupName"], "g3");
        assert_eq!(json["xrayGroups"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn xray_sampling_rules_maps_fields_and_next_token() {
        // `limit: 2` against a 3-record response — same client-truncate
        // reasoning as `xray_groups_maps_fields_and_next_token` above;
        // `GetSamplingRules` has no `MaxResults` input field either.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/GetSamplingRules"), "{}"),
            json_response(
                200,
                r#"{"SamplingRuleRecords":[{"SamplingRule":{"RuleName":"r1","RuleARN":"arn:r1","ResourceARN":"*","Priority":1,"FixedRate":0.05,"ReservoirSize":1,"ServiceName":"svc","ServiceType":"AWS::EC2::Instance","Host":"*","HTTPMethod":"GET","URLPath":"/*","Version":1}},{"SamplingRule":{"ResourceARN":"*","Priority":2,"FixedRate":0.1,"ReservoirSize":2,"ServiceName":"svc2","ServiceType":"type2","Host":"h2","HTTPMethod":"POST","URLPath":"/p","Version":2}},{"SamplingRule":{"ResourceARN":"*","Priority":3,"FixedRate":0.2,"ReservoirSize":3,"ServiceName":"svc3","ServiceType":"type3","Host":"h3","HTTPMethod":"PUT","URLPath":"/q","Version":3}}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(XRayQuery)
            .data(XRayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ xraySamplingRules(limit: 2) { items { ruleName ruleArn priority fixedRate reservoirSize serviceName serviceType host httpMethod urlPath resourceArn version } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["xraySamplingRules"]["items"];
        assert_eq!(items[0]["ruleName"], "r1");
        assert_eq!(items[0]["ruleArn"], "arn:r1");
        assert_eq!(items[0]["priority"], 1);
        assert_eq!(items[0]["fixedRate"], 0.05);
        assert_eq!(items[0]["reservoirSize"], 1);
        assert_eq!(items[0]["serviceName"], "svc");
        assert_eq!(items[0]["serviceType"], "AWS::EC2::Instance");
        assert_eq!(items[0]["host"], "*");
        assert_eq!(items[0]["httpMethod"], "GET");
        assert_eq!(items[0]["urlPath"], "/*");
        assert_eq!(items[0]["resourceArn"], "*");
        assert_eq!(items[0]["version"], 1);

        assert_eq!(items[1]["ruleName"], serde_json::Value::Null);
        assert_eq!(items[1]["serviceName"], "svc2");

        assert_eq!(json["xraySamplingRules"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn xray_encryption_config_returns_populated_config() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/EncryptionConfig"), ""),
            json_response(
                200,
                r#"{"EncryptionConfig":{"KeyId":"arn:kms:key1","Status":"ACTIVE","Type":"KMS"}}"#,
            ),
        )]);
        let schema = build_query_schema(XRayQuery)
            .data(XRayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ xrayEncryptionConfig { keyId status type } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["xrayEncryptionConfig"]["keyId"], "arn:kms:key1");
        assert_eq!(json["xrayEncryptionConfig"]["status"], "ACTIVE");
        assert_eq!(json["xrayEncryptionConfig"]["type"], "KMS");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn xray_encryption_config_returns_null_when_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/EncryptionConfig"), ""),
            json_response(200, r#"{}"#),
        )]);
        let schema = build_query_schema(XRayQuery)
            .data(XRayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute(r#"{ xrayEncryptionConfig { keyId } }"#).await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["xrayEncryptionConfig"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }
}
