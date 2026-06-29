use async_graphql::SimpleObject;

use crate::aws::qldb::{QldbJournalExportInfo, QldbLedgerInfo};
use crate::schema::ec2::types::Tag;

#[derive(SimpleObject, Clone)]
pub struct QldbLedger {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub state: Option<String>,
    pub creation_date_time: Option<String>,
    pub permissions_mode: Option<String>,
    pub deletion_protection: Option<bool>,
    pub kms_key_arn: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<QldbLedgerInfo> for QldbLedger {
    fn from(l: QldbLedgerInfo) -> Self {
        Self {
            name: l.name,
            arn: l.arn,
            state: l.state,
            creation_date_time: l.creation_date_time,
            permissions_mode: l.permissions_mode,
            deletion_protection: l.deletion_protection,
            kms_key_arn: l.kms_key_arn,
            tags: l.tags.into_iter().map(|(k, v)| Tag { key: k, value: v }).collect(),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct QldbJournalExport {
    pub ledger_name: String,
    pub export_id: String,
    pub export_creation_time: Option<String>,
    pub status: Option<String>,
    pub inclusive_start_time: Option<String>,
    pub exclusive_end_time: Option<String>,
    pub output_format: Option<String>,
}

impl From<QldbJournalExportInfo> for QldbJournalExport {
    fn from(e: QldbJournalExportInfo) -> Self {
        Self {
            ledger_name: e.ledger_name,
            export_id: e.export_id,
            export_creation_time: e.export_creation_time,
            status: e.status,
            inclusive_start_time: e.inclusive_start_time,
            exclusive_end_time: e.exclusive_end_time,
            output_format: e.output_format,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::qldb::{QldbJournalExportInfo, QldbLedgerInfo};

    #[test]
    fn test_ledger_from_full() {
        let info = QldbLedgerInfo {
            name: Some("my-ledger".to_string()),
            arn: Some("arn:aws:qldb:us-east-1:123456789:ledger/my-ledger".to_string()),
            state: Some("ACTIVE".to_string()),
            creation_date_time: Some("2024-01-01T00:00:00Z".to_string()),
            permissions_mode: Some("STANDARD".to_string()),
            deletion_protection: Some(true),
            kms_key_arn: Some("arn:aws:kms:us-east-1:123456789:key/abc".to_string()),
            tags: vec![("Env".to_string(), "prod".to_string())],
        };
        let result = QldbLedger::from(info);
        assert_eq!(result.name, Some("my-ledger".to_string()));
        assert_eq!(result.arn, Some("arn:aws:qldb:us-east-1:123456789:ledger/my-ledger".to_string()));
        assert_eq!(result.state, Some("ACTIVE".to_string()));
        assert_eq!(result.permissions_mode, Some("STANDARD".to_string()));
        assert_eq!(result.deletion_protection, Some(true));
        assert!(result.kms_key_arn.is_some());
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0].key, "Env");
        assert_eq!(result.tags[0].value, "prod");
    }

    #[test]
    fn test_ledger_from_minimal() {
        let info = QldbLedgerInfo {
            name: None,
            arn: None,
            state: None,
            creation_date_time: None,
            permissions_mode: None,
            deletion_protection: None,
            kms_key_arn: None,
            tags: vec![],
        };
        let result = QldbLedger::from(info);
        assert!(result.name.is_none());
        assert!(result.arn.is_none());
        assert!(result.deletion_protection.is_none());
        assert!(result.kms_key_arn.is_none());
        assert!(result.tags.is_empty());
    }

    #[test]
    fn test_journal_export_from_full() {
        let info = QldbJournalExportInfo {
            ledger_name: "my-ledger".to_string(),
            export_id: "export-abc123".to_string(),
            export_creation_time: Some("2024-01-01T00:00:00Z".to_string()),
            status: Some("COMPLETED".to_string()),
            inclusive_start_time: Some("2024-01-01T00:00:00Z".to_string()),
            exclusive_end_time: Some("2024-01-02T00:00:00Z".to_string()),
            output_format: Some("ION_BINARY".to_string()),
        };
        let result = QldbJournalExport::from(info);
        assert_eq!(result.ledger_name, "my-ledger");
        assert_eq!(result.export_id, "export-abc123");
        assert_eq!(result.status, Some("COMPLETED".to_string()));
        assert_eq!(result.output_format, Some("ION_BINARY".to_string()));
    }

    #[test]
    fn test_journal_export_from_minimal() {
        let info = QldbJournalExportInfo {
            ledger_name: "my-ledger".to_string(),
            export_id: "export-xyz".to_string(),
            export_creation_time: None,
            status: None,
            inclusive_start_time: None,
            exclusive_end_time: None,
            output_format: None,
        };
        let result = QldbJournalExport::from(info);
        assert_eq!(result.ledger_name, "my-ledger");
        assert_eq!(result.export_id, "export-xyz");
        assert!(result.status.is_none());
        assert!(result.output_format.is_none());
    }
}
