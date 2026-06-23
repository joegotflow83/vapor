use async_graphql::SimpleObject;
use aws_sdk_kinesis::types::{
    Shard as SdkShard,
    StreamDescriptionSummary as SdkStreamSummary,
    Tag as SdkKinesisTag,
};

#[derive(SimpleObject, Clone)]
#[graphql(name = "KinesisTag")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl From<SdkKinesisTag> for Tag {
    fn from(t: SdkKinesisTag) -> Self {
        Self {
            key: t.key().to_string(),
            value: t.value().unwrap_or_default().to_string(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DataStream {
    pub name: String,
    pub arn: Option<String>,
    pub status: Option<String>,
    pub stream_mode: Option<String>,
    pub shard_count: Option<i32>,
    pub retention_period_hours: Option<i32>,
    pub encryption_type: Option<String>,
    pub key_id: Option<String>,
    pub created_at: Option<String>,
    pub tags: Vec<Tag>,
}

impl DataStream {
    pub fn from_summary(s: SdkStreamSummary, tags: Vec<Tag>) -> Self {
        Self {
            name: s.stream_name().to_string(),
            arn: Some(s.stream_arn().to_string()),
            status: Some(s.stream_status().as_str().to_string()),
            stream_mode: s.stream_mode_details().map(|m| m.stream_mode().as_str().to_string()),
            shard_count: Some(s.open_shard_count() as i32),
            retention_period_hours: Some(s.retention_period_hours() as i32),
            encryption_type: s.encryption_type().map(|e| e.as_str().to_string()),
            key_id: s.key_id().map(|k| k.to_string()),
            created_at: Some(s.stream_creation_timestamp().to_string()),
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Shard {
    pub shard_id: String,
    pub parent_shard_id: Option<String>,
    pub starting_hash_key: Option<String>,
    pub ending_hash_key: Option<String>,
    pub starting_sequence_number: Option<String>,
    pub ending_sequence_number: Option<String>,
}

impl From<SdkShard> for Shard {
    fn from(s: SdkShard) -> Self {
        let (starting_hash_key, ending_hash_key) = match s.hash_key_range() {
            Some(r) => (
                Some(r.starting_hash_key().to_string()),
                Some(r.ending_hash_key().to_string()),
            ),
            None => (None, None),
        };
        let (starting_sequence_number, ending_sequence_number) = match s.sequence_number_range() {
            Some(r) => (
                Some(r.starting_sequence_number().to_string()),
                r.ending_sequence_number().map(|v| v.to_string()),
            ),
            None => (None, None),
        };
        Self {
            shard_id: s.shard_id().to_string(),
            parent_shard_id: s.parent_shard_id().map(|v| v.to_string()),
            starting_hash_key,
            ending_hash_key,
            starting_sequence_number,
            ending_sequence_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_from_kinesis() {
        let sdk_tag = SdkKinesisTag::builder().key("env").value("prod").build().unwrap();
        let tag = Tag::from(sdk_tag);
        assert_eq!(tag.key, "env");
        assert_eq!(tag.value, "prod");
    }

    #[test]
    fn test_shard_from_sdk() {
        let hash_range = aws_sdk_kinesis::types::HashKeyRange::builder()
            .starting_hash_key("0")
            .ending_hash_key("170141183460469231731687303715884105727")
            .build()
            .unwrap();
        let seq_range = aws_sdk_kinesis::types::SequenceNumberRange::builder()
            .starting_sequence_number("49600000000000000000")
            .build()
            .unwrap();
        let sdk_shard = SdkShard::builder()
            .shard_id("shardId-000000000000")
            .hash_key_range(hash_range)
            .sequence_number_range(seq_range)
            .build()
            .unwrap();
        let shard = Shard::from(sdk_shard);
        assert_eq!(shard.shard_id, "shardId-000000000000");
        assert_eq!(shard.starting_hash_key, Some("0".to_string()));
        assert_eq!(
            shard.ending_hash_key,
            Some("170141183460469231731687303715884105727".to_string())
        );
        assert_eq!(shard.starting_sequence_number, Some("49600000000000000000".to_string()));
        assert!(shard.ending_sequence_number.is_none());
        assert!(shard.parent_shard_id.is_none());
    }

    #[test]
    fn test_shard_minimal() {
        let hash_range = aws_sdk_kinesis::types::HashKeyRange::builder()
            .starting_hash_key("0")
            .ending_hash_key("0")
            .build()
            .unwrap();
        let seq_range = aws_sdk_kinesis::types::SequenceNumberRange::builder()
            .starting_sequence_number("0")
            .build()
            .unwrap();
        let sdk_shard = SdkShard::builder()
            .shard_id("shardId-000000000001")
            .hash_key_range(hash_range)
            .sequence_number_range(seq_range)
            .build()
            .unwrap();
        let shard = Shard::from(sdk_shard);
        assert_eq!(shard.shard_id, "shardId-000000000001");
        assert!(shard.parent_shard_id.is_none());
    }

    #[test]
    fn test_data_stream_from_summary() {
        let summary = SdkStreamSummary::builder()
            .stream_name("my-stream")
            .stream_arn("arn:aws:kinesis:us-east-1:123456789012:stream/my-stream")
            .stream_status(aws_sdk_kinesis::types::StreamStatus::Active)
            .retention_period_hours(24)
            .open_shard_count(4)
            .stream_creation_timestamp(aws_sdk_kinesis::primitives::DateTime::from_secs(1700000000))
            .enhanced_monitoring(aws_sdk_kinesis::types::EnhancedMetrics::builder().build())
            .build()
            .unwrap();
        let ds = DataStream::from_summary(summary, vec![]);
        assert_eq!(ds.name, "my-stream");
        assert_eq!(ds.arn, Some("arn:aws:kinesis:us-east-1:123456789012:stream/my-stream".to_string()));
        assert_eq!(ds.status, Some("ACTIVE".to_string()));
        assert_eq!(ds.shard_count, Some(4));
        assert_eq!(ds.retention_period_hours, Some(24));
        assert!(ds.tags.is_empty());
    }

}
