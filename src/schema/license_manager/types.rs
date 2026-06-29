use async_graphql::SimpleObject;

use crate::aws::license_manager::{LicenseConfigurationInfo, LicenseGrantInfo, LicenseInfo};

#[derive(SimpleObject, Clone)]
pub struct LicenseConfiguration {
    pub license_configuration_id: Option<String>,
    pub license_configuration_arn: Option<String>,
    pub name: Option<String>,
    pub license_counting_type: Option<String>,
    pub license_count: Option<i64>,
    pub license_count_hard_limit: Option<bool>,
    pub consumed_licenses: Option<i64>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub product_information_list: Vec<String>,
}

impl From<LicenseConfigurationInfo> for LicenseConfiguration {
    fn from(i: LicenseConfigurationInfo) -> Self {
        Self {
            license_configuration_id: i.license_configuration_id,
            license_configuration_arn: i.license_configuration_arn,
            name: i.name,
            license_counting_type: i.license_counting_type,
            license_count: i.license_count,
            license_count_hard_limit: i.license_count_hard_limit,
            consumed_licenses: i.consumed_licenses,
            status: i.status,
            description: i.description,
            product_information_list: i.product_information_list,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct LicenseGrant {
    pub grant_arn: Option<String>,
    pub grant_name: Option<String>,
    pub parent_arn: Option<String>,
    pub license_arn: Option<String>,
    pub grantee_principal_arn: Option<String>,
    pub home_region: Option<String>,
    pub grant_status: Option<String>,
    pub version: Option<String>,
}

impl From<LicenseGrantInfo> for LicenseGrant {
    fn from(i: LicenseGrantInfo) -> Self {
        Self {
            grant_arn: i.grant_arn,
            grant_name: i.grant_name,
            parent_arn: i.parent_arn,
            license_arn: i.license_arn,
            grantee_principal_arn: i.grantee_principal_arn,
            home_region: i.home_region,
            grant_status: i.grant_status,
            version: i.version,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct License {
    pub license_arn: Option<String>,
    pub license_name: Option<String>,
    pub product_name: Option<String>,
    pub product_sku: Option<String>,
    pub issuer: Option<String>,
    pub status: Option<String>,
    pub validity_period_start: Option<String>,
    pub validity_period_end: Option<String>,
}

impl From<LicenseInfo> for License {
    fn from(i: LicenseInfo) -> Self {
        Self {
            license_arn: i.license_arn,
            license_name: i.license_name,
            product_name: i.product_name,
            product_sku: i.product_sku,
            issuer: i.issuer,
            status: i.status,
            validity_period_start: i.validity_period_start,
            validity_period_end: i.validity_period_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::license_manager::{LicenseConfigurationInfo, LicenseGrantInfo, LicenseInfo};

    #[test]
    fn test_license_configuration_from_full() {
        let info = LicenseConfigurationInfo {
            license_configuration_id: Some("lc-abc123".to_string()),
            license_configuration_arn: Some(
                "arn:aws:license-manager:us-east-1:123456789012:license-configuration:lc-abc123"
                    .to_string(),
            ),
            name: Some("MyLicenseConfig".to_string()),
            license_counting_type: Some("vCPU".to_string()),
            license_count: Some(100),
            license_count_hard_limit: Some(true),
            consumed_licenses: Some(42),
            status: Some("AVAILABLE".to_string()),
            description: Some("Test config".to_string()),
            product_information_list: vec!["Windows Server".to_string()],
        };
        let result = LicenseConfiguration::from(info);
        assert_eq!(result.license_configuration_id, Some("lc-abc123".to_string()));
        assert_eq!(result.name, Some("MyLicenseConfig".to_string()));
        assert_eq!(result.license_counting_type, Some("vCPU".to_string()));
        assert_eq!(result.license_count, Some(100));
        assert_eq!(result.license_count_hard_limit, Some(true));
        assert_eq!(result.consumed_licenses, Some(42));
        assert_eq!(result.status, Some("AVAILABLE".to_string()));
        assert_eq!(result.product_information_list.len(), 1);
    }

    #[test]
    fn test_license_configuration_from_minimal() {
        let info = LicenseConfigurationInfo {
            license_configuration_id: None,
            license_configuration_arn: None,
            name: Some("MinimalConfig".to_string()),
            license_counting_type: None,
            license_count: None,
            license_count_hard_limit: None,
            consumed_licenses: None,
            status: None,
            description: None,
            product_information_list: vec![],
        };
        let result = LicenseConfiguration::from(info);
        assert_eq!(result.name, Some("MinimalConfig".to_string()));
        assert!(result.license_configuration_id.is_none());
        assert!(result.product_information_list.is_empty());
    }

    #[test]
    fn test_license_grant_from_full() {
        let info = LicenseGrantInfo {
            grant_arn: Some("arn:aws:license-manager::123456789012:grant:g-abc123".to_string()),
            grant_name: Some("MyGrant".to_string()),
            parent_arn: Some("arn:aws:license-manager::123456789012:license:l-abc123".to_string()),
            license_arn: Some("arn:aws:license-manager::123456789012:license:l-abc123".to_string()),
            grantee_principal_arn: Some("arn:aws:iam::987654321098:root".to_string()),
            home_region: Some("us-east-1".to_string()),
            grant_status: Some("ACTIVE".to_string()),
            version: Some("1".to_string()),
        };
        let result = LicenseGrant::from(info);
        assert_eq!(result.grant_name, Some("MyGrant".to_string()));
        assert_eq!(result.grant_status, Some("ACTIVE".to_string()));
        assert_eq!(result.home_region, Some("us-east-1".to_string()));
        assert_eq!(result.version, Some("1".to_string()));
    }

    #[test]
    fn test_license_grant_from_minimal() {
        let info = LicenseGrantInfo {
            grant_arn: None,
            grant_name: None,
            parent_arn: None,
            license_arn: None,
            grantee_principal_arn: None,
            home_region: None,
            grant_status: Some("PENDING_ACCEPT".to_string()),
            version: None,
        };
        let result = LicenseGrant::from(info);
        assert!(result.grant_arn.is_none());
        assert_eq!(result.grant_status, Some("PENDING_ACCEPT".to_string()));
    }

    #[test]
    fn test_license_from_full() {
        let info = LicenseInfo {
            license_arn: Some(
                "arn:aws:license-manager::123456789012:license:l-abc123".to_string(),
            ),
            license_name: Some("MyLicense".to_string()),
            product_name: Some("Windows Server 2022".to_string()),
            product_sku: Some("BYOL-WS2022".to_string()),
            issuer: Some("Microsoft".to_string()),
            status: Some("AVAILABLE".to_string()),
            validity_period_start: Some("2024-01-01T00:00:00Z".to_string()),
            validity_period_end: Some("2025-01-01T00:00:00Z".to_string()),
        };
        let result = License::from(info);
        assert_eq!(result.license_name, Some("MyLicense".to_string()));
        assert_eq!(result.product_name, Some("Windows Server 2022".to_string()));
        assert_eq!(result.issuer, Some("Microsoft".to_string()));
        assert_eq!(result.status, Some("AVAILABLE".to_string()));
        assert!(result.validity_period_start.is_some());
        assert!(result.validity_period_end.is_some());
    }

    #[test]
    fn test_license_from_minimal() {
        let info = LicenseInfo {
            license_arn: None,
            license_name: None,
            product_name: None,
            product_sku: None,
            issuer: None,
            status: Some("EXPIRED".to_string()),
            validity_period_start: None,
            validity_period_end: None,
        };
        let result = License::from(info);
        assert!(result.license_arn.is_none());
        assert_eq!(result.status, Some("EXPIRED".to_string()));
        assert!(result.validity_period_start.is_none());
    }
}
