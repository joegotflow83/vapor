use async_graphql::SimpleObject;
use aws_sdk_cloudformation::types::{Export, Stack, StackResourceSummary};

use crate::schema::common::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct CfnParameter {
    pub parameter_key: Option<String>,
    pub parameter_value: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct CfnOutput {
    pub output_key: Option<String>,
    pub output_value: Option<String>,
    pub description: Option<String>,
    pub export_name: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct CfnDriftInfo {
    pub stack_drift_status: Option<String>,
    pub last_check_timestamp: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct CfnStack {
    pub stack_id: Option<String>,
    pub stack_name: Option<String>,
    pub description: Option<String>,
    pub stack_status: Option<String>,
    pub creation_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub deletion_time: Option<String>,
    pub role_arn: Option<String>,
    pub capabilities: Vec<String>,
    pub parameters: Vec<CfnParameter>,
    pub outputs: Vec<CfnOutput>,
    pub drift_information: Option<CfnDriftInfo>,
    pub tags: Vec<Tag>,
}

impl From<&Stack> for CfnStack {
    fn from(s: &Stack) -> Self {
        let parameters = s
            .parameters()
            .iter()
            .map(|p| CfnParameter {
                parameter_key: p.parameter_key().map(|v| v.to_string()),
                parameter_value: p.parameter_value().map(|v| v.to_string()),
            })
            .collect();

        let outputs = s
            .outputs()
            .iter()
            .map(|o| CfnOutput {
                output_key: o.output_key().map(|v| v.to_string()),
                output_value: o.output_value().map(|v| v.to_string()),
                description: o.description().map(|v| v.to_string()),
                export_name: o.export_name().map(|v| v.to_string()),
            })
            .collect();

        let drift_information = s.drift_information().map(|d| CfnDriftInfo {
            stack_drift_status: d.stack_drift_status().map(|v| v.as_str().to_string()),
            last_check_timestamp: d.last_check_timestamp().map(|t| t.to_string()),
        });

        let capabilities = s
            .capabilities()
            .iter()
            .map(|c| c.as_str().to_string())
            .collect();

        let tags = s
            .tags()
            .iter()
            .map(|t| Tag {
                key: t.key().unwrap_or("").to_string(),
                value: t.value().unwrap_or("").to_string(),
            })
            .collect();

        Self {
            stack_id: s.stack_id().map(|v| v.to_string()),
            stack_name: s.stack_name().map(|v| v.to_string()),
            description: s.description().map(|v| v.to_string()),
            stack_status: s.stack_status().map(|v| v.as_str().to_string()),
            creation_time: s.creation_time().map(|t| t.to_string()),
            last_updated_time: s.last_updated_time().map(|t| t.to_string()),
            deletion_time: s.deletion_time().map(|t| t.to_string()),
            role_arn: s.role_arn().map(|v| v.to_string()),
            capabilities,
            parameters,
            outputs,
            drift_information,
            tags,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CfnStackResource {
    pub logical_resource_id: Option<String>,
    pub physical_resource_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_status: Option<String>,
    pub last_updated_timestamp: Option<String>,
    pub drift_status: Option<String>,
}

impl From<&StackResourceSummary> for CfnStackResource {
    fn from(r: &StackResourceSummary) -> Self {
        Self {
            logical_resource_id: r.logical_resource_id().map(|v| v.to_string()),
            physical_resource_id: r.physical_resource_id().map(|v| v.to_string()),
            resource_type: r.resource_type().map(|v| v.to_string()),
            resource_status: r.resource_status().map(|v| v.as_str().to_string()),
            last_updated_timestamp: r.last_updated_timestamp().map(|t| t.to_string()),
            drift_status: r
                .drift_information()
                .and_then(|d| d.stack_resource_drift_status())
                .map(|v| v.as_str().to_string()),
        }
    }
}

/// A CloudFormation export (cross-stack reference).
#[derive(SimpleObject, Clone)]
pub struct CfnExport {
    pub name: Option<String>,
    pub value: Option<String>,
    pub exporting_stack_id: Option<String>,
}

impl From<&Export> for CfnExport {
    fn from(e: &Export) -> Self {
        Self {
            name: e.name().map(|v| v.to_string()),
            value: e.value().map(|v| v.to_string()),
            exporting_stack_id: e.exporting_stack_id().map(|v| v.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfn_parameter_fields() {
        let p = CfnParameter {
            parameter_key: Some("Env".to_string()),
            parameter_value: Some("prod".to_string()),
        };
        assert_eq!(p.parameter_key, Some("Env".to_string()));
        assert_eq!(p.parameter_value, Some("prod".to_string()));
    }

    #[test]
    fn test_cfn_output_fields() {
        let o = CfnOutput {
            output_key: Some("BucketName".to_string()),
            output_value: Some("my-bucket".to_string()),
            description: Some("The bucket name".to_string()),
            export_name: Some("MyBucket".to_string()),
        };
        assert_eq!(o.output_key, Some("BucketName".to_string()));
        assert_eq!(o.export_name, Some("MyBucket".to_string()));
    }

    #[test]
    fn test_cfn_drift_info_fields() {
        let d = CfnDriftInfo {
            stack_drift_status: Some("IN_SYNC".to_string()),
            last_check_timestamp: None,
        };
        assert_eq!(d.stack_drift_status, Some("IN_SYNC".to_string()));
    }

    #[test]
    fn test_cfn_stack_resource_fields() {
        let r = CfnStackResource {
            logical_resource_id: Some("MyBucket".to_string()),
            physical_resource_id: Some("my-bucket-abc123".to_string()),
            resource_type: Some("AWS::S3::Bucket".to_string()),
            resource_status: Some("CREATE_COMPLETE".to_string()),
            last_updated_timestamp: None,
            drift_status: None,
        };
        assert_eq!(r.logical_resource_id, Some("MyBucket".to_string()));
        assert_eq!(r.resource_type, Some("AWS::S3::Bucket".to_string()));
    }

    #[test]
    fn test_cfn_export_fields() {
        let e = CfnExport {
            name: Some("SharedVpcId".to_string()),
            value: Some("vpc-0abc12345".to_string()),
            exporting_stack_id: Some(
                "arn:aws:cloudformation:us-east-1:123456789012:stack/network/abc".to_string(),
            ),
        };
        assert_eq!(e.name, Some("SharedVpcId".to_string()));
        assert_eq!(e.value, Some("vpc-0abc12345".to_string()));
        assert_eq!(
            e.exporting_stack_id,
            Some("arn:aws:cloudformation:us-east-1:123456789012:stack/network/abc".to_string())
        );
    }

    #[test]
    fn test_cfn_export_optional_fields() {
        let e = CfnExport {
            name: None,
            value: None,
            exporting_stack_id: None,
        };
        assert!(e.name.is_none());
        assert!(e.value.is_none());
        assert!(e.exporting_stack_id.is_none());
    }
}
