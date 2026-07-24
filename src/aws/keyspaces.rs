use aws_config::SdkConfig;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;

use crate::error::VaporError;

#[derive(Debug)]
pub struct KeyspacesKeyspaceInfo {
    pub keyspace_name: String,
    pub resource_arn: String,
    pub replication_strategy: Option<String>,
    pub replication_regions: Vec<String>,
}

#[derive(Debug)]
pub struct KeyspacesCapacitySpecInfo {
    pub throughput_mode: String,
    pub read_capacity_units: Option<i64>,
    pub write_capacity_units: Option<i64>,
}

#[derive(Debug)]
pub struct KeyspacesEncryptionInfo {
    pub type_: String,
    pub kms_key_identifier: Option<String>,
}

#[derive(Debug)]
pub struct KeyspacesTableInfo {
    pub keyspace_name: String,
    pub table_name: String,
    pub resource_arn: String,
    pub status: Option<String>,
    pub creation_timestamp: Option<aws_smithy_types::DateTime>,
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

    /// Lists keyspaces, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListKeyspaces` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-keyspaces` 1.108.0's
    /// `operation/list_keyspaces/_list_keyspaces_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself, matching
    /// `mq.rs`'s `list_configurations` pattern.
    pub async fn list_keyspaces(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<KeyspacesKeyspaceInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_keyspaces();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for ks in output.keyspaces {
                items.push(KeyspacesKeyspaceInfo {
                    keyspace_name: ks.keyspace_name,
                    resource_arn: ks.resource_arn,
                    replication_strategy: Some(ks.replication_strategy.as_str().to_string()),
                    replication_regions: ks.replication_regions.unwrap_or_default(),
                });
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    fn table_info_from_output(
        output: aws_sdk_keyspaces::operation::get_table::GetTableOutput,
    ) -> KeyspacesTableInfo {
        let capacity_specification =
            output
                .capacity_specification()
                .map(|cs| KeyspacesCapacitySpecInfo {
                    throughput_mode: cs.throughput_mode().as_str().to_string(),
                    read_capacity_units: cs.read_capacity_units(),
                    write_capacity_units: cs.write_capacity_units(),
                });

        let encryption_specification =
            output
                .encryption_specification()
                .map(|es| KeyspacesEncryptionInfo {
                    type_: es.r#type().as_str().to_string(),
                    kms_key_identifier: es.kms_key_identifier().map(|s| s.to_string()),
                });

        let point_in_time_recovery = output
            .point_in_time_recovery()
            .map(|pitr| pitr.status().as_str() == "ENABLED");

        KeyspacesTableInfo {
            keyspace_name: output.keyspace_name().to_string(),
            table_name: output.table_name().to_string(),
            resource_arn: output.resource_arn().to_string(),
            status: output.status().map(|s| s.as_str().to_string()),
            creation_timestamp: output.creation_timestamp().cloned(),
            capacity_specification,
            encryption_specification,
            point_in_time_recovery,
        }
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
            .map_err(crate::error::sdk_err)?;

        Ok(Self::table_info_from_output(output))
    }

    /// Lists tables, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListTables` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-keyspaces` 1.108.0's
    /// `operation/list_tables/_list_tables_input.rs`), so `limit` is capped
    /// to the remaining budget on the request itself, matching `mq.rs`'s
    /// `list_brokers` pattern: the table-name page is collected first (up to
    /// `limit`/`next_token`), then the N+1 `get_table` fan-out only covers
    /// that single page, not an internal loop to exhaustion.
    pub async fn list_tables(
        &self,
        keyspace_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<KeyspacesTableInfo>, Option<String>), VaporError> {
        let mut table_names: Vec<String> = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_tables().keyspace_name(keyspace_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - table_names.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for table in output.tables.unwrap_or_default() {
                table_names.push(table.table_name);
            }
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if table_names.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut results = Vec::with_capacity(table_names.len());
        for name in table_names {
            results.push(self.fetch_table(keyspace_name, &name).await?);
        }
        Ok((results, token))
    }

    pub async fn get_table(
        &self,
        keyspace_name: &str,
        table_name: &str,
    ) -> Result<Option<KeyspacesTableInfo>, VaporError> {
        match self
            .inner
            .get_table()
            .keyspace_name(keyspace_name)
            .table_name(table_name)
            .send()
            .await
        {
            Ok(output) => Ok(Some(Self::table_info_from_output(output))),
            Err(e) => {
                if matches!(e.code(), Some("ResourceNotFoundException")) {
                    Ok(None)
                } else {
                    Err(crate::error::sdk_err(e))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://cassandra.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_keyspaces_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"keyspaces":[{"keyspaceName":"ks1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks1","replicationStrategy":"SINGLE_REGION"}]}"#,
            ),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_keyspaces(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].keyspace_name, "ks1");
        assert_eq!(
            items[0].replication_strategy,
            Some("SINGLE_REGION".to_string())
        );
        assert!(items[0].replication_regions.is_empty());
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_keyspaces_maps_multi_region_replication_regions() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"keyspaces":[{"keyspaceName":"global-ks","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/global-ks","replicationStrategy":"MULTI_REGION","replicationRegions":["us-east-1","eu-west-1"]}]}"#,
            ),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, _) = client.list_keyspaces(None, None).await.unwrap();

        assert_eq!(
            items[0].replication_strategy,
            Some("MULTI_REGION".to_string())
        );
        assert_eq!(
            items[0].replication_regions,
            vec!["us-east-1".to_string(), "eu-west-1".to_string()]
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_keyspaces_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"nextToken":"cursor-a"}"#),
            json_response(
                200,
                r#"{"keyspaces":[{"keyspaceName":"ks2","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks2","replicationStrategy":"SINGLE_REGION"}]}"#,
            ),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_keyspaces(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_keyspaces_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"maxResults":1}"#),
            json_response(
                200,
                r#"{"keyspaces":[{"keyspaceName":"ks1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks1","replicationStrategy":"SINGLE_REGION"}],"nextToken":"cursor-b"}"#,
            ),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_keyspaces(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("cursor-b".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_keyspaces_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":10}"#),
                json_response(
                    200,
                    r#"{"keyspaces":[{"keyspaceName":"ks1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks1","replicationStrategy":"SINGLE_REGION"}],"nextToken":"cursor-c"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"nextToken":"cursor-c","maxResults":9}"#),
                json_response(
                    200,
                    r#"{"keyspaces":[{"keyspaceName":"ks2","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks2","replicationStrategy":"SINGLE_REGION"}]}"#,
                ),
            ),
        ]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_keyspaces(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_keyspaces_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let err = client.list_keyspaces(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"keyspaceName":"mykeyspace"}"#),
                json_response(
                    200,
                    r#"{"tables":[{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t1"}"#,
                ),
                json_response(
                    200,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1","status":"ACTIVE"}"#,
                ),
            ),
        ]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_tables("mykeyspace", None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].table_name, "t1");
        assert_eq!(items[0].status, Some("ACTIVE".to_string()));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"nextToken":"cursor-a","keyspaceName":"mykeyspace"}"#,
                ),
                json_response(
                    200,
                    r#"{"tables":[{"keyspaceName":"mykeyspace","tableName":"t2","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t2"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t2"}"#,
                ),
                json_response(
                    200,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t2","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t2"}"#,
                ),
            ),
        ]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_tables("mykeyspace", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"maxResults":1,"keyspaceName":"mykeyspace"}"#),
                json_response(
                    200,
                    r#"{"tables":[{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1"}],"nextToken":"cursor-b"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t1"}"#,
                ),
                json_response(
                    200,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1"}"#,
                ),
            ),
        ]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_tables("mykeyspace", Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("cursor-b".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_propagates_discovery_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"keyspaceName":"mykeyspace"}"#),
            json_error_response("ResourceNotFoundException", "keyspace not found"),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_tables("mykeyspace", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "keyspace not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_propagates_fan_out_get_table_errors() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"keyspaceName":"mykeyspace"}"#),
                json_response(
                    200,
                    r#"{"tables":[{"keyspaceName":"mykeyspace","tableName":"t1","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/t1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"keyspaceName":"mykeyspace","tableName":"t1"}"#,
                ),
                json_error_response("InternalServerException", "internal error"),
            ),
        ]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_tables("mykeyspace", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InternalServerException".to_string()));
                assert_eq!(message, "internal error");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_table_returns_full_detail_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"keyspaceName":"mykeyspace","tableName":"mytable"}"#,
            ),
            json_response(
                200,
                r#"{"keyspaceName":"mykeyspace","tableName":"mytable","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/mykeyspace/table/mytable","status":"ACTIVE","creationTimestamp":1704067200,"capacitySpecification":{"throughputMode":"PROVISIONED","readCapacityUnits":100,"writeCapacityUnits":50},"encryptionSpecification":{"type":"CUSTOMER_MANAGED_KMS_KEY","kmsKeyIdentifier":"arn:aws:kms:us-east-1:123456789012:key/abc123"},"pointInTimeRecovery":{"status":"ENABLED"}}"#,
            ),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let table = client
            .get_table("mykeyspace", "mytable")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(table.keyspace_name, "mykeyspace");
        assert_eq!(table.table_name, "mytable");
        assert_eq!(table.status, Some("ACTIVE".to_string()));
        assert!(table.creation_timestamp.is_some());
        let capacity = table.capacity_specification.unwrap();
        assert_eq!(capacity.throughput_mode, "PROVISIONED");
        assert_eq!(capacity.read_capacity_units, Some(100));
        assert_eq!(capacity.write_capacity_units, Some(50));
        let encryption = table.encryption_specification.unwrap();
        assert_eq!(encryption.type_, "CUSTOMER_MANAGED_KMS_KEY");
        assert_eq!(
            encryption.kms_key_identifier,
            Some("arn:aws:kms:us-east-1:123456789012:key/abc123".to_string())
        );
        assert_eq!(table.point_in_time_recovery, Some(true));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_table_returns_minimal_detail_when_optional_fields_absent() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"keyspaceName":"ks","tableName":"tbl"}"#),
            json_response(
                200,
                r#"{"keyspaceName":"ks","tableName":"tbl","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks/table/tbl"}"#,
            ),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let table = client.get_table("ks", "tbl").await.unwrap().unwrap();

        assert_eq!(table.keyspace_name, "ks");
        assert!(table.status.is_none());
        assert!(table.creation_timestamp.is_none());
        assert!(table.capacity_specification.is_none());
        assert!(table.encryption_specification.is_none());
        assert!(table.point_in_time_recovery.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_table_maps_point_in_time_recovery_disabled_to_false() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"keyspaceName":"ks","tableName":"tbl"}"#),
            json_response(
                200,
                r#"{"keyspaceName":"ks","tableName":"tbl","resourceArn":"arn:aws:cassandra:us-east-1:123456789012:/keyspace/ks/table/tbl","pointInTimeRecovery":{"status":"DISABLED"}}"#,
            ),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let table = client.get_table("ks", "tbl").await.unwrap().unwrap();

        assert_eq!(table.point_in_time_recovery, Some(false));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_table_returns_none_on_resource_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"keyspaceName":"ks","tableName":"missing"}"#),
            json_error_response("ResourceNotFoundException", "table not found"),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let table = client.get_table("ks", "missing").await.unwrap();

        assert!(table.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_table_propagates_other_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"keyspaceName":"ks","tableName":"tbl"}"#),
            json_error_response("AccessDeniedException", "not authorized"),
        )]);
        let client = KeyspacesClient::new(&sdk_config(http_client.clone()));

        let err = client.get_table("ks", "tbl").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("AccessDeniedException".to_string()));
                assert_eq!(message, "not authorized");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
