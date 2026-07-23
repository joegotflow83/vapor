use async_graphql::{Context, Object, Result};

use crate::aws::iot::IotClient;
use crate::schema::iot::types::{IotCertificate, IotPolicy, IotThing, IotThingGroup, IotTopicRule};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct IotQuery;

#[Object]
impl IotQuery {
    /// Lists IoT things, optionally capped at `limit` results (default unlimited).
    async fn iot_things(
        &self,
        ctx: &Context<'_>,
        thing_type_name: Option<String>,
        attribute_name: Option<String>,
        attribute_value: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IotThing>> {
        let client = ctx.data::<IotClient>()?;
        let (items, next_token) = client
            .list_things(thing_type_name, attribute_name, attribute_value, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(IotThing::from).collect(),
            next_token,
        })
    }

    /// Lists IoT thing groups, optionally capped at `limit` results (default unlimited).
    async fn iot_thing_groups(
        &self,
        ctx: &Context<'_>,
        parent_group: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IotThingGroup>> {
        let client = ctx.data::<IotClient>()?;
        let (items, next_token) = client.list_thing_groups(parent_group, limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(IotThingGroup::from).collect(),
            next_token,
        })
    }

    /// Lists IoT policies, optionally capped at `limit` results (default unlimited).
    async fn iot_policies(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IotPolicy>> {
        let client = ctx.data::<IotClient>()?;
        let (items, next_token) = client.list_policies(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(IotPolicy::from).collect(),
            next_token,
        })
    }

    /// Lists IoT certificates, optionally capped at `limit` results (default unlimited).
    async fn iot_certificates(
        &self,
        ctx: &Context<'_>,
        ascending_order: Option<bool>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IotCertificate>> {
        let client = ctx.data::<IotClient>()?;
        let (items, next_token) = client
            .list_certificates(ascending_order, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(IotCertificate::from).collect(),
            next_token,
        })
    }

    /// Lists IoT topic rules, optionally capped at `limit` results (default unlimited).
    async fn iot_topic_rules(
        &self,
        ctx: &Context<'_>,
        topic_rule_disabled: Option<bool>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IotTopicRule>> {
        let client = ctx.data::<IotClient>()?;
        let (items, next_token) = client
            .list_topic_rules(topic_rule_disabled, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(IotTopicRule::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::iot::IotClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::IotQuery;

    const BASE: &str = "https://iot.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn iot_things_maps_items_and_next_token() {
        // `limit: 1` applied proactively (gotcha 29): the mocked response
        // carries a `nextToken`, and without a `limit` arg `list_things`'s
        // internal pagination loop keeps fetching until the token runs out,
        // starving the single-event replay queue.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/things?maxResults=1"), ""),
            json_response(
                200,
                r#"{"things":[{"thingName":"t1","thingArn":"arn1","thingTypeName":"sensor","attributes":{"loc":"nyc"},"version":1}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(IotQuery)
            .data(IotClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iotThings(limit: 1) { items { thingName thingArn thingTypeName attributes { key value } version } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iotThings"]["items"];
        assert_eq!(items[0]["thingName"], "t1");
        assert_eq!(items[0]["thingArn"], "arn1");
        assert_eq!(items[0]["thingTypeName"], "sensor");
        assert_eq!(items[0]["attributes"][0]["key"], "loc");
        assert_eq!(items[0]["attributes"][0]["value"], "nyc");
        assert_eq!(items[0]["version"], 1);
        assert_eq!(json["iotThings"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iot_thing_groups_maps_items_via_describe_fan_out() {
        // `limit: 1` applied proactively (gotcha 29) since the discovery
        // response carries a `nextToken`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups?maxResults=1"), ""),
                json_response(
                    200,
                    r#"{"thingGroups":[{"groupName":"g1","groupArn":"garn1"}],"nextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/thing-groups/g1"), ""),
                json_response(200, r#"{"thingGroupId":"gid1","status":"ACTIVE"}"#),
            ),
        ]);
        let schema = build_query_schema(IotQuery)
            .data(IotClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ iotThingGroups(limit: 1) { items { groupName groupArn groupId status } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iotThingGroups"]["items"];
        assert_eq!(items[0]["groupName"], "g1");
        assert_eq!(items[0]["groupArn"], "garn1");
        assert_eq!(items[0]["groupId"], "gid1");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(json["iotThingGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iot_policies_maps_items_and_next_token() {
        // `limit: 1` applied proactively (gotcha 29) since the mocked
        // response carries a `nextMarker` (`ListPolicies`' `page_size`/
        // `marker` naming mismatch, per `src/aws/iot.rs`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/policies?pageSize=1"), ""),
            json_response(
                200,
                r#"{"policies":[{"policyName":"p1","policyArn":"parn1"}],"nextMarker":"m2"}"#,
            ),
        )]);
        let schema = build_query_schema(IotQuery)
            .data(IotClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ iotPolicies(limit: 1) { items { policyName policyArn } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iotPolicies"]["items"];
        assert_eq!(items[0]["policyName"], "p1");
        assert_eq!(items[0]["policyArn"], "parn1");
        assert_eq!(json["iotPolicies"]["nextToken"], "m2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iot_certificates_maps_items_and_next_token() {
        // `limit: 1` applied proactively (gotcha 29) since the mocked
        // response carries a `nextMarker` (`ListCertificates`' `page_size`/
        // `marker` naming mismatch, per `src/aws/iot.rs`).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/certificates?pageSize=1"), ""),
            json_response(
                200,
                r#"{"certificates":[{"certificateId":"c1","certificateArn":"carn1","status":"ACTIVE","creationDate":1700000000}],"nextMarker":"m2"}"#,
            ),
        )]);
        let schema = build_query_schema(IotQuery)
            .data(IotClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iotCertificates(limit: 1) { items { certificateId certificateArn status creationDate } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iotCertificates"]["items"];
        assert_eq!(items[0]["certificateId"], "c1");
        assert_eq!(items[0]["certificateArn"], "carn1");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert!(items[0]["creationDate"].is_string());
        assert_eq!(json["iotCertificates"]["nextToken"], "m2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iot_topic_rules_maps_items_and_next_token() {
        // `limit: 1` applied proactively (gotcha 29) since the mocked
        // response carries a `nextToken`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/rules?maxResults=1"), ""),
            json_response(
                200,
                r#"{"rules":[{"ruleName":"r1","topicPattern":"sensors/+/temp","createdAt":1700000000,"ruleDisabled":false,"ruleArn":"rarn1"}],"nextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(IotQuery)
            .data(IotClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iotTopicRules(limit: 1) { items { ruleName topicPattern createdAt ruleDisabled ruleArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iotTopicRules"]["items"];
        assert_eq!(items[0]["ruleName"], "r1");
        assert_eq!(items[0]["topicPattern"], "sensors/+/temp");
        assert!(items[0]["createdAt"].is_string());
        assert_eq!(items[0]["ruleDisabled"], false);
        assert_eq!(items[0]["ruleArn"], "rarn1");
        assert_eq!(json["iotTopicRules"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
