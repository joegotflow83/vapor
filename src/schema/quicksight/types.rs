use async_graphql::SimpleObject;

use crate::aws::quicksight::{
    QuickSightDashboardInfo, QuickSightDataSetInfo, QuickSightDataSourceInfo, QuickSightUserInfo,
};

#[derive(SimpleObject, Clone)]
pub struct QuickSightUser {
    pub user_name: Option<String>,
    pub arn: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub identity_type: Option<String>,
    pub active: bool,
    pub principal_id: Option<String>,
}

impl From<QuickSightUserInfo> for QuickSightUser {
    fn from(u: QuickSightUserInfo) -> Self {
        Self {
            user_name: u.user_name,
            arn: u.arn,
            email: u.email,
            role: u.role,
            identity_type: u.identity_type,
            active: u.active,
            principal_id: u.principal_id,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct QuickSightDashboard {
    pub dashboard_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub created_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub published_version_number: Option<i64>,
    pub last_published_time: Option<String>,
}

impl From<QuickSightDashboardInfo> for QuickSightDashboard {
    fn from(d: QuickSightDashboardInfo) -> Self {
        Self {
            dashboard_id: d.dashboard_id,
            arn: d.arn,
            name: d.name,
            created_time: d.created_time,
            last_updated_time: d.last_updated_time,
            published_version_number: d.published_version_number,
            last_published_time: d.last_published_time,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct QuickSightDataSet {
    pub data_set_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub created_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub import_mode: Option<String>,
}

impl From<QuickSightDataSetInfo> for QuickSightDataSet {
    fn from(ds: QuickSightDataSetInfo) -> Self {
        Self {
            data_set_id: ds.data_set_id,
            arn: ds.arn,
            name: ds.name,
            created_time: ds.created_time,
            last_updated_time: ds.last_updated_time,
            import_mode: ds.import_mode,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct QuickSightDataSource {
    pub data_source_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub type_: Option<String>,
    pub status: Option<String>,
    pub created_time: Option<String>,
    pub last_updated_time: Option<String>,
}

impl From<QuickSightDataSourceInfo> for QuickSightDataSource {
    fn from(src: QuickSightDataSourceInfo) -> Self {
        Self {
            data_source_id: src.data_source_id,
            arn: src.arn,
            name: src.name,
            type_: src.type_,
            status: src.status,
            created_time: src.created_time,
            last_updated_time: src.last_updated_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::quicksight::{
        QuickSightDashboardInfo, QuickSightDataSetInfo, QuickSightDataSourceInfo,
        QuickSightUserInfo,
    };

    #[test]
    fn test_user_from_full() {
        let info = QuickSightUserInfo {
            user_name: Some("john.doe".to_string()),
            arn: Some("arn:aws:quicksight:us-east-1:123456789012:user/default/john.doe".to_string()),
            email: Some("john.doe@example.com".to_string()),
            role: Some("ADMIN".to_string()),
            identity_type: Some("IAM".to_string()),
            active: true,
            principal_id: Some("federated/iam/123456789012:john.doe".to_string()),
        };
        let result = QuickSightUser::from(info);
        assert_eq!(result.user_name, Some("john.doe".to_string()));
        assert_eq!(result.role, Some("ADMIN".to_string()));
        assert_eq!(result.identity_type, Some("IAM".to_string()));
        assert!(result.active);
        assert!(result.principal_id.is_some());
    }

    #[test]
    fn test_user_from_minimal() {
        let info = QuickSightUserInfo {
            user_name: None,
            arn: None,
            email: None,
            role: None,
            identity_type: None,
            active: false,
            principal_id: None,
        };
        let result = QuickSightUser::from(info);
        assert!(result.user_name.is_none());
        assert!(!result.active);
    }

    #[test]
    fn test_dashboard_from_full() {
        let info = QuickSightDashboardInfo {
            dashboard_id: Some("sales-overview".to_string()),
            arn: Some("arn:aws:quicksight:us-east-1:123456789012:dashboard/sales-overview".to_string()),
            name: Some("Sales Overview".to_string()),
            created_time: Some("2024-01-15T10:30:00Z".to_string()),
            last_updated_time: Some("2024-02-01T12:00:00Z".to_string()),
            published_version_number: Some(3),
            last_published_time: Some("2024-02-01T12:00:00Z".to_string()),
        };
        let result = QuickSightDashboard::from(info);
        assert_eq!(result.dashboard_id, Some("sales-overview".to_string()));
        assert_eq!(result.name, Some("Sales Overview".to_string()));
        assert_eq!(result.published_version_number, Some(3));
        assert!(result.last_published_time.is_some());
    }

    #[test]
    fn test_dashboard_from_minimal() {
        let info = QuickSightDashboardInfo {
            dashboard_id: None,
            arn: None,
            name: None,
            created_time: None,
            last_updated_time: None,
            published_version_number: None,
            last_published_time: None,
        };
        let result = QuickSightDashboard::from(info);
        assert!(result.dashboard_id.is_none());
        assert!(result.published_version_number.is_none());
        assert!(result.last_published_time.is_none());
    }

    #[test]
    fn test_data_set_from_full() {
        let info = QuickSightDataSetInfo {
            data_set_id: Some("customer-orders".to_string()),
            arn: Some("arn:aws:quicksight:us-east-1:123456789012:dataset/customer-orders".to_string()),
            name: Some("Customer Orders".to_string()),
            created_time: Some("2024-01-15T10:30:00Z".to_string()),
            last_updated_time: Some("2024-02-01T12:00:00Z".to_string()),
            import_mode: Some("SPICE".to_string()),
        };
        let result = QuickSightDataSet::from(info);
        assert_eq!(result.data_set_id, Some("customer-orders".to_string()));
        assert_eq!(result.import_mode, Some("SPICE".to_string()));
    }

    #[test]
    fn test_data_set_from_direct_query() {
        let info = QuickSightDataSetInfo {
            data_set_id: Some("live-orders".to_string()),
            arn: Some("arn:aws:quicksight:us-east-1:123456789012:dataset/live-orders".to_string()),
            name: Some("Live Orders".to_string()),
            created_time: None,
            last_updated_time: None,
            import_mode: Some("DIRECT_QUERY".to_string()),
        };
        let result = QuickSightDataSet::from(info);
        assert_eq!(result.import_mode, Some("DIRECT_QUERY".to_string()));
        assert!(result.created_time.is_none());
    }

    #[test]
    fn test_data_source_from_full() {
        let info = QuickSightDataSourceInfo {
            data_source_id: Some("prod-redshift".to_string()),
            arn: Some("arn:aws:quicksight:us-east-1:123456789012:datasource/prod-redshift".to_string()),
            name: Some("Production Redshift".to_string()),
            type_: Some("REDSHIFT".to_string()),
            status: Some("CREATION_SUCCESSFUL".to_string()),
            created_time: Some("2024-01-10T08:00:00Z".to_string()),
            last_updated_time: Some("2024-01-10T08:05:00Z".to_string()),
        };
        let result = QuickSightDataSource::from(info);
        assert_eq!(result.data_source_id, Some("prod-redshift".to_string()));
        assert_eq!(result.type_, Some("REDSHIFT".to_string()));
        assert_eq!(result.status, Some("CREATION_SUCCESSFUL".to_string()));
    }

    #[test]
    fn test_data_source_from_minimal() {
        let info = QuickSightDataSourceInfo {
            data_source_id: None,
            arn: None,
            name: None,
            type_: None,
            status: None,
            created_time: None,
            last_updated_time: None,
        };
        let result = QuickSightDataSource::from(info);
        assert!(result.type_.is_none());
        assert!(result.status.is_none());
    }
}
