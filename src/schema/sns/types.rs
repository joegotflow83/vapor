use std::collections::HashMap;

use async_graphql::SimpleObject;

/// An SNS topic with its metadata and attributes.
#[derive(SimpleObject, Clone)]
pub struct SnsTopic {
    pub arn: String,
    pub display_name: Option<String>,
    pub owner: Option<String>,
    pub subscriptions_confirmed: Option<i64>,
    pub subscriptions_deleted: Option<i64>,
    pub subscriptions_pending: Option<i64>,
    pub kms_master_key_id: Option<String>,
    pub fifo_topic: Option<bool>,
    pub content_based_deduplication: Option<bool>,
    /// Raw JSON string.
    pub effective_delivery_policy: Option<String>,
    /// Raw IAM policy JSON string.
    pub policy: Option<String>,
}

impl SnsTopic {
    pub fn from_attrs(arn: String, attrs: HashMap<String, String>) -> Self {
        let parse_i64 = |key: &str| -> Option<i64> {
            attrs.get(key).and_then(|v| v.parse::<i64>().ok())
        };
        let parse_bool = |key: &str| -> Option<bool> {
            attrs.get(key).map(|v| v == "true")
        };

        Self {
            arn,
            display_name: attrs.get("DisplayName").map(|s| s.to_string()),
            owner: attrs.get("Owner").map(|s| s.to_string()),
            subscriptions_confirmed: parse_i64("SubscriptionsConfirmed"),
            subscriptions_deleted: parse_i64("SubscriptionsDeleted"),
            subscriptions_pending: parse_i64("SubscriptionsPending"),
            kms_master_key_id: attrs.get("KmsMasterKeyId").map(|s| s.to_string()),
            fifo_topic: parse_bool("FifoTopic"),
            content_based_deduplication: parse_bool("ContentBasedDeduplication"),
            effective_delivery_policy: attrs.get("EffectiveDeliveryPolicy").map(|s| s.to_string()),
            policy: attrs.get("Policy").map(|s| s.to_string()),
        }
    }
}

/// An SNS subscription.
#[derive(SimpleObject, Clone)]
pub struct SnsSubscription {
    pub subscription_arn: Option<String>,
    pub owner: Option<String>,
    /// http | https | email | email-json | sms | sqs | lambda | ...
    pub protocol: Option<String>,
    pub endpoint: Option<String>,
    pub topic_arn: Option<String>,
}

impl From<aws_sdk_sns::types::Subscription> for SnsSubscription {
    fn from(s: aws_sdk_sns::types::Subscription) -> Self {
        Self {
            subscription_arn: s.subscription_arn().map(|v| v.to_string()),
            owner: s.owner().map(|v| v.to_string()),
            protocol: s.protocol().map(|v| v.to_string()),
            endpoint: s.endpoint().map(|v| v.to_string()),
            topic_arn: s.topic_arn().map(|v| v.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_attrs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn test_sns_topic_full_attrs() {
        let attrs = make_attrs(&[
            ("DisplayName", "My Topic"),
            ("Owner", "123456789012"),
            ("SubscriptionsConfirmed", "5"),
            ("SubscriptionsDeleted", "1"),
            ("SubscriptionsPending", "2"),
            ("KmsMasterKeyId", "alias/aws/sns"),
            ("FifoTopic", "false"),
            ("ContentBasedDeduplication", "false"),
            ("EffectiveDeliveryPolicy", r#"{"http":{}}"#),
            ("Policy", r#"{"Version":"2012-10-17"}"#),
        ]);
        let topic = SnsTopic::from_attrs("arn:aws:sns:us-east-1:123:my-topic".to_string(), attrs);

        assert_eq!(topic.arn, "arn:aws:sns:us-east-1:123:my-topic");
        assert_eq!(topic.display_name, Some("My Topic".to_string()));
        assert_eq!(topic.owner, Some("123456789012".to_string()));
        assert_eq!(topic.subscriptions_confirmed, Some(5));
        assert_eq!(topic.subscriptions_deleted, Some(1));
        assert_eq!(topic.subscriptions_pending, Some(2));
        assert_eq!(topic.kms_master_key_id, Some("alias/aws/sns".to_string()));
        assert_eq!(topic.fifo_topic, Some(false));
        assert_eq!(topic.content_based_deduplication, Some(false));
        assert!(topic.effective_delivery_policy.is_some());
        assert!(topic.policy.is_some());
    }

    #[test]
    fn test_sns_topic_empty_attrs() {
        let topic = SnsTopic::from_attrs("arn:aws:sns:us-east-1:123:empty".to_string(), HashMap::new());
        assert_eq!(topic.arn, "arn:aws:sns:us-east-1:123:empty");
        assert_eq!(topic.display_name, None);
        assert_eq!(topic.owner, None);
        assert_eq!(topic.subscriptions_confirmed, None);
        assert_eq!(topic.subscriptions_deleted, None);
        assert_eq!(topic.subscriptions_pending, None);
        assert_eq!(topic.kms_master_key_id, None);
        assert_eq!(topic.fifo_topic, None);
        assert_eq!(topic.content_based_deduplication, None);
        assert_eq!(topic.effective_delivery_policy, None);
        assert_eq!(topic.policy, None);
    }

    #[test]
    fn test_sns_topic_fifo_true() {
        let attrs = make_attrs(&[
            ("FifoTopic", "true"),
            ("ContentBasedDeduplication", "true"),
        ]);
        let topic = SnsTopic::from_attrs("arn:aws:sns:us-east-1:123:my-topic.fifo".to_string(), attrs);
        assert_eq!(topic.fifo_topic, Some(true));
        assert_eq!(topic.content_based_deduplication, Some(true));
    }

    #[test]
    fn test_sns_topic_invalid_int_is_none() {
        let attrs = make_attrs(&[("SubscriptionsConfirmed", "not-a-number")]);
        let topic = SnsTopic::from_attrs("arn:aws:sns:us-east-1:123:t".to_string(), attrs);
        assert_eq!(topic.subscriptions_confirmed, None);
    }

    #[test]
    fn test_sns_subscription_from_sdk() {
        let sdk = aws_sdk_sns::types::Subscription::builder()
            .subscription_arn("arn:aws:sns:us-east-1:123:topic:sub-id")
            .owner("123456789012")
            .protocol("sqs")
            .endpoint("arn:aws:sqs:us-east-1:123:my-queue")
            .topic_arn("arn:aws:sns:us-east-1:123:topic")
            .build();

        let sub = SnsSubscription::from(sdk);
        assert_eq!(sub.subscription_arn, Some("arn:aws:sns:us-east-1:123:topic:sub-id".to_string()));
        assert_eq!(sub.owner, Some("123456789012".to_string()));
        assert_eq!(sub.protocol, Some("sqs".to_string()));
        assert_eq!(sub.endpoint, Some("arn:aws:sqs:us-east-1:123:my-queue".to_string()));
        assert_eq!(sub.topic_arn, Some("arn:aws:sns:us-east-1:123:topic".to_string()));
    }

    #[test]
    fn test_sns_subscription_empty() {
        let sdk = aws_sdk_sns::types::Subscription::builder().build();
        let sub = SnsSubscription::from(sdk);
        assert_eq!(sub.subscription_arn, None);
        assert_eq!(sub.owner, None);
        assert_eq!(sub.protocol, None);
        assert_eq!(sub.endpoint, None);
        assert_eq!(sub.topic_arn, None);
    }
}
