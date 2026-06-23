use std::collections::HashMap;

use async_graphql::SimpleObject;

use crate::schema::ec2::types::Tag;

/// An SQS queue with its metadata, attributes, and tags.
#[derive(SimpleObject, Clone)]
pub struct SqsQueue {
    /// The queue URL (primary identifier).
    pub url: String,
    /// The ARN of the queue.
    pub arn: Option<String>,
    /// The queue name, derived from the URL path's last segment.
    pub name: Option<String>,
    pub approximate_number_of_messages: Option<i64>,
    pub approximate_number_of_messages_not_visible: Option<i64>,
    pub approximate_number_of_messages_delayed: Option<i64>,
    pub visibility_timeout_seconds: Option<i64>,
    pub message_retention_period_seconds: Option<i64>,
    pub maximum_message_size: Option<i64>,
    pub delay_seconds: Option<i64>,
    pub receive_message_wait_time_seconds: Option<i64>,
    /// Unix timestamp (seconds) of queue creation, as a string.
    pub created_timestamp: Option<String>,
    /// Unix timestamp (seconds) of last modification, as a string.
    pub last_modified_timestamp: Option<String>,
    /// Raw IAM policy JSON.
    pub policy: Option<String>,
    /// Raw redrive policy JSON.
    pub redrive_policy: Option<String>,
    pub fifo_queue: Option<bool>,
    pub content_based_deduplication: Option<bool>,
    pub kms_master_key_id: Option<String>,
    pub kms_data_key_reuse_period_seconds: Option<i64>,
    pub tags: Vec<Tag>,
}

impl SqsQueue {
    pub fn from_parts(
        url: String,
        attrs: HashMap<String, String>,
        tag_map: HashMap<String, String>,
    ) -> Self {
        let parse_i64 = |key: &str| -> Option<i64> {
            attrs.get(key).and_then(|v| v.parse::<i64>().ok())
        };
        let parse_bool = |key: &str| -> Option<bool> {
            attrs.get(key).map(|v| v == "true")
        };

        let name = url.split('/').next_back().map(|s| s.to_string());
        let arn = attrs.get("QueueArn").map(|s| s.to_string());
        let policy = attrs.get("Policy").map(|s| s.to_string());
        let redrive_policy = attrs.get("RedrivePolicy").map(|s| s.to_string());
        let kms_master_key_id = attrs.get("KmsMasterKeyId").map(|s| s.to_string());
        let created_timestamp = attrs.get("CreatedTimestamp").map(|s| s.to_string());
        let last_modified_timestamp = attrs.get("LastModifiedTimestamp").map(|s| s.to_string());

        let tags = tag_map
            .into_iter()
            .map(|(k, v)| Tag { key: k, value: v })
            .collect();

        Self {
            url,
            arn,
            name,
            approximate_number_of_messages: parse_i64("ApproximateNumberOfMessages"),
            approximate_number_of_messages_not_visible: parse_i64(
                "ApproximateNumberOfMessagesNotVisible",
            ),
            approximate_number_of_messages_delayed: parse_i64(
                "ApproximateNumberOfMessagesDelayed",
            ),
            visibility_timeout_seconds: parse_i64("VisibilityTimeout"),
            message_retention_period_seconds: parse_i64("MessageRetentionPeriod"),
            maximum_message_size: parse_i64("MaximumMessageSize"),
            delay_seconds: parse_i64("DelaySeconds"),
            receive_message_wait_time_seconds: parse_i64("ReceiveMessageWaitTimeSeconds"),
            created_timestamp,
            last_modified_timestamp,
            policy,
            redrive_policy,
            fifo_queue: parse_bool("FifoQueue"),
            content_based_deduplication: parse_bool("ContentBasedDeduplication"),
            kms_master_key_id,
            kms_data_key_reuse_period_seconds: parse_i64("KmsDataKeyReusePeriodSeconds"),
            tags,
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
    fn test_sqs_queue_name_from_url() {
        let url = "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string();
        let q = SqsQueue::from_parts(url, HashMap::new(), HashMap::new());
        assert_eq!(q.name, Some("my-queue".to_string()));
    }

    #[test]
    fn test_sqs_queue_attributes_parsed() {
        let attrs = make_attrs(&[
            ("QueueArn", "arn:aws:sqs:us-east-1:123456789012:my-queue"),
            ("ApproximateNumberOfMessages", "5"),
            ("ApproximateNumberOfMessagesNotVisible", "2"),
            ("ApproximateNumberOfMessagesDelayed", "0"),
            ("VisibilityTimeout", "30"),
            ("MessageRetentionPeriod", "86400"),
            ("MaximumMessageSize", "262144"),
            ("DelaySeconds", "0"),
            ("ReceiveMessageWaitTimeSeconds", "20"),
            ("CreatedTimestamp", "1609459200"),
            ("LastModifiedTimestamp", "1609459200"),
        ]);
        let url = "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string();
        let q = SqsQueue::from_parts(url, attrs, HashMap::new());

        assert_eq!(q.arn, Some("arn:aws:sqs:us-east-1:123456789012:my-queue".to_string()));
        assert_eq!(q.approximate_number_of_messages, Some(5));
        assert_eq!(q.approximate_number_of_messages_not_visible, Some(2));
        assert_eq!(q.approximate_number_of_messages_delayed, Some(0));
        assert_eq!(q.visibility_timeout_seconds, Some(30));
        assert_eq!(q.message_retention_period_seconds, Some(86400));
        assert_eq!(q.maximum_message_size, Some(262144));
        assert_eq!(q.delay_seconds, Some(0));
        assert_eq!(q.receive_message_wait_time_seconds, Some(20));
        assert_eq!(q.created_timestamp, Some("1609459200".to_string()));
        assert_eq!(q.last_modified_timestamp, Some("1609459200".to_string()));
    }

    #[test]
    fn test_sqs_queue_fifo_bool_parsing() {
        let attrs = make_attrs(&[
            ("FifoQueue", "true"),
            ("ContentBasedDeduplication", "false"),
        ]);
        let q = SqsQueue::from_parts(
            "https://sqs.us-east-1.amazonaws.com/123/my-queue.fifo".to_string(),
            attrs,
            HashMap::new(),
        );
        assert_eq!(q.fifo_queue, Some(true));
        assert_eq!(q.content_based_deduplication, Some(false));
    }

    #[test]
    fn test_sqs_queue_tags() {
        let tag_map = make_attrs(&[("env", "prod"), ("team", "platform")]);
        let q = SqsQueue::from_parts(
            "https://sqs.us-east-1.amazonaws.com/123/my-queue".to_string(),
            HashMap::new(),
            tag_map,
        );
        assert_eq!(q.tags.len(), 2);
    }

    #[test]
    fn test_sqs_queue_missing_attrs_are_none() {
        let q = SqsQueue::from_parts(
            "https://sqs.us-east-1.amazonaws.com/123/q".to_string(),
            HashMap::new(),
            HashMap::new(),
        );
        assert_eq!(q.arn, None);
        assert_eq!(q.approximate_number_of_messages, None);
        assert_eq!(q.fifo_queue, None);
        assert_eq!(q.kms_master_key_id, None);
    }

    #[test]
    fn test_sqs_queue_invalid_int_attr_is_none() {
        let attrs = make_attrs(&[("ApproximateNumberOfMessages", "not-a-number")]);
        let q = SqsQueue::from_parts(
            "https://sqs.us-east-1.amazonaws.com/123/q".to_string(),
            attrs,
            HashMap::new(),
        );
        assert_eq!(q.approximate_number_of_messages, None);
    }

    #[test]
    fn test_sqs_queue_policy_and_redrive() {
        let attrs = make_attrs(&[
            ("Policy", r#"{"Version":"2012-10-17"}"#),
            ("RedrivePolicy", r#"{"deadLetterTargetArn":"arn:..."}"#),
        ]);
        let q = SqsQueue::from_parts(
            "https://sqs.us-east-1.amazonaws.com/123/q".to_string(),
            attrs,
            HashMap::new(),
        );
        assert!(q.policy.is_some());
        assert!(q.redrive_policy.is_some());
    }

    #[test]
    fn test_sqs_queue_kms_fields() {
        let attrs = make_attrs(&[
            ("KmsMasterKeyId", "alias/aws/sqs"),
            ("KmsDataKeyReusePeriodSeconds", "300"),
        ]);
        let q = SqsQueue::from_parts(
            "https://sqs.us-east-1.amazonaws.com/123/q".to_string(),
            attrs,
            HashMap::new(),
        );
        assert_eq!(q.kms_master_key_id, Some("alias/aws/sqs".to_string()));
        assert_eq!(q.kms_data_key_reuse_period_seconds, Some(300));
    }
}
