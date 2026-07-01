use async_graphql::SimpleObject;

use crate::aws::ses::{SesAccountInfo, SesConfigSetDetail, SesIdentityInfo};
use crate::schema::common::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct SesIdentity {
    pub identity: String,
    pub identity_type: String,
    pub sending_enabled: bool,
    pub dkim_signing_enabled: Option<bool>,
    pub dkim_status: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<SesIdentityInfo> for SesIdentity {
    fn from(info: SesIdentityInfo) -> Self {
        Self {
            identity: info.identity,
            identity_type: info.identity_type.unwrap_or_default(),
            sending_enabled: info.sending_enabled,
            dkim_signing_enabled: info.dkim_signing_enabled,
            dkim_status: info.dkim_status,
            tags: info
                .tags
                .into_iter()
                .map(|(k, v)| Tag { key: k, value: v })
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SesSendingOptions {
    pub sending_enabled: bool,
}

#[derive(SimpleObject, Clone)]
pub struct SesConfigurationSet {
    pub name: String,
    pub sending_options: Option<SesSendingOptions>,
    pub tags: Vec<Tag>,
}

impl From<SesConfigSetDetail> for SesConfigurationSet {
    fn from(d: SesConfigSetDetail) -> Self {
        Self {
            name: d.name,
            sending_options: d
                .sending_enabled
                .map(|se| SesSendingOptions { sending_enabled: se }),
            tags: d
                .tags
                .into_iter()
                .map(|(k, v)| Tag { key: k, value: v })
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SesEmailTemplate {
    pub template_name: String,
    pub created_timestamp: Option<String>,
}

impl From<aws_sdk_sesv2::types::EmailTemplateMetadata> for SesEmailTemplate {
    fn from(t: aws_sdk_sesv2::types::EmailTemplateMetadata) -> Self {
        Self {
            template_name: t.template_name().unwrap_or_default().to_string(),
            created_timestamp: t.created_timestamp().map(|dt| dt.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SesSuppressedDestination {
    pub email_address: String,
    pub reason: String,
    pub last_update_time: Option<String>,
}

impl From<aws_sdk_sesv2::types::SuppressedDestinationSummary> for SesSuppressedDestination {
    fn from(s: aws_sdk_sesv2::types::SuppressedDestinationSummary) -> Self {
        Self {
            email_address: s.email_address().to_string(),
            reason: s.reason().as_str().to_string(),
            last_update_time: Some(s.last_update_time().to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct SesAccountDetails {
    pub sending_enabled: bool,
    pub sending_quota: Option<f64>,
    pub max_send_rate: Option<f64>,
    pub sent_last_24_hours: Option<f64>,
}

impl From<SesAccountInfo> for SesAccountDetails {
    fn from(info: SesAccountInfo) -> Self {
        Self {
            sending_enabled: info.sending_enabled,
            sending_quota: info.sending_quota,
            max_send_rate: info.max_send_rate,
            sent_last_24_hours: info.sent_last_24_hours,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::ses::{SesAccountInfo, SesConfigSetDetail, SesIdentityInfo};

    #[test]
    fn test_ses_identity_from_minimal() {
        let info = SesIdentityInfo {
            identity: "example.com".to_string(),
            identity_type: None,
            sending_enabled: false,
            dkim_signing_enabled: None,
            dkim_status: None,
            tags: vec![],
        };
        let result = SesIdentity::from(info);
        assert_eq!(result.identity, "example.com");
        assert_eq!(result.identity_type, "");
        assert!(!result.sending_enabled);
        assert!(result.dkim_signing_enabled.is_none());
        assert!(result.dkim_status.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_ses_identity_from_full() {
        let info = SesIdentityInfo {
            identity: "user@example.com".to_string(),
            identity_type: Some("EMAIL_ADDRESS".to_string()),
            sending_enabled: true,
            dkim_signing_enabled: Some(true),
            dkim_status: Some("SUCCESS".to_string()),
            tags: vec![("env".to_string(), "prod".to_string())],
        };
        let result = SesIdentity::from(info);
        assert_eq!(result.identity, "user@example.com");
        assert_eq!(result.identity_type, "EMAIL_ADDRESS");
        assert!(result.sending_enabled);
        assert_eq!(result.dkim_signing_enabled, Some(true));
        assert_eq!(result.dkim_status, Some("SUCCESS".to_string()));
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_ses_configuration_set_from_minimal() {
        let detail = SesConfigSetDetail {
            name: "my-config-set".to_string(),
            sending_enabled: None,
            tags: vec![],
        };
        let result = SesConfigurationSet::from(detail);
        assert_eq!(result.name, "my-config-set");
        assert!(result.sending_options.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_ses_configuration_set_from_with_sending_options() {
        let detail = SesConfigSetDetail {
            name: "active-set".to_string(),
            sending_enabled: Some(true),
            tags: vec![("team".to_string(), "infra".to_string())],
        };
        let result = SesConfigurationSet::from(detail);
        assert_eq!(result.name, "active-set");
        assert!(result.sending_options.is_some());
        assert!(result.sending_options.unwrap().sending_enabled);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "team");
    }

    #[test]
    fn test_ses_email_template_from_minimal() {
        let t = aws_sdk_sesv2::types::EmailTemplateMetadata::builder().build();
        let result = SesEmailTemplate::from(t);
        assert_eq!(result.template_name, "");
        assert!(result.created_timestamp.is_none());
    }

    #[test]
    fn test_ses_email_template_from_full() {
        let t = aws_sdk_sesv2::types::EmailTemplateMetadata::builder()
            .template_name("welcome-email")
            .build();
        let result = SesEmailTemplate::from(t);
        assert_eq!(result.template_name, "welcome-email");
    }

    #[test]
    fn test_ses_suppressed_destination_from_full() {
        // email_address, reason and last_update_time are all required fields.
        let s = aws_sdk_sesv2::types::SuppressedDestinationSummary::builder()
            .email_address("blocked@example.com")
            .reason(aws_sdk_sesv2::types::SuppressionListReason::Bounce)
            .last_update_time(aws_sdk_sesv2::primitives::DateTime::from_secs(1_700_000_000))
            .build()
            .unwrap();
        let result = SesSuppressedDestination::from(s);
        assert_eq!(result.email_address, "blocked@example.com");
        assert_eq!(result.reason, "BOUNCE");
        assert!(result.last_update_time.is_some());
    }

    #[test]
    fn test_ses_account_details_from_minimal() {
        let info = SesAccountInfo {
            sending_enabled: false,
            sending_quota: None,
            max_send_rate: None,
            sent_last_24_hours: None,
        };
        let result = SesAccountDetails::from(info);
        assert!(!result.sending_enabled);
        assert!(result.sending_quota.is_none());
        assert!(result.max_send_rate.is_none());
        assert!(result.sent_last_24_hours.is_none());
    }

    #[test]
    fn test_ses_account_details_from_full() {
        let info = SesAccountInfo {
            sending_enabled: true,
            sending_quota: Some(50000.0),
            max_send_rate: Some(14.0),
            sent_last_24_hours: Some(1234.5),
        };
        let result = SesAccountDetails::from(info);
        assert!(result.sending_enabled);
        assert_eq!(result.sending_quota, Some(50000.0));
        assert_eq!(result.max_send_rate, Some(14.0));
        assert_eq!(result.sent_last_24_hours, Some(1234.5));
    }

    #[test]
    fn test_ses_suppressed_destination_complaint() {
        let s = aws_sdk_sesv2::types::SuppressedDestinationSummary::builder()
            .email_address("complaint@example.com")
            .reason(aws_sdk_sesv2::types::SuppressionListReason::Complaint)
            .last_update_time(aws_sdk_sesv2::primitives::DateTime::from_secs(1_700_000_000))
            .build()
            .unwrap();
        let result = SesSuppressedDestination::from(s);
        assert_eq!(result.email_address, "complaint@example.com");
        assert_eq!(result.reason, "COMPLAINT");
    }
}
