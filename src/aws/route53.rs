use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct Route53Client {
    inner: aws_sdk_route53::Client,
}

impl Route53Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_route53::Client::new(config),
        }
    }

    /// List all hosted zones via is_truncated + next_marker pagination.
    pub async fn list_hosted_zones(
        &self,
    ) -> Result<Vec<aws_sdk_route53::types::HostedZone>, VaporError> {
        let mut zones = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_hosted_zones().max_items(100);
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            zones.extend(output.hosted_zones().iter().cloned());
            if output.is_truncated() {
                next_marker = output.next_marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(zones)
    }

    /// List all resource record sets for a hosted zone, paginated via
    /// is_truncated + next_record_name / next_record_type / next_record_identifier.
    pub async fn list_resource_record_sets(
        &self,
        hosted_zone_id: &str,
    ) -> Result<Vec<aws_sdk_route53::types::ResourceRecordSet>, VaporError> {
        let mut records = Vec::new();
        let mut next_record_name: Option<String> = None;
        let mut next_record_type: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_resource_record_sets()
                .hosted_zone_id(hosted_zone_id)
                .max_items(300);
            if let Some(ref name) = next_record_name {
                req = req.start_record_name(name);
                if let Some(ref t) = next_record_type {
                    req = req.start_record_type(t.as_str().into());
                }
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            records.extend(output.resource_record_sets().iter().cloned());
            if output.is_truncated() {
                next_record_name = output.next_record_name().map(|s| s.to_string());
                next_record_type = output.next_record_type().map(|t| t.as_str().to_string());
            } else {
                break;
            }
        }

        Ok(records)
    }

    /// List all health checks via is_truncated + next_marker pagination.
    pub async fn list_health_checks(
        &self,
    ) -> Result<Vec<aws_sdk_route53::types::HealthCheck>, VaporError> {
        let mut checks = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_health_checks().max_items(100);
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            checks.extend(output.health_checks().iter().cloned());
            if output.is_truncated() {
                next_marker = output.next_marker().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(checks)
    }
}
