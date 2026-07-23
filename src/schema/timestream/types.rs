use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use crate::aws::timestream::{TimestreamDatabaseInfo, TimestreamRetentionInfo, TimestreamTableInfo};
use crate::schema::time::to_utc;

#[derive(SimpleObject, Clone)]
pub struct TimestreamRetention {
    pub memory_store_retention_period_in_hours: Option<i64>,
    pub magnetic_store_retention_period_in_days: Option<i64>,
}

impl From<TimestreamRetentionInfo> for TimestreamRetention {
    fn from(r: TimestreamRetentionInfo) -> Self {
        Self {
            memory_store_retention_period_in_hours: r.memory_store_retention_period_in_hours,
            magnetic_store_retention_period_in_days: r.magnetic_store_retention_period_in_days,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TimestreamDatabase {
    pub database_name: Option<String>,
    pub arn: Option<String>,
    pub table_count: Option<i64>,
    pub kms_key_id: Option<String>,
    pub creation_time: Option<DateTime<Utc>>,
    pub last_updated_time: Option<DateTime<Utc>>,
}

impl From<TimestreamDatabaseInfo> for TimestreamDatabase {
    fn from(db: TimestreamDatabaseInfo) -> Self {
        Self {
            database_name: db.database_name,
            arn: db.arn,
            table_count: db.table_count,
            kms_key_id: db.kms_key_id,
            creation_time: to_utc(db.creation_time.as_ref()),
            last_updated_time: to_utc(db.last_updated_time.as_ref()),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct TimestreamTable {
    pub database_name: Option<String>,
    pub table_name: Option<String>,
    pub table_status: Option<String>,
    pub arn: Option<String>,
    pub creation_time: Option<DateTime<Utc>>,
    pub last_updated_time: Option<DateTime<Utc>>,
    pub retention_properties: Option<TimestreamRetention>,
}

impl From<TimestreamTableInfo> for TimestreamTable {
    fn from(t: TimestreamTableInfo) -> Self {
        Self {
            database_name: t.database_name,
            table_name: t.table_name,
            table_status: t.table_status,
            arn: t.arn,
            creation_time: to_utc(t.creation_time.as_ref()),
            last_updated_time: to_utc(t.last_updated_time.as_ref()),
            retention_properties: t.retention_properties.map(TimestreamRetention::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_smithy_types::DateTime as SmithyDateTime;
    use crate::aws::timestream::{TimestreamDatabaseInfo, TimestreamRetentionInfo, TimestreamTableInfo};

    #[test]
    fn test_retention_from() {
        let info = TimestreamRetentionInfo {
            memory_store_retention_period_in_hours: Some(24),
            magnetic_store_retention_period_in_days: Some(365),
        };
        let result = TimestreamRetention::from(info);
        assert_eq!(result.memory_store_retention_period_in_hours, Some(24));
        assert_eq!(result.magnetic_store_retention_period_in_days, Some(365));
    }

    #[test]
    fn test_retention_from_none() {
        let info = TimestreamRetentionInfo {
            memory_store_retention_period_in_hours: None,
            magnetic_store_retention_period_in_days: None,
        };
        let result = TimestreamRetention::from(info);
        assert!(result.memory_store_retention_period_in_hours.is_none());
        assert!(result.magnetic_store_retention_period_in_days.is_none());
    }

    #[test]
    fn test_database_from_full() {
        let info = TimestreamDatabaseInfo {
            database_name: Some("my-db".to_string()),
            arn: Some("arn:aws:timestream:us-east-1:123456789012:database/my-db".to_string()),
            table_count: Some(5),
            kms_key_id: Some("arn:aws:kms:us-east-1:123456789012:key/abc".to_string()),
            creation_time: Some(SmithyDateTime::from_secs(1_000_000)),
            last_updated_time: Some(SmithyDateTime::from_secs(2_000_000)),
        };
        let result = TimestreamDatabase::from(info);
        assert_eq!(result.database_name, Some("my-db".to_string()));
        assert_eq!(result.table_count, Some(5));
        assert!(result.kms_key_id.is_some());
        assert_eq!(result.creation_time, Some(DateTime::<Utc>::from_timestamp(1_000_000, 0).unwrap()));
        assert_eq!(result.last_updated_time, Some(DateTime::<Utc>::from_timestamp(2_000_000, 0).unwrap()));
    }

    #[test]
    fn test_database_from_minimal() {
        let info = TimestreamDatabaseInfo {
            database_name: None,
            arn: None,
            table_count: None,
            kms_key_id: None,
            creation_time: None,
            last_updated_time: None,
        };
        let result = TimestreamDatabase::from(info);
        assert!(result.database_name.is_none());
        assert!(result.table_count.is_none());
    }

    #[test]
    fn test_table_from_full() {
        let info = TimestreamTableInfo {
            database_name: Some("my-db".to_string()),
            table_name: Some("my-table".to_string()),
            table_status: Some("ACTIVE".to_string()),
            arn: Some("arn:aws:timestream:us-east-1:123456789012:database/my-db/table/my-table".to_string()),
            creation_time: Some(SmithyDateTime::from_secs(1_000_000)),
            last_updated_time: Some(SmithyDateTime::from_secs(2_000_000)),
            retention_properties: Some(TimestreamRetentionInfo {
                memory_store_retention_period_in_hours: Some(72),
                magnetic_store_retention_period_in_days: Some(30),
            }),
        };
        let result = TimestreamTable::from(info);
        assert_eq!(result.table_name, Some("my-table".to_string()));
        assert_eq!(result.table_status, Some("ACTIVE".to_string()));
        assert!(result.retention_properties.is_some());
        assert_eq!(result.creation_time, Some(DateTime::<Utc>::from_timestamp(1_000_000, 0).unwrap()));
        let ret = result.retention_properties.unwrap();
        assert_eq!(ret.memory_store_retention_period_in_hours, Some(72));
        assert_eq!(ret.magnetic_store_retention_period_in_days, Some(30));
    }

    #[test]
    fn test_table_from_no_retention() {
        let info = TimestreamTableInfo {
            database_name: Some("my-db".to_string()),
            table_name: Some("my-table".to_string()),
            table_status: Some("DELETING".to_string()),
            arn: None,
            creation_time: None,
            last_updated_time: None,
            retention_properties: None,
        };
        let result = TimestreamTable::from(info);
        assert_eq!(result.table_status, Some("DELETING".to_string()));
        assert!(result.retention_properties.is_none());
    }
}
