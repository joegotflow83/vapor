use async_graphql::{Context, Object, Result};

use crate::aws::shield::ShieldClient;
use crate::schema::pagination::Page;
use crate::schema::shield::types::{
    AttackSummary, ProtectionGroup, ShieldProtection, ShieldSubscription,
};

#[derive(Default)]
pub struct ShieldQuery;

#[Object]
impl ShieldQuery {
    async fn shield_subscription(&self, ctx: &Context<'_>) -> Result<Option<ShieldSubscription>> {
        let client = ctx.data::<ShieldClient>()?;
        let sub = client.describe_subscription().await?;
        Ok(sub.map(ShieldSubscription::from))
    }

    /// Lists Shield protections, optionally capped at `limit` results and resumed via `next_token`.
    async fn shield_protections(
        &self,
        ctx: &Context<'_>,
        resource_arn: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ShieldProtection>> {
        let client = ctx.data::<ShieldClient>()?;
        let (items, next_token) = client
            .list_protections(resource_arn.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(ShieldProtection::from).collect(),
            next_token,
        })
    }

    /// Lists Shield protection groups, optionally capped at `limit` results and resumed via `next_token`.
    async fn shield_protection_groups(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ProtectionGroup>> {
        let client = ctx.data::<ShieldClient>()?;
        let (items, next_token) = client.list_protection_groups(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(ProtectionGroup::from).collect(),
            next_token,
        })
    }

    /// Lists Shield attacks, optionally capped at `limit` results and resumed via `next_token`.
    async fn shield_attacks(
        &self,
        ctx: &Context<'_>,
        resource_arns: Option<Vec<String>>,
        start_time: Option<String>,
        end_time: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<AttackSummary>> {
        let client = ctx.data::<ShieldClient>()?;
        let (items, next_token) = client
            .list_attacks(resource_arns, start_time, end_time, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(AttackSummary::from).collect(),
            next_token,
        })
    }
}

// All 4 resolvers here are bare passthroughs to already-tested `ShieldClient`
// methods; `types.rs` already has thorough `From`-impl unit tests for all 4
// mapped shapes. `shield_protections` and `shield_attacks` additionally
// forward filter args (`resource_arn` / `resource_arns`+`start_time`+
// `end_time`) into the SDK request, which is real logic worth pinning here
// per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::shield::ShieldClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::ShieldQuery;

    const BASE: &str = "https://shield.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn shield_subscription_maps_subscription() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"Subscription":{"SubscriptionArn":"arn:aws:shield::111111111111:subscription/1","TimeCommitmentInSeconds":31536000,"AutoRenew":"ENABLED","ProactiveEngagementStatus":"ENABLED"}}"#,
            ),
        )]);
        let schema = build_query_schema(ShieldQuery)
            .data(ShieldClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ shieldSubscription { autoRenew proactiveEngagementStatus } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["shieldSubscription"]["autoRenew"], "ENABLED");
        assert_eq!(
            json["shieldSubscription"]["proactiveEngagementStatus"],
            "ENABLED"
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn shield_protections_forwards_filter_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"InclusionFilters":{"ResourceArns":["arn:aws:ec2:us-east-1:111111111111:eip-allocation/eipalloc-1"]},"MaxResults":1}"#,
            ),
            json_response(
                200,
                r#"{"Protections":[{"Id":"p-1","Name":"my-protection","ResourceArn":"arn:aws:cloudfront::111111111111:distribution/E1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(ShieldQuery)
            .data(ShieldClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ shieldProtections(resourceArn: "arn:aws:ec2:us-east-1:111111111111:eip-allocation/eipalloc-1", limit: 1) { items { id name resourceArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["shieldProtections"]["items"];
        assert_eq!(items[0]["id"], "p-1");
        assert_eq!(items[0]["name"], "my-protection");
        assert_eq!(json["shieldProtections"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn shield_protection_groups_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"ProtectionGroups":[{"ProtectionGroupId":"pg-1","Aggregation":"SUM","Pattern":"ALL","Members":[]}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(ShieldQuery)
            .data(ShieldClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ shieldProtectionGroups(limit: 1) { items { protectionGroupId aggregation pattern } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["shieldProtectionGroups"]["items"];
        assert_eq!(items[0]["protectionGroupId"], "pg-1");
        assert_eq!(items[0]["aggregation"], "SUM");
        assert_eq!(json["shieldProtectionGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn shield_attacks_forwards_filters_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"ResourceArns":["arn:aws:cloudfront::111111111111:distribution/E1"],"StartTime":{"FromInclusive":1700000000},"EndTime":{"ToExclusive":1700003600}}"#,
            ),
            json_response(
                200,
                r#"{"AttackSummaries":[{"AttackId":"a-1","ResourceArn":"arn:aws:cloudfront::111111111111:distribution/E1"}]}"#,
            ),
        )]);
        let schema = build_query_schema(ShieldQuery)
            .data(ShieldClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ shieldAttacks(resourceArns: ["arn:aws:cloudfront::111111111111:distribution/E1"], startTime: "2023-11-14T22:13:20Z", endTime: "2023-11-14T23:13:20Z") { items { attackId resourceArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["shieldAttacks"]["items"];
        assert_eq!(items[0]["attackId"], "a-1");
        assert_eq!(
            items[0]["resourceArn"],
            "arn:aws:cloudfront::111111111111:distribution/E1"
        );
        assert_eq!(json["shieldAttacks"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }
}
