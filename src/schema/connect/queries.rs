use async_graphql::{Context, Object, Result};

use crate::aws::connect::ConnectClient;
use crate::schema::connect::types::{
    ConnectContactFlow, ConnectInstance, ConnectQueue, ConnectUser,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct ConnectQuery;

#[Object]
impl ConnectQuery {
    /// Lists Connect instances. `limit` caps the number of results returned
    /// (default: unlimited); `next_token` resumes from a prior page.
    async fn connect_instances(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ConnectInstance>> {
        let client = ctx.data::<ConnectClient>()?;
        let (items, next_token) = client.list_instances(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(ConnectInstance::from).collect(),
            next_token,
        })
    }

    /// Lists queues for a Connect instance. `limit` caps the number of
    /// results returned (default: unlimited); `next_token` resumes from a
    /// prior page.
    async fn connect_queues(
        &self,
        ctx: &Context<'_>,
        instance_id: String,
        queue_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ConnectQueue>> {
        let client = ctx.data::<ConnectClient>()?;
        let (items, next_token) = client
            .list_queues(&instance_id, queue_types, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(ConnectQueue::from).collect(),
            next_token,
        })
    }

    /// Lists contact flows for a Connect instance. `limit` caps the number
    /// of results returned (default: unlimited); `next_token` resumes from a
    /// prior page.
    async fn connect_contact_flows(
        &self,
        ctx: &Context<'_>,
        instance_id: String,
        contact_flow_types: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ConnectContactFlow>> {
        let client = ctx.data::<ConnectClient>()?;
        let (items, next_token) = client
            .list_contact_flows(&instance_id, contact_flow_types, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(ConnectContactFlow::from).collect(),
            next_token,
        })
    }

    /// Lists users for a Connect instance. `limit` caps the number of
    /// results returned (default: unlimited); `next_token` resumes from a
    /// prior page.
    async fn connect_users(
        &self,
        ctx: &Context<'_>,
        instance_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ConnectUser>> {
        let client = ctx.data::<ConnectClient>()?;
        let (items, next_token) = client.list_users(&instance_id, limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(ConnectUser::from).collect(),
            next_token,
        })
    }
}

// All four resolvers are 1:1 passthroughs to a single already-tested
// `ConnectClient` method each (see `src/aws/connect.rs`'s own test module for
// the pagination/describe-fan-out/error-mapping behavior) — only light smoke
// tests are needed here per the resolver-layer sweep's stated scope
// (config_svc precedent: one test per resolver, `limit` matched to the mocked
// item count so the hand-rolled pagination loop doesn't over-page past the
// mock event list when the response also carries a `NextToken`).
#[cfg(test)]
mod tests {
    use crate::aws::connect::ConnectClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::ConnectQuery;

    const BASE: &str = "https://connect.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn connect_instances_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/instance?maxResults=1"), ""),
            json_response(
                200,
                r#"{"InstanceSummaryList":[{"Id":"i1","Arn":"arn1","IdentityManagementType":"CONNECT_MANAGED","InstanceAlias":"my-cc","ServiceRole":"role-arn","InstanceStatus":"ACTIVE","InboundCallsEnabled":true,"OutboundCallsEnabled":false}],"NextToken":"cursor-a"}"#,
            ),
        )]);
        let schema = build_query_schema(ConnectQuery)
            .data(ConnectClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ connectInstances(limit: 1) { items { id arn identityManagementType instanceAlias serviceRole instanceStatus inboundCallsEnabled outboundCallsEnabled } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["connectInstances"]["items"];
        assert_eq!(items[0]["id"], "i1");
        assert_eq!(items[0]["arn"], "arn1");
        assert_eq!(items[0]["identityManagementType"], "CONNECT_MANAGED");
        assert_eq!(items[0]["instanceAlias"], "my-cc");
        assert_eq!(items[0]["serviceRole"], "role-arn");
        assert_eq!(items[0]["instanceStatus"], "ACTIVE");
        assert_eq!(items[0]["inboundCallsEnabled"], true);
        assert_eq!(items[0]["outboundCallsEnabled"], false);
        assert_eq!(json["connectInstances"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn connect_queues_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/queues-summary/inst1?maxResults=1"), ""),
                json_response(
                    200,
                    r#"{"QueueSummaryList":[{"Id":"q1","Arn":"qarn1","Name":"queue-one","QueueType":"STANDARD"}],"NextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/queues/inst1/q1"), ""),
                json_response(
                    200,
                    r#"{"Queue":{"QueueId":"q1","Description":"first queue","Status":"ENABLED"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(ConnectQuery)
            .data(ConnectClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ connectQueues(instanceId: "inst1", limit: 1) { items { queueId queueArn name description queueType status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["connectQueues"]["items"];
        assert_eq!(items[0]["queueId"], "q1");
        assert_eq!(items[0]["queueArn"], "qarn1");
        assert_eq!(items[0]["name"], "queue-one");
        assert_eq!(items[0]["queueType"], "STANDARD");
        assert_eq!(items[0]["description"], "first queue");
        assert_eq!(items[0]["status"], "ENABLED");
        assert_eq!(json["connectQueues"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn connect_contact_flows_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/contact-flows-summary/inst1?maxResults=1"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"ContactFlowSummaryList":[{"Id":"f1","Arn":"farn1","Name":"flow-one","ContactFlowType":"CONTACT_FLOW"}],"NextToken":"cursor-c"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/contact-flows/inst1/f1"), ""),
                json_response(
                    200,
                    r#"{"ContactFlow":{"Id":"f1","Description":"first flow"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(ConnectQuery)
            .data(ConnectClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ connectContactFlows(instanceId: "inst1", limit: 1) { items { id arn name contactFlowType description } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["connectContactFlows"]["items"];
        assert_eq!(items[0]["id"], "f1");
        assert_eq!(items[0]["arn"], "farn1");
        assert_eq!(items[0]["name"], "flow-one");
        assert_eq!(items[0]["contactFlowType"], "CONTACT_FLOW");
        assert_eq!(items[0]["description"], "first flow");
        assert_eq!(json["connectContactFlows"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn connect_users_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/users-summary/inst1?maxResults=1"), ""),
                json_response(
                    200,
                    r#"{"UserSummaryList":[{"Id":"u1","Arn":"uarn1","Username":"alice"}],"NextToken":"cursor-d"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/users/inst1/u1"), ""),
                json_response(
                    200,
                    r#"{"User":{"Id":"u1","RoutingProfileId":"rp1","HierarchyGroupId":"hg1","SecurityProfileIds":["sp1","sp2"]}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(ConnectQuery)
            .data(ConnectClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ connectUsers(instanceId: "inst1", limit: 1) { items { id arn username routingProfileId hierarchyGroupId securityProfileIds } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["connectUsers"]["items"];
        assert_eq!(items[0]["id"], "u1");
        assert_eq!(items[0]["arn"], "uarn1");
        assert_eq!(items[0]["username"], "alice");
        assert_eq!(items[0]["routingProfileId"], "rp1");
        assert_eq!(items[0]["hierarchyGroupId"], "hg1");
        assert_eq!(items[0]["securityProfileIds"][0], "sp1");
        assert_eq!(items[0]["securityProfileIds"][1], "sp2");
        assert_eq!(json["connectUsers"]["nextToken"], "cursor-d");
        http_client.relaxed_requests_match();
    }
}
