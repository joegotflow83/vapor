use async_graphql::SimpleObject;

#[derive(SimpleObject, Clone)]
pub struct ServiceQuota {
    pub service_code: String,
    pub service_name: Option<String>,
    pub quota_code: String,
    pub quota_name: Option<String>,
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub adjustable: bool,
    pub global_quota: bool,
}

impl From<&aws_sdk_servicequotas::types::ServiceQuota> for ServiceQuota {
    fn from(q: &aws_sdk_servicequotas::types::ServiceQuota) -> Self {
        Self {
            service_code: q.service_code().unwrap_or_default().to_string(),
            service_name: q.service_name().map(|s| s.to_string()),
            quota_code: q.quota_code().unwrap_or_default().to_string(),
            quota_name: q.quota_name().map(|s| s.to_string()),
            value: q.value(),
            unit: q.unit().map(|s| s.to_string()),
            adjustable: q.adjustable(),
            global_quota: q.global_quota(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_quota_from_sdk() {
        let sdk_quota = aws_sdk_servicequotas::types::ServiceQuota::builder()
            .service_code("ec2")
            .service_name("Amazon Elastic Compute Cloud (Amazon EC2)")
            .quota_code("L-1216C47A")
            .quota_name("Running On-Demand Standard (A, C, D, H, I, M, R, T, Z) instances")
            .value(32.0)
            .unit("None")
            .adjustable(true)
            .global_quota(false)
            .build();

        let result = ServiceQuota::from(&sdk_quota);
        assert_eq!(result.service_code, "ec2");
        assert_eq!(result.service_name, Some("Amazon Elastic Compute Cloud (Amazon EC2)".to_string()));
        assert_eq!(result.quota_code, "L-1216C47A");
        assert_eq!(result.quota_name, Some("Running On-Demand Standard (A, C, D, H, I, M, R, T, Z) instances".to_string()));
        assert_eq!(result.value, Some(32.0));
        assert_eq!(result.unit, Some("None".to_string()));
        assert!(result.adjustable);
        assert!(!result.global_quota);
    }

    #[test]
    fn test_service_quota_minimal() {
        let sdk_quota = aws_sdk_servicequotas::types::ServiceQuota::builder().build();
        let result = ServiceQuota::from(&sdk_quota);
        assert_eq!(result.service_code, "");
        assert!(result.service_name.is_none());
        assert_eq!(result.quota_code, "");
        assert!(result.quota_name.is_none());
        assert!(result.value.is_none());
        assert!(result.unit.is_none());
        assert!(!result.adjustable);
        assert!(!result.global_quota);
    }

    #[test]
    fn test_service_quota_global() {
        let sdk_quota = aws_sdk_servicequotas::types::ServiceQuota::builder()
            .service_code("iam")
            .quota_code("L-F55AF5D4")
            .quota_name("Groups per account")
            .value(300.0)
            .global_quota(true)
            .adjustable(true)
            .build();

        let result = ServiceQuota::from(&sdk_quota);
        assert_eq!(result.service_code, "iam");
        assert!(result.global_quota);
        assert!(result.adjustable);
        assert_eq!(result.value, Some(300.0));
    }
}
