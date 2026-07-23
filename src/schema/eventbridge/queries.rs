use async_graphql::{Context, Object, Result};

use crate::aws::eventbridge::EventBridgeClient;
use crate::schema::pagination::Page;
use super::types::{EbEventBus, EbRule, EbTarget};

#[derive(Default)]
pub struct EventBridgeQuery;

#[Object]
impl EventBridgeQuery {
    /// Lists event buses, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn event_bridge_buses(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EbEventBus>> {
        let client = ctx.data::<EventBridgeClient>()?;
        let (buses, token) = client.list_event_buses(limit, next_token).await?;
        Ok(Page {
            items: buses.iter().map(EbEventBus::from).collect(),
            next_token: token,
        })
    }

    /// Lists rules, optionally scoped to `event_bus_name`, capped at `limit`
    /// results (default unlimited), and resumed from `next_token`.
    async fn event_bridge_rules(
        &self,
        ctx: &Context<'_>,
        event_bus_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EbRule>> {
        let client = ctx.data::<EventBridgeClient>()?;
        let (rules, token) = client
            .list_rules(event_bus_name.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: rules.iter().map(EbRule::from).collect(),
            next_token: token,
        })
    }

    /// Lists targets for `rule_name`, optionally scoped to `event_bus_name`,
    /// capped at `limit` results (default unlimited), and resumed from
    /// `next_token`.
    async fn event_bridge_targets(
        &self,
        ctx: &Context<'_>,
        rule_name: String,
        event_bus_name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EbTarget>> {
        let client = ctx.data::<EventBridgeClient>()?;
        let (targets, token) = client
            .list_targets_by_rule(&rule_name, event_bus_name.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: targets.iter().map(EbTarget::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::eventbridge::EventBridgeClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::EventBridgeQuery;

    const ENDPOINT: &str = "https://events.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn event_bridge_buses_maps_items_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":1}"#),
            json_response(
                200,
                r#"{"EventBuses":[{"Name":"default","Arn":"arn:aws:events:us-east-1:1:event-bus/default","Description":"My event bus","Policy":"{\"Statement\":[]}"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EventBridgeQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ eventBridgeBuses(limit: 1) { items { name arn description policy createdBy } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["eventBridgeBuses"]["items"][0]["name"], "default");
        assert_eq!(
            data["eventBridgeBuses"]["items"][0]["arn"],
            "arn:aws:events:us-east-1:1:event-bus/default"
        );
        assert_eq!(data["eventBridgeBuses"]["items"][0]["description"], "My event bus");
        assert_eq!(data["eventBridgeBuses"]["items"][0]["policy"], "{\"Statement\":[]}");
        assert!(data["eventBridgeBuses"]["items"][0]["createdBy"].is_null());
        assert_eq!(data["eventBridgeBuses"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn event_bridge_rules_forwards_event_bus_name_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"EventBusName":"custom-bus"}"#),
            json_response(
                200,
                r#"{"Rules":[{"Name":"my-rule","Arn":"arn:aws:events:us-east-1:1:rule/my-rule","EventBusName":"custom-bus","State":"ENABLED","Description":"My rule","ScheduleExpression":"rate(5 minutes)","EventPattern":"{\"source\":[\"aws.ec2\"]}","RoleArn":"arn:aws:iam::1:role/MyRole"}]}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EventBridgeQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ eventBridgeRules(eventBusName: "custom-bus") { items { name arn eventBusName state description scheduleExpression eventPattern roleArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["eventBridgeRules"]["items"][0]["name"], "my-rule");
        assert_eq!(data["eventBridgeRules"]["items"][0]["eventBusName"], "custom-bus");
        assert_eq!(data["eventBridgeRules"]["items"][0]["state"], "ENABLED");
        assert_eq!(
            data["eventBridgeRules"]["items"][0]["scheduleExpression"],
            "rate(5 minutes)"
        );
        assert_eq!(
            data["eventBridgeRules"]["items"][0]["eventPattern"],
            "{\"source\":[\"aws.ec2\"]}"
        );
        assert_eq!(
            data["eventBridgeRules"]["items"][0]["roleArn"],
            "arn:aws:iam::1:role/MyRole"
        );
        assert!(data["eventBridgeRules"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn event_bridge_targets_forwards_rule_name_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Rule":"my-rule","Limit":1}"#),
            json_response(
                200,
                r#"{"Targets":[{"Id":"1","Arn":"arn:aws:lambda:us-east-1:1:function:fn1","RoleArn":"arn:aws:iam::1:role/MyRole","Input":"{\"key\":\"value\"}","InputPath":"$.body"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = EventBridgeClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EventBridgeQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ eventBridgeTargets(ruleName: "my-rule", limit: 1) { items { id arn roleArn input inputPath } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["eventBridgeTargets"]["items"][0]["id"], "1");
        assert_eq!(
            data["eventBridgeTargets"]["items"][0]["arn"],
            "arn:aws:lambda:us-east-1:1:function:fn1"
        );
        assert_eq!(
            data["eventBridgeTargets"]["items"][0]["roleArn"],
            "arn:aws:iam::1:role/MyRole"
        );
        assert_eq!(data["eventBridgeTargets"]["items"][0]["input"], "{\"key\":\"value\"}");
        assert_eq!(data["eventBridgeTargets"]["items"][0]["inputPath"], "$.body");
        assert_eq!(data["eventBridgeTargets"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
