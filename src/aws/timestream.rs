use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct TimestreamDatabaseInfo {
    pub database_name: Option<String>,
    pub arn: Option<String>,
    pub table_count: Option<i64>,
    pub kms_key_id: Option<String>,
    pub creation_time: Option<String>,
    pub last_updated_time: Option<String>,
}

pub struct TimestreamRetentionInfo {
    pub memory_store_retention_period_in_hours: Option<i64>,
    pub magnetic_store_retention_period_in_days: Option<i64>,
}

pub struct TimestreamTableInfo {
    pub database_name: Option<String>,
    pub table_name: Option<String>,
    pub table_status: Option<String>,
    pub arn: Option<String>,
    pub creation_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub retention_properties: Option<TimestreamRetentionInfo>,
}

pub struct TimestreamClient {
    inner: aws_sdk_timestreamwrite::Client,
}

impl TimestreamClient {
    pub fn new(config: &SdkConfig) -> Self {
        // Endpoint discovery is handled automatically by the SDK for Timestream
        // and is not a builder option, so construct the client from config directly.
        Self {
            inner: aws_sdk_timestreamwrite::Client::new(config),
        }
    }

    pub async fn list_databases(&self) -> Result<Vec<TimestreamDatabaseInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_databases();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for db in output.databases() {
                items.push(TimestreamDatabaseInfo {
                    database_name: db.database_name().map(|s| s.to_string()),
                    arn: db.arn().map(|s| s.to_string()),
                    table_count: Some(db.table_count()),
                    kms_key_id: db.kms_key_id().map(|s| s.to_string()),
                    creation_time: db
                        .creation_time()
                        .map(|t| t.secs().to_string()),
                    last_updated_time: db
                        .last_updated_time()
                        .map(|t| t.secs().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_tables(
        &self,
        database_name: &str,
    ) -> Result<Vec<TimestreamTableInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_tables().database_name(database_name);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for table in output.tables() {
                items.push(TimestreamTableInfo {
                    database_name: table.database_name().map(|s| s.to_string()),
                    table_name: table.table_name().map(|s| s.to_string()),
                    table_status: table.table_status().map(|s| s.as_str().to_string()),
                    arn: table.arn().map(|s| s.to_string()),
                    creation_time: table
                        .creation_time()
                        .map(|t| t.secs().to_string()),
                    last_updated_time: table
                        .last_updated_time()
                        .map(|t| t.secs().to_string()),
                    retention_properties: table.retention_properties().map(|r| {
                        TimestreamRetentionInfo {
                            memory_store_retention_period_in_hours: Some(
                                r.memory_store_retention_period_in_hours(),
                            ),
                            magnetic_store_retention_period_in_days: Some(
                                r.magnetic_store_retention_period_in_days(),
                            ),
                        }
                    }),
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
