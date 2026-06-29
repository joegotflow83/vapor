use async_graphql::SimpleObject;

use crate::aws::detective::DatasourcePackageInfo;

#[derive(SimpleObject, Clone)]
pub struct DetectiveGraph {
    pub arn: Option<String>,
    pub created_time: Option<String>,
}

impl From<aws_sdk_detective::types::Graph> for DetectiveGraph {
    fn from(g: aws_sdk_detective::types::Graph) -> Self {
        Self {
            arn: g.arn().map(|v| v.to_string()),
            created_time: g.created_time().map(|t| t.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DetectiveMember {
    pub account_id: Option<String>,
    pub graph_arn: Option<String>,
    pub email_address: Option<String>,
    pub status: Option<String>,
    pub invited_time: Option<String>,
    pub updated_time: Option<String>,
    pub administrator_id: Option<String>,
}

impl From<aws_sdk_detective::types::MemberDetail> for DetectiveMember {
    fn from(m: aws_sdk_detective::types::MemberDetail) -> Self {
        Self {
            account_id: m.account_id().map(|v| v.to_string()),
            graph_arn: m.graph_arn().map(|v| v.to_string()),
            email_address: m.email_address().map(|v| v.to_string()),
            status: m.status().map(|s| s.as_str().to_string()),
            invited_time: m.invited_time().map(|t| t.to_string()),
            updated_time: m.updated_time().map(|t| t.to_string()),
            administrator_id: m.administrator_id().map(|v| v.to_string()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct DetectiveDatasourcePackage {
    pub datasource_package: Option<String>,
    pub ingest_state: Option<String>,
}

impl From<DatasourcePackageInfo> for DetectiveDatasourcePackage {
    fn from(d: DatasourcePackageInfo) -> Self {
        Self {
            datasource_package: d.datasource_package,
            ingest_state: d.ingest_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::detective::DatasourcePackageInfo;

    #[test]
    fn test_detective_graph_from_minimal() {
        let g = aws_sdk_detective::types::Graph::builder().build();
        let result = DetectiveGraph::from(g);
        assert!(result.arn.is_none());
        assert!(result.created_time.is_none());
    }

    #[test]
    fn test_detective_graph_from_full() {
        let g = aws_sdk_detective::types::Graph::builder()
            .arn("arn:aws:detective:us-east-1:123456789012:graph:abc123")
            .build();
        let result = DetectiveGraph::from(g);
        assert_eq!(
            result.arn,
            Some("arn:aws:detective:us-east-1:123456789012:graph:abc123".to_string())
        );
    }

    #[test]
    fn test_detective_member_from_minimal() {
        let m = aws_sdk_detective::types::MemberDetail::builder().build();
        let result = DetectiveMember::from(m);
        assert!(result.account_id.is_none());
        assert!(result.graph_arn.is_none());
        assert!(result.email_address.is_none());
        assert!(result.status.is_none());
        assert!(result.invited_time.is_none());
        assert!(result.updated_time.is_none());
        assert!(result.administrator_id.is_none());
    }

    #[test]
    fn test_detective_member_from_full() {
        let m = aws_sdk_detective::types::MemberDetail::builder()
            .account_id("123456789012")
            .graph_arn("arn:aws:detective:us-east-1:123456789012:graph:abc123")
            .email_address("user@example.com")
            .status(aws_sdk_detective::types::MemberStatus::Enabled)
            .administrator_id("987654321098")
            .build();
        let result = DetectiveMember::from(m);
        assert_eq!(result.account_id, Some("123456789012".to_string()));
        assert_eq!(
            result.graph_arn,
            Some("arn:aws:detective:us-east-1:123456789012:graph:abc123".to_string())
        );
        assert_eq!(result.email_address, Some("user@example.com".to_string()));
        assert_eq!(result.status, Some("ENABLED".to_string()));
        assert_eq!(result.administrator_id, Some("987654321098".to_string()));
    }

    #[test]
    fn test_detective_datasource_package_from() {
        let info = DatasourcePackageInfo {
            datasource_package: Some("DETECTIVE_CORE".to_string()),
            ingest_state: Some("STARTED".to_string()),
        };
        let result = DetectiveDatasourcePackage::from(info);
        assert_eq!(result.datasource_package, Some("DETECTIVE_CORE".to_string()));
        assert_eq!(result.ingest_state, Some("STARTED".to_string()));
    }

    #[test]
    fn test_detective_datasource_package_from_none() {
        let info = DatasourcePackageInfo {
            datasource_package: None,
            ingest_state: None,
        };
        let result = DetectiveDatasourcePackage::from(info);
        assert!(result.datasource_package.is_none());
        assert!(result.ingest_state.is_none());
    }
}
