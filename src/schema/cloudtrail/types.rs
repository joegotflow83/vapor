use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct Trail {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub home_region: Option<String>,
    pub is_multi_region: bool,
    pub is_organization_trail: bool,
    pub s3_bucket_name: Option<String>,
    pub s3_key_prefix: Option<String>,
    pub log_file_validation_enabled: bool,
    pub kms_key_id: Option<String>,
    pub is_logging: bool,
}

impl Trail {
    pub fn from_sdk(trail: &aws_sdk_cloudtrail::types::Trail, is_logging: bool) -> Self {
        Self {
            name: trail.name().map(|s| s.to_string()),
            arn: trail.trail_arn().map(|s| s.to_string()),
            home_region: trail.home_region().map(|s| s.to_string()),
            is_multi_region: trail.is_multi_region_trail().unwrap_or(false),
            is_organization_trail: trail.is_organization_trail().unwrap_or(false),
            s3_bucket_name: trail.s3_bucket_name().map(|s| s.to_string()),
            s3_key_prefix: trail.s3_key_prefix().map(|s| s.to_string()),
            log_file_validation_enabled: trail.log_file_validation_enabled().unwrap_or(false),
            kms_key_id: trail.kms_key_id().map(|s| s.to_string()),
            is_logging,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CloudTrailEvent {
    pub event_id: Option<String>,
    pub event_name: Option<String>,
    pub event_source: Option<String>,
    pub event_time: Option<String>,
    pub username: Option<String>,
    pub access_key_id: Option<String>,
    pub read_only: Option<String>,
    pub resources: Vec<EventResource>,
}

impl From<aws_sdk_cloudtrail::types::Event> for CloudTrailEvent {
    fn from(e: aws_sdk_cloudtrail::types::Event) -> Self {
        let resources = e
            .resources()
            .iter()
            .map(|r| EventResource {
                resource_type: r.resource_type().map(|s| s.to_string()),
                resource_name: r.resource_name().map(|s| s.to_string()),
            })
            .collect();

        Self {
            event_id: e.event_id().map(|s| s.to_string()),
            event_name: e.event_name().map(|s| s.to_string()),
            event_source: e.event_source().map(|s| s.to_string()),
            event_time: e.event_time().map(|t| t.to_string()),
            username: e.username().map(|s| s.to_string()),
            access_key_id: e.access_key_id().map(|s| s.to_string()),
            read_only: e.read_only().map(|s| s.to_string()),
            resources,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct EventResource {
    pub resource_type: Option<String>,
    pub resource_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trail_from_sdk_minimal() {
        let sdk_trail = aws_sdk_cloudtrail::types::Trail::builder().build();
        let trail = Trail::from_sdk(&sdk_trail, false);
        assert!(trail.name.is_none());
        assert!(trail.arn.is_none());
        assert!(!trail.is_multi_region);
        assert!(!trail.is_organization_trail);
        assert!(!trail.log_file_validation_enabled);
        assert!(!trail.is_logging);
    }

    #[test]
    fn test_trail_from_sdk_full() {
        let sdk_trail = aws_sdk_cloudtrail::types::Trail::builder()
            .name("my-trail")
            .trail_arn("arn:aws:cloudtrail:us-east-1:123456789012:trail/my-trail")
            .home_region("us-east-1")
            .is_multi_region_trail(true)
            .is_organization_trail(false)
            .s3_bucket_name("my-bucket")
            .s3_key_prefix("prefix")
            .log_file_validation_enabled(true)
            .kms_key_id("arn:aws:kms:us-east-1:123456789012:key/abc")
            .build();
        let trail = Trail::from_sdk(&sdk_trail, true);
        assert_eq!(trail.name, Some("my-trail".to_string()));
        assert_eq!(
            trail.arn,
            Some("arn:aws:cloudtrail:us-east-1:123456789012:trail/my-trail".to_string())
        );
        assert_eq!(trail.home_region, Some("us-east-1".to_string()));
        assert!(trail.is_multi_region);
        assert!(!trail.is_organization_trail);
        assert_eq!(trail.s3_bucket_name, Some("my-bucket".to_string()));
        assert_eq!(trail.s3_key_prefix, Some("prefix".to_string()));
        assert!(trail.log_file_validation_enabled);
        assert!(trail.is_logging);
    }

    #[test]
    fn test_event_from_sdk() {
        let resource = aws_sdk_cloudtrail::types::Resource::builder()
            .resource_type("AWS::S3::Bucket")
            .resource_name("my-bucket")
            .build();
        let event = aws_sdk_cloudtrail::types::Event::builder()
            .event_id("evt-123")
            .event_name("PutObject")
            .event_source("s3.amazonaws.com")
            .username("admin")
            .access_key_id("AKIA1234")
            .read_only("false")
            .resources(resource)
            .build();
        let result = CloudTrailEvent::from(event);
        assert_eq!(result.event_id, Some("evt-123".to_string()));
        assert_eq!(result.event_name, Some("PutObject".to_string()));
        assert_eq!(result.event_source, Some("s3.amazonaws.com".to_string()));
        assert_eq!(result.username, Some("admin".to_string()));
        assert_eq!(result.access_key_id, Some("AKIA1234".to_string()));
        assert_eq!(result.read_only, Some("false".to_string()));
        assert_eq!(result.resources.len(), 1);
        assert_eq!(
            result.resources[0].resource_type,
            Some("AWS::S3::Bucket".to_string())
        );
        assert_eq!(
            result.resources[0].resource_name,
            Some("my-bucket".to_string())
        );
    }

    #[test]
    fn test_event_from_sdk_empty_resources() {
        let event = aws_sdk_cloudtrail::types::Event::builder()
            .event_id("evt-456")
            .build();
        let result = CloudTrailEvent::from(event);
        assert_eq!(result.event_id, Some("evt-456".to_string()));
        assert!(result.resources.is_empty());
    }
}
