use async_graphql::SimpleObject;
use aws_sdk_health::types::Event;

#[derive(SimpleObject, Clone)]
pub struct HealthEvent {
    pub arn: String,
    pub service: Option<String>,
    pub event_type_code: Option<String>,
    pub event_type_category: Option<String>,
    pub region: Option<String>,
    pub status: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub last_updated_time: Option<String>,
}

impl From<Event> for HealthEvent {
    fn from(e: Event) -> Self {
        Self {
            arn: e.arn().unwrap_or_default().to_string(),
            service: e.service().map(|s| s.to_string()),
            event_type_code: e.event_type_code().map(|s| s.to_string()),
            event_type_category: e.event_type_category().map(|c| c.as_str().to_string()),
            region: e.region().map(|s| s.to_string()),
            status: e.status_code().map(|s| s.as_str().to_string()),
            start_time: e.start_time().map(|t| t.to_string()),
            end_time: e.end_time().map(|t| t.to_string()),
            last_updated_time: e.last_updated_time().map(|t| t.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_event_fields() {
        let event = HealthEvent {
            arn: "arn:aws:health:us-east-1::event/EC2/AWS_EC2_OPERATIONAL_ISSUE/123".to_string(),
            service: Some("EC2".to_string()),
            event_type_code: Some("AWS_EC2_OPERATIONAL_ISSUE".to_string()),
            event_type_category: Some("issue".to_string()),
            region: Some("us-east-1".to_string()),
            status: Some("open".to_string()),
            start_time: Some("2024-01-15T10:00:00Z".to_string()),
            end_time: None,
            last_updated_time: Some("2024-01-15T11:00:00Z".to_string()),
        };
        assert_eq!(
            event.arn,
            "arn:aws:health:us-east-1::event/EC2/AWS_EC2_OPERATIONAL_ISSUE/123"
        );
        assert_eq!(event.service, Some("EC2".to_string()));
        assert_eq!(event.event_type_category, Some("issue".to_string()));
        assert_eq!(event.status, Some("open".to_string()));
        assert!(event.end_time.is_none());
        assert!(event.last_updated_time.is_some());
    }

    #[test]
    fn test_health_event_minimal() {
        let event = HealthEvent {
            arn: "arn:aws:health:::event/test".to_string(),
            service: None,
            event_type_code: None,
            event_type_category: None,
            region: None,
            status: None,
            start_time: None,
            end_time: None,
            last_updated_time: None,
        };
        assert_eq!(event.arn, "arn:aws:health:::event/test");
        assert!(event.service.is_none());
        assert!(event.status.is_none());
    }

    #[test]
    fn test_health_event_from_sdk() {
        let sdk_event = aws_sdk_health::types::Event::builder()
            .arn("arn:aws:health:us-east-1::event/RDS/AWS_RDS_MAINTENANCE/456")
            .service("RDS")
            .event_type_code("AWS_RDS_MAINTENANCE")
            .event_type_category(aws_sdk_health::types::EventTypeCategory::ScheduledChange)
            .region("us-east-1")
            .status_code(aws_sdk_health::types::EventStatusCode::Upcoming)
            .build();
        let result = HealthEvent::from(sdk_event);
        assert_eq!(
            result.arn,
            "arn:aws:health:us-east-1::event/RDS/AWS_RDS_MAINTENANCE/456"
        );
        assert_eq!(result.service, Some("RDS".to_string()));
        assert_eq!(result.event_type_code, Some("AWS_RDS_MAINTENANCE".to_string()));
        assert_eq!(result.region, Some("us-east-1".to_string()));
        assert_eq!(result.status, Some("upcoming".to_string()));
        assert!(result.start_time.is_none());
    }

    #[test]
    fn test_health_event_from_sdk_empty() {
        let sdk_event = aws_sdk_health::types::Event::builder().build();
        let result = HealthEvent::from(sdk_event);
        assert_eq!(result.arn, "");
        assert!(result.service.is_none());
        assert!(result.status.is_none());
    }
}
