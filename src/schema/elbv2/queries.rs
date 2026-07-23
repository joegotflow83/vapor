use async_graphql::{Context, Object, Result};

use crate::aws::elbv2::Elbv2Client;
use crate::schema::elbv2::types::{Listener, ListenerRule, LoadBalancer, TargetGroup, TargetHealthInfo};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct Elbv2Query;

#[Object]
impl Elbv2Query {
    /// Lists load balancers. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn load_balancers(
        &self,
        ctx: &Context<'_>,
        arns: Option<Vec<String>>,
        names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<LoadBalancer>> {
        let client = ctx.data::<Elbv2Client>()?;
        let (results, next_token) = client
            .describe_load_balancers(arns, names, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(LoadBalancer::from).collect(),
            next_token,
        })
    }

    /// Lists target groups. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn target_groups(
        &self,
        ctx: &Context<'_>,
        arns: Option<Vec<String>>,
        load_balancer_arn: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TargetGroup>> {
        let client = ctx.data::<Elbv2Client>()?;
        let (results, next_token) = client
            .describe_target_groups(arns, load_balancer_arn, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(TargetGroup::from).collect(),
            next_token,
        })
    }

    async fn target_health(
        &self,
        ctx: &Context<'_>,
        target_group_arn: String,
    ) -> Result<Vec<TargetHealthInfo>> {
        let client = ctx.data::<Elbv2Client>()?;
        let results = client.describe_target_health(target_group_arn).await?;
        Ok(results.into_iter().map(TargetHealthInfo::from).collect())
    }

    /// Lists listeners for a load balancer. `limit` caps the total number of
    /// results (default unlimited); pass `nextToken` from a prior page to
    /// resume.
    async fn listeners(
        &self,
        ctx: &Context<'_>,
        load_balancer_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<Listener>> {
        let client = ctx.data::<Elbv2Client>()?;
        let (results, next_token) = client
            .describe_listeners(load_balancer_arn, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(Listener::from).collect(),
            next_token,
        })
    }

    /// Lists rules for a listener. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn listener_rules(
        &self,
        ctx: &Context<'_>,
        listener_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ListenerRule>> {
        let client = ctx.data::<Elbv2Client>()?;
        let (results, next_token) = client.describe_rules(listener_arn, limit, next_token).await?;
        Ok(Page {
            items: results.into_iter().map(ListenerRule::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::elbv2::Elbv2Client;
    use crate::aws::test_util::{request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::Elbv2Query;

    const ENDPOINT: &str = "https://elasticloadbalancing.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn load_balancers_maps_items_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeLoadBalancers&Version=2015-12-01&PageSize=1",
            ),
            xml_response(
                200,
                "<DescribeLoadBalancersResponse><DescribeLoadBalancersResult><LoadBalancers>\
                 <member><LoadBalancerArn>lb-arn-1</LoadBalancerArn><LoadBalancerName>my-lb</LoadBalancerName>\
                 <Type>application</Type><Scheme>internet-facing</Scheme>\
                 <State><Code>active</Code></State>\
                 <CreatedTime>2023-11-14T22:13:20.000Z</CreatedTime></member>\
                 </LoadBalancers><NextMarker>page2</NextMarker></DescribeLoadBalancersResult></DescribeLoadBalancersResponse>",
            ),
        )]);
        let schema = build_query_schema(Elbv2Query)
            .data(Elbv2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ loadBalancers(limit: 1) { items { arn name scheme lbType state createdTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["loadBalancers"]["items"];
        assert_eq!(items[0]["arn"], "lb-arn-1");
        assert_eq!(items[0]["name"], "my-lb");
        assert_eq!(items[0]["scheme"], "INTERNET_FACING");
        assert_eq!(items[0]["lbType"], "APPLICATION");
        assert_eq!(items[0]["state"], "active");
        assert_eq!(items[0]["createdTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(json["loadBalancers"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn target_groups_forwards_load_balancer_arn_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTargetGroups&Version=2015-12-01&LoadBalancerArn=lb-arn-1&PageSize=1",
            ),
            xml_response(
                200,
                "<DescribeTargetGroupsResponse><DescribeTargetGroupsResult><TargetGroups>\
                 <member><TargetGroupArn>tg-arn-1</TargetGroupArn><TargetGroupName>my-tg</TargetGroupName>\
                 <Protocol>HTTPS</Protocol><Port>443</Port><TargetType>instance</TargetType></member>\
                 </TargetGroups><NextMarker>page2</NextMarker></DescribeTargetGroupsResult></DescribeTargetGroupsResponse>",
            ),
        )]);
        let schema = build_query_schema(Elbv2Query)
            .data(Elbv2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ targetGroups(loadBalancerArn: "lb-arn-1", limit: 1) { items { arn name protocol port targetType } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["targetGroups"]["items"];
        assert_eq!(items[0]["arn"], "tg-arn-1");
        assert_eq!(items[0]["name"], "my-tg");
        assert_eq!(items[0]["protocol"], "HTTPS");
        assert_eq!(items[0]["port"], 443);
        assert_eq!(items[0]["targetType"], "INSTANCE");
        assert_eq!(json["targetGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn target_health_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeTargetHealth&Version=2015-12-01&TargetGroupArn=tg-arn-1",
            ),
            xml_response(
                200,
                "<DescribeTargetHealthResponse><DescribeTargetHealthResult><TargetHealthDescriptions>\
                 <member><Target><Id>i-1234</Id><Port>80</Port></Target>\
                 <TargetHealth><State>healthy</State></TargetHealth></member>\
                 </TargetHealthDescriptions></DescribeTargetHealthResult></DescribeTargetHealthResponse>",
            ),
        )]);
        let schema = build_query_schema(Elbv2Query)
            .data(Elbv2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ targetHealth(targetGroupArn: "tg-arn-1") { targetId port healthState } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["targetHealth"];
        assert_eq!(items[0]["targetId"], "i-1234");
        assert_eq!(items[0]["port"], 80);
        assert_eq!(items[0]["healthState"], "healthy");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn listeners_forwards_load_balancer_arn_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeListeners&Version=2015-12-01&LoadBalancerArn=lb-arn-1&PageSize=1",
            ),
            xml_response(
                200,
                "<DescribeListenersResponse><DescribeListenersResult><Listeners>\
                 <member><ListenerArn>listener-arn-1</ListenerArn><Protocol>HTTPS</Protocol><Port>443</Port></member>\
                 </Listeners><NextMarker>page2</NextMarker></DescribeListenersResult></DescribeListenersResponse>",
            ),
        )]);
        let schema = build_query_schema(Elbv2Query)
            .data(Elbv2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ listeners(loadBalancerArn: "lb-arn-1", limit: 1) { items { arn protocol port } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["listeners"]["items"];
        assert_eq!(items[0]["arn"], "listener-arn-1");
        assert_eq!(items[0]["protocol"], "HTTPS");
        assert_eq!(items[0]["port"], 443);
        assert_eq!(json["listeners"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn listener_rules_maps_conditions_and_actions() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeRules&Version=2015-12-01&ListenerArn=listener-arn-1&PageSize=1",
            ),
            xml_response(
                200,
                "<DescribeRulesResponse><DescribeRulesResult><Rules>\
                 <member><RuleArn>rule-arn-1</RuleArn><Priority>10</Priority><IsDefault>false</IsDefault>\
                 <Conditions><member><Field>path-pattern</Field><Values><member>/api/*</member></Values></member></Conditions>\
                 <Actions><member><Type>forward</Type><TargetGroupArn>tg-arn-1</TargetGroupArn></member></Actions></member>\
                 </Rules><NextMarker>page2</NextMarker></DescribeRulesResult></DescribeRulesResponse>",
            ),
        )]);
        let schema = build_query_schema(Elbv2Query)
            .data(Elbv2Client::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ listenerRules(listenerArn: "listener-arn-1", limit: 1) { items { ruleArn priority isDefault conditions { field values } actions { actionType targetGroupArn } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["listenerRules"]["items"];
        assert_eq!(items[0]["ruleArn"], "rule-arn-1");
        assert_eq!(items[0]["priority"], "10");
        assert_eq!(items[0]["isDefault"], false);
        assert_eq!(items[0]["conditions"][0]["field"], "path-pattern");
        assert_eq!(items[0]["conditions"][0]["values"][0], "/api/*");
        assert_eq!(items[0]["actions"][0]["actionType"], "forward");
        assert_eq!(items[0]["actions"][0]["targetGroupArn"], "tg-arn-1");
        assert_eq!(json["listenerRules"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
