use aws_config::SdkConfig;
use std::collections::HashMap;

use crate::error::VaporError;
use crate::aws::pagination::apply_limit;

pub struct SnsClient {
    inner: aws_sdk_sns::Client,
}

impl SnsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sns::Client::new(config),
        }
    }

    /// List all topic ARNs, then fetch attributes for each. `limit` caps the
    /// ARN list before the per-topic attribute fetch, avoiding unnecessary
    /// `get_topic_attributes` calls. `ListTopics` has no `max_results`-
    /// equivalent input field (only `next_token`, verified against pinned
    /// `aws-sdk-sns` 1.104.0's `_list_topics_input.rs`), so `limit` can only
    /// be enforced via client-side `apply_limit` truncation — when it trips
    /// mid-page, the returned `next_token` is AWS's *next*-page token,
    /// permanently skipping whatever was truncated off the current page
    /// (same caveat as `cost_explorer.rs`/`polly.rs`).
    pub async fn list_topics_with_attributes(
        &self,
        next_token: Option<String>,
        limit: Option<i32>,
    ) -> Result<(Vec<(String, HashMap<String, String>)>, Option<String>), VaporError> {
        let mut arns: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_topics();
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            arns.extend(
                output
                    .topics
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|t| t.topic_arn),
            );
            token = output.next_token;

            if apply_limit(&mut arns, limit) {
                break;
            }
            if token.is_none() {
                break;
            }
        }

        let mut result = Vec::with_capacity(arns.len());
        for arn in arns {
            let attrs = self.get_topic_attributes(&arn).await?;
            result.push((arn, attrs));
        }

        Ok((result, token))
    }

    /// Fetch attributes for a single topic ARN.
    pub async fn get_topic_attributes(
        &self,
        topic_arn: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .get_topic_attributes()
            .topic_arn(topic_arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output.attributes().cloned().unwrap_or_default())
    }

    /// List subscriptions, optionally filtered by topic ARN. Neither
    /// `ListSubscriptions` nor `ListSubscriptionsByTopic` has a
    /// `max_results`-equivalent input field (only `next_token`, verified
    /// against pinned `aws-sdk-sns` 1.104.0's `_list_subscriptions_input.rs`/
    /// `_list_subscriptions_by_topic_input.rs`), so `limit` can only be
    /// enforced via client-side `apply_limit` truncation (same caveat as
    /// `list_topics_with_attributes` above).
    pub async fn list_subscriptions(
        &self,
        topic_arn: Option<&str>,
        next_token: Option<String>,
        limit: Option<i32>,
    ) -> Result<(Vec<aws_sdk_sns::types::Subscription>, Option<String>), VaporError> {
        let mut all: Vec<aws_sdk_sns::types::Subscription> = Vec::new();
        let mut token = next_token;

        loop {
            let (subscriptions, next) = if let Some(arn) = topic_arn {
                let mut req = self.inner.list_subscriptions_by_topic().topic_arn(arn);
                if let Some(t) = &token {
                    req = req.next_token(t);
                }
                let output = req.send().await.map_err(crate::error::sdk_err)?;
                (output.subscriptions, output.next_token)
            } else {
                let mut req = self.inner.list_subscriptions();
                if let Some(t) = &token {
                    req = req.next_token(t);
                }
                let output = req.send().await.map_err(crate::error::sdk_err)?;
                (output.subscriptions, output.next_token)
            };

            all.extend(subscriptions.unwrap_or_default());
            token = next;

            if apply_limit(&mut all, limit) {
                break;
            }
            if token.is_none() {
                break;
            }
        }

        Ok((all, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://sns.us-east-1.amazonaws.com/";
    const TOPIC_ARN: &str = "arn:aws:sns:us-east-1:123456789012:my-topic";
    const TOPIC_ARN_ENC: &str = "arn%3Aaws%3Asns%3Aus-east-1%3A123456789012%3Amy-topic";

    #[tokio::test]
    async fn list_topics_with_attributes_happy_path_no_limit_no_token() {
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
                     </Attributes></GetTopicAttributesResult></GetTopicAttributesResponse>",
                ),
            ),
        ]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (result, token) = client.list_topics_with_attributes(None, None).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, TOPIC_ARN);
        assert_eq!(result[0].1.get("DisplayName"), Some(&"My Topic".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topics_with_attributes_resumes_from_provided_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListTopics&Version=2010-03-31&NextToken=cursor-a"),
            xml_response(
                200,
                "<ListTopicsResponse><ListTopicsResult><Topics>\
                 </Topics></ListTopicsResult></ListTopicsResponse>",
            ),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (result, token) = client
            .list_topics_with_attributes(Some("cursor-a".to_string()), None)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topics_with_attributes_stops_at_limit_truncates_client_side() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListTopics&Version=2010-03-31"),
                xml_response(
                    200,
                    "<ListTopicsResponse><ListTopicsResult><Topics>\
                     <member><TopicArn>arn:a</TopicArn></member>\
                     <member><TopicArn>arn:b</TopicArn></member>\
                     <member><TopicArn>arn:c</TopicArn></member>\
                     </Topics><NextToken>page2</NextToken></ListTopicsResult></ListTopicsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetTopicAttributes&Version=2010-03-31&TopicArn=arn%3Aa",
                ),
                xml_response(
                    200,
                    "<GetTopicAttributesResponse><GetTopicAttributesResult><Attributes>\
                     </Attributes></GetTopicAttributesResult></GetTopicAttributesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetTopicAttributes&Version=2010-03-31&TopicArn=arn%3Ab",
                ),
                xml_response(
                    200,
                    "<GetTopicAttributesResponse><GetTopicAttributesResult><Attributes>\
                     </Attributes></GetTopicAttributesResult></GetTopicAttributesResponse>",
                ),
            ),
        ]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (result, token) = client.list_topics_with_attributes(None, Some(2)).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "arn:a");
        assert_eq!(result[1].0, "arn:b");
        // apply_limit trips mid-page: the truncated 3rd item ("arn:c") is
        // dropped, but the returned token is still AWS's *next*-page token,
        // not a marker that resumes from the truncation point.
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topics_with_attributes_propagates_list_error() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListTopics&Version=2010-03-31"),
            xml_error_response("InvalidParameter", "bad token"),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_topics_with_attributes(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameter".to_string()));
                assert_eq!(message, "bad token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_topics_with_attributes_propagates_attribute_fetch_error() {
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
                xml_error_response("NotFound", "topic not found"),
            ),
        ]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        // The per-topic attribute fetch uses `?`, not `.ok()` — a fan-out
        // failure must propagate, not be swallowed into an empty map.
        let err = client.list_topics_with_attributes(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFound".to_string()));
                assert_eq!(message, "topic not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_topic_attributes_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetTopicAttributes&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}"),
            ),
            xml_response(
                200,
                "<GetTopicAttributesResponse><GetTopicAttributesResult><Attributes>\
                 <entry><key>Owner</key><value>123456789012</value></entry>\
                 </Attributes></GetTopicAttributesResult></GetTopicAttributesResponse>",
            ),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let attrs = client.get_topic_attributes(TOPIC_ARN).await.unwrap();

        assert_eq!(attrs.get("Owner"), Some(&"123456789012".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_topic_attributes_missing_map_returns_empty() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetTopicAttributes&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}"),
            ),
            xml_response(
                200,
                "<GetTopicAttributesResponse><GetTopicAttributesResult>\
                 </GetTopicAttributesResult></GetTopicAttributesResponse>",
            ),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let attrs = client.get_topic_attributes(TOPIC_ARN).await.unwrap();

        assert!(attrs.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_topic_attributes_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetTopicAttributes&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}"),
            ),
            xml_error_response("AuthorizationError", "not authorized"),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let err = client.get_topic_attributes(TOPIC_ARN).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AuthorizationError".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_subscriptions_all_happy_path_no_limit_no_token() {
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
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (subs, token) = client.list_subscriptions(None, None, None).await.unwrap();

        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].subscription_arn(), Some("sub-1"));
        assert_eq!(subs[0].owner(), Some("123456789012"));
        assert_eq!(subs[0].protocol(), Some("email"));
        assert_eq!(subs[0].endpoint(), Some("a@example.com"));
        assert_eq!(subs[0].topic_arn(), Some(TOPIC_ARN));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_subscriptions_by_topic_happy_path() {
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
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (subs, token) = client
            .list_subscriptions(Some(TOPIC_ARN), None, None)
            .await
            .unwrap();

        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].subscription_arn(), Some("sub-2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_subscriptions_by_topic_resumes_from_provided_token() {
        // Confirms per-op field order: TopicArn is written before NextToken
        // in `ser_list_subscriptions_by_topic_input_input_input`.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=ListSubscriptionsByTopic&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}&NextToken=cursor-b"),
            ),
            xml_response(
                200,
                "<ListSubscriptionsByTopicResponse><ListSubscriptionsByTopicResult><Subscriptions>\
                 </Subscriptions></ListSubscriptionsByTopicResult></ListSubscriptionsByTopicResponse>",
            ),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (subs, token) = client
            .list_subscriptions(Some(TOPIC_ARN), Some("cursor-b".to_string()), None)
            .await
            .unwrap();

        assert_eq!(subs.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_subscriptions_stops_at_limit_truncates_client_side() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListSubscriptions&Version=2010-03-31"),
            xml_response(
                200,
                "<ListSubscriptionsResponse><ListSubscriptionsResult><Subscriptions>\
                 <member><SubscriptionArn>sub-a</SubscriptionArn></member>\
                 <member><SubscriptionArn>sub-b</SubscriptionArn></member>\
                 <member><SubscriptionArn>sub-c</SubscriptionArn></member>\
                 </Subscriptions><NextToken>page2</NextToken></ListSubscriptionsResult></ListSubscriptionsResponse>",
            ),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (subs, token) = client.list_subscriptions(None, None, Some(2)).await.unwrap();

        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].subscription_arn(), Some("sub-a"));
        assert_eq!(subs[1].subscription_arn(), Some("sub-b"));
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_subscriptions_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListSubscriptions&Version=2010-03-31"),
                xml_response(
                    200,
                    "<ListSubscriptionsResponse><ListSubscriptionsResult><Subscriptions>\
                     <member><SubscriptionArn>sub-1</SubscriptionArn></member>\
                     </Subscriptions><NextToken>p2</NextToken></ListSubscriptionsResult></ListSubscriptionsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListSubscriptions&Version=2010-03-31&NextToken=p2"),
                xml_response(
                    200,
                    "<ListSubscriptionsResponse><ListSubscriptionsResult><Subscriptions>\
                     <member><SubscriptionArn>sub-2</SubscriptionArn></member>\
                     </Subscriptions></ListSubscriptionsResult></ListSubscriptionsResponse>",
                ),
            ),
        ]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let (subs, token) = client.list_subscriptions(None, None, None).await.unwrap();

        assert_eq!(subs.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_subscriptions_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListSubscriptions&Version=2010-03-31"),
            xml_error_response("InternalError", "internal failure"),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let err = client.list_subscriptions(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InternalError".to_string()));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_subscriptions_by_topic_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=ListSubscriptionsByTopic&Version=2010-03-31&TopicArn={TOPIC_ARN_ENC}"),
            ),
            xml_error_response("NotFound", "topic not found"),
        )]);
        let client = SnsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_subscriptions(Some(TOPIC_ARN), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("NotFound".to_string()));
                assert_eq!(message, "topic not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

