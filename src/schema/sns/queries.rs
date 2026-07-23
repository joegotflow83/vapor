use async_graphql::{Context, Object, Result};

use crate::aws::sns::SnsClient;
use crate::schema::pagination::Page;
use crate::schema::sns::types::{SnsSubscription, SnsTopic};

#[derive(Default)]
pub struct SnsQuery;

#[Object]
impl SnsQuery {
    /// List all SNS topics with their attributes. `limit` caps the number of
    /// topics returned (default unlimited).
    async fn sns_topics(
        &self,
        ctx: &Context<'_>,
        next_token: Option<String>,
        limit: Option<i32>,
    ) -> Result<Page<SnsTopic>> {
        let client = ctx.data::<SnsClient>()?;
        let (results, next_token) = client
            .list_topics_with_attributes(next_token, limit)
            .await?;
        Ok(Page {
            items: results
                .into_iter()
                .map(|(arn, attrs)| SnsTopic::from_attrs(arn, attrs))
                .collect(),
            next_token,
        })
    }

    /// Fetch a single SNS topic by ARN.
    async fn sns_topic(
        &self,
        ctx: &Context<'_>,
        topic_arn: String,
    ) -> Result<Option<SnsTopic>> {
        let client = ctx.data::<SnsClient>()?;
        let attrs = client.get_topic_attributes(&topic_arn).await?;
        Ok(Some(SnsTopic::from_attrs(topic_arn, attrs)))
    }

    /// List SNS subscriptions. Optionally filter by topicArn. `limit` caps
    /// the number of subscriptions returned (default unlimited).
    async fn sns_subscriptions(
        &self,
        ctx: &Context<'_>,
        topic_arn: Option<String>,
        next_token: Option<String>,
        limit: Option<i32>,
    ) -> Result<Page<SnsSubscription>> {
        let client = ctx.data::<SnsClient>()?;
        let (results, next_token) = client
            .list_subscriptions(topic_arn.as_deref(), next_token, limit)
            .await?;
        Ok(Page {
            items: results.into_iter().map(SnsSubscription::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::sns::SnsClient;
    use crate::aws::test_util::{request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::SnsQuery;

    const ENDPOINT: &str = "https://sns.us-east-1.amazonaws.com/";
    const TOPIC_ARN: &str = "arn:aws:sns:us-east-1:123456789012:my-topic";
    const TOPIC_ARN_ENC: &str = "arn%3Aaws%3Asns%3Aus-east-1%3A123456789012%3Amy-topic";

    // --- sns_topics (list + per-topic attribute fan-out) ---

    #[tokio::test]
    async fn sns_topics_fans_out_list_and_attributes() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListTopics&Version=2010-03-31"),
                xml_response(
                    200,
                    format!(
                        "<ListTopicsResponse><ListTopicsResult><Topics>\
                         <member><TopicArn>{TOPIC_ARN}</TopicArn></member>\
                         </Topics></ListTopicsResult></ListTopicsResponse>"
                    ),
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    format!("Action=GetTopicAttributes&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}"),
                ),
                xml_response(
                    200,
                    "<GetTopicAttributesResponse><GetTopicAttributesResult><Attributes>\
                     <entry><key>DisplayName</key><value>My Topic</value></entry>\
                     <entry><key>Owner</key><value>123456789012</value></entry>\
                     <entry><key>FifoTopic</key><value>true</value></entry>\
                     <entry><key>ContentBasedDeduplication</key><value>true</value></entry>\
                     <entry><key>SubscriptionsConfirmed</key><value>3</value></entry>\
                     </Attributes></GetTopicAttributesResult></GetTopicAttributesResponse>",
                ),
            ),
        ]);
        let schema = build_query_schema(SnsQuery)
            .data(SnsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ snsTopics { items { arn displayName owner fifoTopic \
                 contentBasedDeduplication subscriptionsConfirmed } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["snsTopics"]["items"][0];
        assert_eq!(item["arn"], TOPIC_ARN);
        assert_eq!(item["displayName"], "My Topic");
        assert_eq!(item["owner"], "123456789012");
        assert_eq!(item["fifoTopic"], true);
        assert_eq!(item["contentBasedDeduplication"], true);
        assert_eq!(item["subscriptionsConfirmed"], 3);
        assert!(json["snsTopics"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    // --- sns_topic (bare passthrough) ---

    #[tokio::test]
    async fn sns_topic_maps_attributes() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetTopicAttributes&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}"),
            ),
            xml_response(
                200,
                "<GetTopicAttributesResponse><GetTopicAttributesResult><Attributes>\
                 <entry><key>DisplayName</key><value>My Topic</value></entry>\
                 </Attributes></GetTopicAttributesResult></GetTopicAttributesResponse>",
            ),
        )]);
        let schema = build_query_schema(SnsQuery)
            .data(SnsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ snsTopic(topicArn: "{TOPIC_ARN}") {{ arn displayName }} }}"#
            ))
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["snsTopic"]["arn"], TOPIC_ARN);
        assert_eq!(json["snsTopic"]["displayName"], "My Topic");
        http_client.relaxed_requests_match();
    }

    // --- sns_subscriptions (bare passthrough, branches on topicArn arg) ---

    #[tokio::test]
    async fn sns_subscriptions_lists_all_when_no_topic_arn() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListSubscriptions&Version=2010-03-31"),
            xml_response(
                200,
                format!(
                    "<ListSubscriptionsResponse><ListSubscriptionsResult><Subscriptions>\
                     <member><SubscriptionArn>sub-1</SubscriptionArn><Owner>123456789012</Owner>\
                     <Protocol>email</Protocol><Endpoint>a@example.com</Endpoint>\
                     <TopicArn>{TOPIC_ARN}</TopicArn></member>\
                     </Subscriptions></ListSubscriptionsResult></ListSubscriptionsResponse>"
                ),
            ),
        )]);
        let schema = build_query_schema(SnsQuery)
            .data(SnsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ snsSubscriptions { items { subscriptionArn owner protocol endpoint topicArn } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["snsSubscriptions"]["items"][0];
        assert_eq!(item["subscriptionArn"], "sub-1");
        assert_eq!(item["owner"], "123456789012");
        assert_eq!(item["protocol"], "email");
        assert_eq!(item["endpoint"], "a@example.com");
        assert_eq!(item["topicArn"], TOPIC_ARN);
        assert!(json["snsSubscriptions"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn sns_subscriptions_filters_by_topic_arn() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=ListSubscriptionsByTopic&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}"),
            ),
            xml_response(
                200,
                "<ListSubscriptionsByTopicResponse><ListSubscriptionsByTopicResult><Subscriptions>\
                 <member><SubscriptionArn>sub-2</SubscriptionArn></member>\
                 </Subscriptions></ListSubscriptionsByTopicResult></ListSubscriptionsByTopicResponse>",
            ),
        )]);
        let schema = build_query_schema(SnsQuery)
            .data(SnsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(format!(
                r#"{{ snsSubscriptions(topicArn: "{TOPIC_ARN}") {{ items {{ subscriptionArn }} nextToken }} }}"#
            ))
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["snsSubscriptions"]["items"][0]["subscriptionArn"], "sub-2");
        assert!(json["snsSubscriptions"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
