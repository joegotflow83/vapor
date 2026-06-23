use async_graphql::SimpleObject;
use aws_sdk_firehose::types::{
    DeliveryStreamDescription as SdkDeliveryStream,
    Tag as SdkTag,
};

#[derive(SimpleObject, Clone)]
pub struct FirehoseTag {
    pub key: String,
    pub value: Option<String>,
}

impl From<SdkTag> for FirehoseTag {
    fn from(t: SdkTag) -> Self {
        Self {
            key: t.key().to_string(),
            value: t.value().map(|v| v.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct FirehoseDeliveryStream {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub stream_type: Option<String>,
    pub create_timestamp: Option<String>,
    pub last_update: Option<String>,
    pub destinations: Vec<String>,
    pub tags: Vec<FirehoseTag>,
}

impl FirehoseDeliveryStream {
    pub fn from_description(d: SdkDeliveryStream, tags: Vec<FirehoseTag>) -> Self {
        let mut destinations = Vec::new();
        for dest in d.destinations() {
            if dest.extended_s3_destination_description().is_some()
                || dest.s3_destination_description().is_some()
            {
                if !destinations.contains(&"S3".to_string()) {
                    destinations.push("S3".to_string());
                }
            }
            if dest.redshift_destination_description().is_some() {
                destinations.push("Redshift".to_string());
            }
            if dest.amazonopensearchservice_destination_description().is_some() {
                destinations.push("OpenSearch".to_string());
            }
            if dest.splunk_destination_description().is_some() {
                destinations.push("Splunk".to_string());
            }
            if dest.http_endpoint_destination_description().is_some() {
                destinations.push("HttpEndpoint".to_string());
            }
        }
        Self {
            name: d.delivery_stream_name().to_string(),
            arn: Some(d.delivery_stream_arn().to_string()),
            status: Some(d.delivery_stream_status().as_str().to_string()),
            stream_type: Some(d.delivery_stream_type().as_str().to_string()),
            create_timestamp: d.create_timestamp().map(|t| t.to_string()),
            last_update: d.last_update_timestamp().map(|t| t.to_string()),
            destinations,
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firehose_tag_with_value() {
        let sdk_tag = SdkTag::builder().key("env").value("prod").build().unwrap();
        let tag = FirehoseTag::from(sdk_tag);
        assert_eq!(tag.key, "env");
        assert_eq!(tag.value, Some("prod".to_string()));
    }

    #[test]
    fn test_firehose_tag_no_value() {
        let sdk_tag = SdkTag::builder().key("flag").build().unwrap();
        let tag = FirehoseTag::from(sdk_tag);
        assert_eq!(tag.key, "flag");
        assert_eq!(tag.value, None);
    }

    #[test]
    fn test_delivery_stream_from_description_minimal() {
        let desc = SdkDeliveryStream::builder()
            .delivery_stream_name("my-stream")
            .delivery_stream_arn("arn:aws:firehose:us-east-1:123456789012:deliverystream/my-stream")
            .delivery_stream_status(aws_sdk_firehose::types::DeliveryStreamStatus::Active)
            .delivery_stream_type(aws_sdk_firehose::types::DeliveryStreamType::DirectPut)
            .version_id("1")
            .set_destinations(Some(vec![]))
            .has_more_destinations(false)
            .build()
            .unwrap();
        let stream = FirehoseDeliveryStream::from_description(desc, vec![]);
        assert_eq!(stream.name, "my-stream");
        assert_eq!(
            stream.arn,
            Some("arn:aws:firehose:us-east-1:123456789012:deliverystream/my-stream".to_string())
        );
        assert_eq!(stream.status, Some("ACTIVE".to_string()));
        assert_eq!(stream.stream_type, Some("DirectPut".to_string()));
        assert!(stream.destinations.is_empty());
        assert!(stream.tags.is_empty());
    }

    #[test]
    fn test_delivery_stream_with_tags() {
        let desc = SdkDeliveryStream::builder()
            .delivery_stream_name("tagged-stream")
            .delivery_stream_arn(
                "arn:aws:firehose:us-east-1:123456789012:deliverystream/tagged-stream",
            )
            .delivery_stream_status(aws_sdk_firehose::types::DeliveryStreamStatus::Active)
            .delivery_stream_type(aws_sdk_firehose::types::DeliveryStreamType::DirectPut)
            .version_id("1")
            .set_destinations(Some(vec![]))
            .has_more_destinations(false)
            .build()
            .unwrap();
        let tags = vec![FirehoseTag {
            key: "team".to_string(),
            value: Some("data".to_string()),
        }];
        let stream = FirehoseDeliveryStream::from_description(desc, tags);
        assert_eq!(stream.tags.len(), 1);
        assert_eq!(stream.tags[0].key, "team");
        assert_eq!(stream.tags[0].value, Some("data".to_string()));
    }
}
