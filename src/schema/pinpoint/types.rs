use async_graphql::SimpleObject;

use crate::aws::pinpoint::{PinpointAppInfo, PinpointCampaignInfo, PinpointSegmentInfo};
use crate::schema::common::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct PinpointApp {
    pub id: Option<String>,
    pub name: Option<String>,
    pub arn: Option<String>,
    pub creation_date: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<PinpointAppInfo> for PinpointApp {
    fn from(i: PinpointAppInfo) -> Self {
        Self {
            id: i.id,
            name: i.name,
            arn: i.arn,
            creation_date: i.creation_date,
            tags: i.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PinpointCampaign {
    pub id: Option<String>,
    pub application_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub creation_date: Option<String>,
    pub last_modified_date: Option<String>,
}

impl From<PinpointCampaignInfo> for PinpointCampaign {
    fn from(i: PinpointCampaignInfo) -> Self {
        Self {
            id: i.id,
            application_id: i.application_id,
            name: i.name,
            status: i.status,
            creation_date: i.creation_date,
            last_modified_date: i.last_modified_date,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct PinpointSegment {
    pub id: Option<String>,
    pub application_id: Option<String>,
    pub name: Option<String>,
    pub segment_type: Option<String>,
    pub creation_date: Option<String>,
    pub last_modified_date: Option<String>,
}

impl From<PinpointSegmentInfo> for PinpointSegment {
    fn from(i: PinpointSegmentInfo) -> Self {
        Self {
            id: i.id,
            application_id: i.application_id,
            name: i.name,
            segment_type: i.segment_type,
            creation_date: i.creation_date,
            last_modified_date: i.last_modified_date,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::pinpoint::{PinpointAppInfo, PinpointCampaignInfo, PinpointSegmentInfo};

    #[test]
    fn test_pinpoint_app_from_full() {
        let info = PinpointAppInfo {
            id: Some("app-abc123".to_string()),
            name: Some("MyApp".to_string()),
            arn: Some("arn:aws:mobiletargeting:us-east-1:123456789:apps/app-abc123".to_string()),
            creation_date: Some("2024-01-15T10:30:00Z".to_string()),
            tags: vec![("env".to_string(), "prod".to_string())],
        };
        let result = PinpointApp::from(info);
        assert_eq!(result.id, Some("app-abc123".to_string()));
        assert_eq!(result.name, Some("MyApp".to_string()));
        assert_eq!(result.arn, Some("arn:aws:mobiletargeting:us-east-1:123456789:apps/app-abc123".to_string()));
        assert_eq!(result.creation_date, Some("2024-01-15T10:30:00Z".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_pinpoint_app_from_minimal() {
        let info = PinpointAppInfo {
            id: None,
            name: None,
            arn: None,
            creation_date: None,
            tags: vec![],
        };
        let result = PinpointApp::from(info);
        assert!(result.id.is_none());
        assert!(result.name.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_pinpoint_campaign_from_full() {
        let info = PinpointCampaignInfo {
            id: Some("campaign-xyz".to_string()),
            application_id: Some("app-abc123".to_string()),
            name: Some("WelcomeCampaign".to_string()),
            status: Some("SCHEDULED".to_string()),
            creation_date: Some("2024-02-01T08:00:00Z".to_string()),
            last_modified_date: Some("2024-02-10T12:00:00Z".to_string()),
        };
        let result = PinpointCampaign::from(info);
        assert_eq!(result.id, Some("campaign-xyz".to_string()));
        assert_eq!(result.application_id, Some("app-abc123".to_string()));
        assert_eq!(result.name, Some("WelcomeCampaign".to_string()));
        assert_eq!(result.status, Some("SCHEDULED".to_string()));
        assert_eq!(result.creation_date, Some("2024-02-01T08:00:00Z".to_string()));
        assert_eq!(result.last_modified_date, Some("2024-02-10T12:00:00Z".to_string()));
    }

    #[test]
    fn test_pinpoint_campaign_from_minimal() {
        let info = PinpointCampaignInfo {
            id: None,
            application_id: None,
            name: None,
            status: None,
            creation_date: None,
            last_modified_date: None,
        };
        let result = PinpointCampaign::from(info);
        assert!(result.id.is_none());
        assert!(result.status.is_none());
    }

    #[test]
    fn test_pinpoint_segment_from_full() {
        let info = PinpointSegmentInfo {
            id: Some("seg-111".to_string()),
            application_id: Some("app-abc123".to_string()),
            name: Some("HighValueUsers".to_string()),
            segment_type: Some("DIMENSIONAL".to_string()),
            creation_date: Some("2024-03-01T09:00:00Z".to_string()),
            last_modified_date: Some("2024-03-15T11:00:00Z".to_string()),
        };
        let result = PinpointSegment::from(info);
        assert_eq!(result.id, Some("seg-111".to_string()));
        assert_eq!(result.application_id, Some("app-abc123".to_string()));
        assert_eq!(result.name, Some("HighValueUsers".to_string()));
        assert_eq!(result.segment_type, Some("DIMENSIONAL".to_string()));
        assert_eq!(result.creation_date, Some("2024-03-01T09:00:00Z".to_string()));
        assert_eq!(result.last_modified_date, Some("2024-03-15T11:00:00Z".to_string()));
    }

    #[test]
    fn test_pinpoint_segment_from_minimal() {
        let info = PinpointSegmentInfo {
            id: None,
            application_id: None,
            name: None,
            segment_type: None,
            creation_date: None,
            last_modified_date: None,
        };
        let result = PinpointSegment::from(info);
        assert!(result.id.is_none());
        assert!(result.segment_type.is_none());
    }

    #[test]
    fn test_pinpoint_app_multiple_tags() {
        let info = PinpointAppInfo {
            id: Some("app-multi".to_string()),
            name: Some("MultiTagApp".to_string()),
            arn: None,
            creation_date: None,
            tags: vec![
                ("env".to_string(), "prod".to_string()),
                ("team".to_string(), "marketing".to_string()),
                ("cost-center".to_string(), "eng-001".to_string()),
            ],
        };
        let result = PinpointApp::from(info);
        assert_eq!(result.tags.len(), 3);
        let env_tag = result.tags.iter().find(|t| t.key == "env").unwrap();
        assert_eq!(env_tag.value, "prod");
    }
}
