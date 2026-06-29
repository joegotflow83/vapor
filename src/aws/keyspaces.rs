use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct KeyspacesKeyspaceInfo {
    pub keyspace_name: String,
    pub resource_arn: String,
    pub replication_strategy: Option<String>,
    pub replication_regions: Vec<String>,
}

pub struct KeyspacesCapacitySpecInfo {
    pub throughput_mode: String,
    pub read_capacity_units: Option<i64>,
    pub write_capacity_units: Option<i64>,
}

pub struct KeyspacesEncryptionInfo {
    pub type_: String,
    pub kms_key_identifier: Option<String>,
}

pub struct KeyspacesTableInfo {
    pub keyspace_name: String,
    pub table_name: String,
    pub resource_arn: String,
    pub status: Option<String>,
    pub creation_timestamp: Option<String>,
    pub capacity_specification: Option<KeyspacesCapacitySpecInfo>,
    pub encryption_specification: Option<KeyspacesEncryptionInfo>,
    pub point_in_time_recovery: Option<bool>,
}

pub struct KeyspacesClient {
    inner: aws_sdk_keyspaces::Client,
}

impl KeyspacesClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_keyspaces::Client::new(config),
        }
    }

    pub async fn list_keyspaces(&self) -> Result<Vec<KeyspacesKeyspaceInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_keyspaces();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for ks in output.keyspaces() {
                items.push(KeyspacesKeyspaceInfo {
                    keyspace_name: ks.keyspace_name().to_string(),
                    resource_arn: ks.resource_arn().to_string(),
                    replication_strategy: Some(
                        ks.replication_strategy().as_str().to_string(),
                    ),
                    replication_regions: ks
                        .replication_regions()
                        .iter()
                        .map(|r| r.to_string())
                        .collect(),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    async fn fetch_table(
        &self,
        keyspace_name: &str,
        table_name: &str,
    ) -> Result<KeyspacesTableInfo, VaporError> {
        let output = self
            .inner
            .get_table()
            .keyspace_name(keyspace_name)
            .table_name(table_name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        let capacity_specification = output.capacity_specification().map(|cs| {
            KeyspacesCapacitySpecInfo {
                throughput_mode: cs.throughput_mode().as_str().to_string(),
                read_capacity_units: cs.read_capacity_units(),
                write_capacity_units: cs.write_capacity_units(),
            }
        });

        let encryption_specification = output.encryption_specification().map(|es| {
            KeyspacesEncryptionInfo {
                type_: es.r#type().as_str().to_string(),
                kms_key_identifier: es.kms_key_identifier().map(|s| s.to_string()),
            }
        });

        let point_in_time_recovery = output
            .point_in_time_recovery()
            .map(|pitr| pitr.status().as_str() == "ENABLED");

        Ok(KeyspacesTableInfo {
            keyspace_name: output.keyspace_name().to_string(),
            table_name: output.table_name().to_string(),
            resource_arn: output.resource_arn().to_string(),
            status: output.status().map(|s| s.as_str().to_string()),
            creation_timestamp: output.creation_timestamp().map(|dt| dt.to_string()),
            capacity_specification,
            encryption_specification,
            point_in_time_recovery,
        })
    }

    pub async fn list_tables(
        &self,
        keyspace_name: &str,
    ) -> Result<Vec<KeyspacesTableInfo>, VaporError> {
        let mut table_names: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_tables().keyspace_name(keyspace_name);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for table in output.tables() {
                table_names.push(table.table_name().to_string());
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        let mut results = Vec::new();
        for name in table_names {
            results.push(self.fetch_table(keyspace_name, &name).await?);
        }
        Ok(results)
    }

    pub async fn get_table(
        &self,
        keyspace_name: &str,
        table_name: &str,
    ) -> Result<Option<KeyspacesTableInfo>, VaporError> {
        match self.fetch_table(keyspace_name, table_name).await {
            Ok(info) => Ok(Some(info)),
            Err(_) => Ok(None),
        }
    }
}
