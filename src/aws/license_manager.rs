use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct LicenseConfigurationInfo {
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

pub struct LicenseGrantInfo {
    pub grant_arn: Option<String>,
    pub grant_name: Option<String>,
    pub parent_arn: Option<String>,
    pub license_arn: Option<String>,
    pub grantee_principal_arn: Option<String>,
    pub home_region: Option<String>,
    pub grant_status: Option<String>,
    pub version: Option<String>,
}

pub struct LicenseInfo {
    pub license_arn: Option<String>,
    pub license_name: Option<String>,
    pub product_name: Option<String>,
    pub product_sku: Option<String>,
    pub issuer: Option<String>,
    pub status: Option<String>,
    pub validity_period_start: Option<String>,
    pub validity_period_end: Option<String>,
}

pub struct LicenseManagerClient {
    inner: aws_sdk_licensemanager::Client,
}

impl LicenseManagerClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_licensemanager::Client::new(config),
        }
    }

    pub async fn list_license_configurations(
        &self,
    ) -> Result<Vec<LicenseConfigurationInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_license_configurations();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for cfg in output.license_configurations() {
                let product_information_list: Vec<String> = cfg
                    .product_information_list()
                    .iter()
                    .filter_map(|pi| {
                        pi.product_information_filter_list()
                            .first()
                            .map(|f| f.product_information_filter_name().to_string())
                    })
                    .collect();

                items.push(LicenseConfigurationInfo {
                    license_configuration_id: cfg
                        .license_configuration_id()
                        .map(|s| s.to_string()),
                    license_configuration_arn: cfg
                        .license_configuration_arn()
                        .map(|s| s.to_string()),
                    name: cfg.name().map(|s| s.to_string()),
                    license_counting_type: cfg
                        .license_counting_type()
                        .map(|t| t.as_str().to_string()),
                    license_count: cfg.license_count(),
                    license_count_hard_limit: cfg.license_count_hard_limit(),
                    consumed_licenses: cfg.consumed_licenses(),
                    status: cfg.status().map(|s| s.to_string()),
                    description: cfg.description().map(|s| s.to_string()),
                    product_information_list,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_licenses(&self) -> Result<Vec<LicenseInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_licenses();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for license in output.licenses() {
                let (validity_start, validity_end) =
                    if let Some(validity) = license.validity() {
                        (
                            Some(validity.begin().to_string()),
                            validity.end().map(|s| s.to_string()),
                        )
                    } else {
                        (None, None)
                    };

                items.push(LicenseInfo {
                    license_arn: license.license_arn().map(|s| s.to_string()),
                    license_name: license.license_name().map(|s| s.to_string()),
                    product_name: license.product_name().map(|s| s.to_string()),
                    product_sku: license.product_sku().map(|s| s.to_string()),
                    issuer: license.issuer().and_then(|i| i.name()).map(|s| s.to_string()),
                    status: license.status().map(|s| s.as_str().to_string()),
                    validity_period_start: validity_start,
                    validity_period_end: validity_end,
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_received_grants(&self) -> Result<Vec<LicenseGrantInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_received_grants();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for grant in output.grants() {
                items.push(LicenseGrantInfo {
                    grant_arn: Some(grant.grant_arn().to_string()),
                    grant_name: Some(grant.grant_name().to_string()),
                    parent_arn: Some(grant.parent_arn().to_string()),
                    license_arn: Some(grant.license_arn().to_string()),
                    grantee_principal_arn: Some(grant.grantee_principal_arn().to_string()),
                    home_region: Some(grant.home_region().to_string()),
                    grant_status: Some(grant.grant_status().as_str().to_string()),
                    version: Some(grant.version().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
