use async_graphql::{Context, Object, Result};

use crate::aws::mq::MqClient;
use crate::schema::mq::types::{MqBroker, MqConfiguration};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct MqQuery;

#[Object]
impl MqQuery {
    /// Lists brokers, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`.
    async fn mq_brokers(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MqBroker>> {
        let client = ctx.data::<MqClient>()?;
        let (brokers, next_token) = client.list_brokers(limit, next_token).await?;
        Ok(Page {
            items: brokers.into_iter().map(MqBroker::from).collect(),
            next_token,
        })
    }

    async fn mq_broker(&self, ctx: &Context<'_>, broker_id: String) -> Result<Option<MqBroker>> {
        let client = ctx.data::<MqClient>()?;
        let broker = client.describe_broker(&broker_id).await?;
        Ok(broker.map(MqBroker::from))
    }

    /// Lists configurations, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn mq_configurations(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<MqConfiguration>> {
        let client = ctx.data::<MqClient>()?;
        let (configs, next_token) = client.list_configurations(limit, next_token).await?;
        Ok(Page {
            items: configs.into_iter().map(MqConfiguration::from).collect(),
            next_token,
        })
    }
}

// `mq_brokers` wraps `MqClient::list_brokers`'s internal list+per-id
// `describe_broker` fan-out (already covered by `src/aws/mq.rs`'s own test
// module), so per the connect/control_tower/datasync precedent it's
// exercised end-to-end here via 2 `ReplayEvent`s (list + describe) to prove
// real item-mapping through the GraphQL layer, plus a dedicated
// found/not-found pair for the single-id `mq_broker` passthrough (acm
// precedent). `mq_configurations` is a 1:1 passthrough, light-smoke only.
#[cfg(test)]
mod tests {
    use crate::aws::mq::MqClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::MqQuery;

    const BASE: &str = "https://mq.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn mq_brokers_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers?maxResults=1"), ""),
                json_response(
                    200,
                    r#"{"brokerSummaries":[{"brokerId":"b1"}],"nextToken":"cursor-a"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v1/brokers/b1"), ""),
                json_response(
                    200,
                    r#"{"brokerId":"b1","brokerArn":"arn:b1","brokerName":"broker-one","brokerState":"RUNNING","engineType":"ACTIVEMQ","engineVersion":"5.17.6","deploymentMode":"SINGLE_INSTANCE","hostInstanceType":"mq.t3.micro","publiclyAccessible":true,"subnetIds":["subnet-1"],"securityGroups":["sg-1"],"tags":{"env":"prod"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(MqQuery)
            .data(MqClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ mqBrokers(limit: 1) { items { brokerId brokerArn brokerName brokerState engineType engineVersion deploymentMode hostInstanceType publiclyAccessible subnetIds securityGroups tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["mqBrokers"]["items"];
        assert_eq!(items[0]["brokerId"], "b1");
        assert_eq!(items[0]["brokerArn"], "arn:b1");
        assert_eq!(items[0]["brokerName"], "broker-one");
        assert_eq!(items[0]["brokerState"], "RUNNING");
        assert_eq!(items[0]["engineType"], "ACTIVEMQ");
        assert_eq!(items[0]["deploymentMode"], "SINGLE_INSTANCE");
        assert_eq!(items[0]["publiclyAccessible"], true);
        assert_eq!(items[0]["subnetIds"][0], "subnet-1");
        assert_eq!(items[0]["securityGroups"][0], "sg-1");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["mqBrokers"]["nextToken"], "cursor-a");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn mq_broker_returns_full_broker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/brokers/b1"), ""),
            json_response(
                200,
                r#"{"brokerId":"b1","brokerName":"broker-one","brokerState":"RUNNING"}"#,
            ),
        )]);
        let schema = build_query_schema(MqQuery)
            .data(MqClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ mqBroker(brokerId: "b1") { brokerId brokerName brokerState } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["mqBroker"]["brokerId"], "b1");
        assert_eq!(json["mqBroker"]["brokerName"], "broker-one");
        assert_eq!(json["mqBroker"]["brokerState"], "RUNNING");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn mq_broker_returns_null_when_not_found() {
        use crate::aws::test_util::json_error_response;

        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/brokers/missing"), ""),
            json_error_response("NotFoundException", "broker not found"),
        )]);
        let schema = build_query_schema(MqQuery)
            .data(MqClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ mqBroker(brokerId: "missing") { brokerId } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["mqBroker"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn mq_configurations_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v1/configurations?maxResults=1"), ""),
            json_response(
                200,
                r#"{"configurations":[{"id":"c1","arn":"arn:c1","name":"cfg-one","engineType":"RABBITMQ","engineVersion":"3.11.20","description":"first config","created":"2024-01-01T00:00:00Z","latestRevision":{"revision":3},"tags":{"team":"platform"}}],"nextToken":"cursor-b"}"#,
            ),
        )]);
        let schema = build_query_schema(MqQuery)
            .data(MqClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ mqConfigurations(limit: 1) { items { id arn name engineType engineVersion description latestRevision created tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["mqConfigurations"]["items"];
        assert_eq!(items[0]["id"], "c1");
        assert_eq!(items[0]["arn"], "arn:c1");
        assert_eq!(items[0]["name"], "cfg-one");
        assert_eq!(items[0]["engineType"], "RABBITMQ");
        assert_eq!(items[0]["latestRevision"], 3);
        assert_eq!(items[0]["created"], "2024-01-01T00:00:00+00:00");
        assert_eq!(items[0]["tags"][0]["key"], "team");
        assert_eq!(items[0]["tags"][0]["value"], "platform");
        assert_eq!(json["mqConfigurations"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }
}
