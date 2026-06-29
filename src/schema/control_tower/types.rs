use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct ControlTowerLandingZone {
    pub arn: Option<String>,
    pub version: String,
    pub latest_available_version: Option<String>,
    pub status: Option<String>,
    pub drift_status: Option<String>,
}

impl From<aws_sdk_controltower::types::LandingZoneDetail> for ControlTowerLandingZone {
    fn from(l: aws_sdk_controltower::types::LandingZoneDetail) -> Self {
        Self {
            arn: l.arn().map(|s| s.to_string()),
            version: l.version().to_string(),
            latest_available_version: l.latest_available_version().map(|s| s.to_string()),
            status: l.status().map(|s| s.as_str().to_string()),
            drift_status: l
                .drift_status()
                .and_then(|d| d.status())
                .map(|s| s.as_str().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct EnabledControlStatus {
    pub status: Option<String>,
    pub last_operation_identifier: Option<String>,
}

impl From<aws_sdk_controltower::types::EnablementStatusSummary> for EnabledControlStatus {
    fn from(s: aws_sdk_controltower::types::EnablementStatusSummary) -> Self {
        Self {
            status: s.status().map(|st| st.as_str().to_string()),
            last_operation_identifier: s.last_operation_identifier().map(|m| m.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct EnabledControl {
    pub arn: Option<String>,
    pub control_identifier: Option<String>,
    pub target_identifier: Option<String>,
    pub status_summary: Option<EnabledControlStatus>,
}

impl From<aws_sdk_controltower::types::EnabledControlSummary> for EnabledControl {
    fn from(e: aws_sdk_controltower::types::EnabledControlSummary) -> Self {
        Self {
            arn: e.arn().map(|s| s.to_string()),
            control_identifier: e.control_identifier().map(|s| s.to_string()),
            target_identifier: e.target_identifier().map(|s| s.to_string()),
            status_summary: e
                .status_summary()
                .cloned()
                .map(EnabledControlStatus::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_landing_zone() -> aws_sdk_controltower::types::LandingZoneDetail {
        aws_sdk_controltower::types::LandingZoneDetail::builder()
            .version("3.0")
            .manifest(aws_smithy_types::Document::Null)
            .build()
            .unwrap()
    }

    #[test]
    fn test_control_tower_landing_zone_from_minimal() {
        let result = ControlTowerLandingZone::from(minimal_landing_zone());
        assert!(result.arn.is_none());
        assert_eq!(result.version, "3.0");
        assert!(result.status.is_none());
        assert!(result.drift_status.is_none());
    }

    #[test]
    fn test_control_tower_landing_zone_from_with_status() {
        let detail = aws_sdk_controltower::types::LandingZoneDetail::builder()
            .arn("arn:aws:controltower:us-east-1::landingzone/ABC123")
            .version("3.3")
            .latest_available_version("3.3")
            .status(aws_sdk_controltower::types::LandingZoneStatus::Active)
            .manifest(aws_smithy_types::Document::Null)
            .build()
            .unwrap();
        let result = ControlTowerLandingZone::from(detail);
        assert_eq!(
            result.arn,
            Some("arn:aws:controltower:us-east-1::landingzone/ABC123".to_string())
        );
        assert_eq!(result.version, "3.3");
        assert_eq!(result.latest_available_version, Some("3.3".to_string()));
        assert_eq!(result.status, Some("ACTIVE".to_string()));
    }

    #[test]
    fn test_control_tower_landing_zone_processing_status() {
        let detail = aws_sdk_controltower::types::LandingZoneDetail::builder()
            .version("3.0")
            .status(aws_sdk_controltower::types::LandingZoneStatus::Processing)
            .manifest(aws_smithy_types::Document::Null)
            .build()
            .unwrap();
        let result = ControlTowerLandingZone::from(detail);
        assert_eq!(result.status, Some("PROCESSING".to_string()));
    }

    #[test]
    fn test_enabled_control_status_from_full() {
        let summary = aws_sdk_controltower::types::EnablementStatusSummary::builder()
            .status(aws_sdk_controltower::types::EnablementStatus::Succeeded)
            .last_operation_identifier("op-abc123")
            .build();
        let result = EnabledControlStatus::from(summary);
        assert_eq!(result.status, Some("SUCCEEDED".to_string()));
        assert_eq!(
            result.last_operation_identifier,
            Some("op-abc123".to_string())
        );
    }

    #[test]
    fn test_enabled_control_status_from_minimal() {
        let summary = aws_sdk_controltower::types::EnablementStatusSummary::builder().build();
        let result = EnabledControlStatus::from(summary);
        assert!(result.status.is_none());
        assert!(result.last_operation_identifier.is_none());
    }

    #[test]
    fn test_enabled_control_from_minimal() {
        let summary = aws_sdk_controltower::types::EnabledControlSummary::builder().build();
        let result = EnabledControl::from(summary);
        assert!(result.arn.is_none());
        assert!(result.control_identifier.is_none());
        assert!(result.target_identifier.is_none());
        assert!(result.status_summary.is_none());
    }

    #[test]
    fn test_enabled_control_from_full() {
        let summary = aws_sdk_controltower::types::EnabledControlSummary::builder()
            .arn("arn:aws:controltower:us-east-1::enabledcontrol/ABC123")
            .control_identifier("arn:aws:controlcatalog:::control/xyz789")
            .target_identifier("arn:aws:organizations::123456789012:ou/o-abc/ou-abc-xyz")
            .build();
        let result = EnabledControl::from(summary);
        assert_eq!(
            result.arn,
            Some("arn:aws:controltower:us-east-1::enabledcontrol/ABC123".to_string())
        );
        assert_eq!(
            result.control_identifier,
            Some("arn:aws:controlcatalog:::control/xyz789".to_string())
        );
        assert_eq!(
            result.target_identifier,
            Some("arn:aws:organizations::123456789012:ou/o-abc/ou-abc-xyz".to_string())
        );
        assert!(result.status_summary.is_none());
    }
}
