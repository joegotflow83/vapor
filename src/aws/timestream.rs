use aws_config::SdkConfig;
use tokio::sync::OnceCell;

use crate::error::VaporError;

#[derive(Debug)]
pub struct TimestreamDatabaseInfo {
    pub database_name: Option<String>,
    pub arn: Option<String>,
    pub table_count: Option<i64>,
    pub kms_key_id: Option<String>,
    pub creation_time: Option<aws_smithy_types::DateTime>,
    pub last_updated_time: Option<aws_smithy_types::DateTime>,
}

#[derive(Debug)]
pub struct TimestreamRetentionInfo {
    pub memory_store_retention_period_in_hours: Option<i64>,
    pub magnetic_store_retention_period_in_days: Option<i64>,
}

#[derive(Debug)]
pub struct TimestreamTableInfo {
    pub database_name: Option<String>,
    pub table_name: Option<String>,
    pub table_status: Option<String>,
    pub arn: Option<String>,
    pub creation_time: Option<aws_smithy_types::DateTime>,
    pub last_updated_time: Option<aws_smithy_types::DateTime>,
    pub retention_properties: Option<TimestreamRetentionInfo>,
}

pub struct TimestreamClient {
    // Not endpoint-discovery-enabled — kept as the base for cloning into
    // `discovered`. Never used to send requests directly.
    base: aws_sdk_timestreamwrite::Client,
    // The generated SDK client's own doc comment states endpoint discovery
    // "MUST be called to construct a working client" for every operation on
    // this service (there's no fixed regional endpoint; the real one is
    // fetched via `DescribeEndpoints`). That call is async and hits the
    // network, so it can't happen in `new()`: `build_schema` (this
    // constructor's only caller) is a sync, no-I/O function also used by
    // `gen_docs` to introspect the schema with a bare `SdkConfig` (no
    // credentials) — an eager network call here would break doc generation.
    // Discovery is deferred to first real use and cached here instead.
    discovered: OnceCell<aws_sdk_timestreamwrite::Client>,
}

impl TimestreamClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            base: aws_sdk_timestreamwrite::Client::new(config),
            discovered: OnceCell::new(),
        }
    }

    async fn client(&self) -> Result<&aws_sdk_timestreamwrite::Client, VaporError> {
        self.discovered
            .get_or_try_init(|| async {
                let (client, reloader) = self
                    .base
                    .clone()
                    .with_endpoint_discovery_enabled()
                    .await
                    .map_err(|e| {
                        VaporError::InvalidInput(format!(
                            "Timestream endpoint discovery failed: {e}"
                        ))
                    })?;
                // Refreshes the cached endpoint in the background before it
                // expires; stops automatically once every clone of `client`
                // (held only by this cache) is dropped.
                tokio::spawn(reloader.reload_task());
                Ok(client)
            })
            .await
    }

    /// Lists Timestream databases, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListDatabases`
    /// has both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-timestreamwrite` 1.104.0's
    /// `operation/list_databases/_list_databases_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself
    /// (sso_admin/ram hand-rolled-loop pattern).
    pub async fn list_databases(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TimestreamDatabaseInfo>, Option<String>), VaporError> {
        let client = self.client().await?;
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = client.list_databases();
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.databases.unwrap_or_default().into_iter().map(|db| {
                TimestreamDatabaseInfo {
                    database_name: db.database_name,
                    arn: db.arn,
                    table_count: Some(db.table_count),
                    kms_key_id: db.kms_key_id,
                    creation_time: db.creation_time,
                    last_updated_time: db.last_updated_time,
                }
            }));
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }

    /// Lists tables in a database, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `ListTables` has
    /// both `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-timestreamwrite` 1.104.0's
    /// `operation/list_tables/_list_tables_input.rs`), so `limit` is
    /// capped to the remaining budget on the request itself.
    pub async fn list_tables(
        &self,
        database_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<TimestreamTableInfo>, Option<String>), VaporError> {
        let client = self.client().await?;
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let remaining = limit.map(|l| l - items.len() as i32);
            if remaining.is_some_and(|r| r <= 0) {
                break;
            }

            let mut req = client.list_tables().database_name(database_name);
            if let Some(t) = &token {
                req = req.next_token(t);
            }
            if let Some(r) = remaining {
                req = req.max_results(r);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.tables.unwrap_or_default().into_iter().map(|table| {
                TimestreamTableInfo {
                    database_name: table.database_name,
                    table_name: table.table_name,
                    table_status: table.table_status.map(|s| s.as_str().to_string()),
                    arn: table.arn,
                    creation_time: table.creation_time,
                    last_updated_time: table.last_updated_time,
                    retention_properties: table.retention_properties.map(|r| {
                        TimestreamRetentionInfo {
                            memory_store_retention_period_in_hours: Some(
                                r.memory_store_retention_period_in_hours,
                            ),
                            magnetic_store_retention_period_in_days: Some(
                                r.magnetic_store_retention_period_in_days,
                            ),
                        }
                    }),
                }
            }));
            token = output.next_token;

            if token.is_none() || limit.is_some_and(|l| items.len() as i32 >= l) {
                break;
            }
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // Base endpoint used for the initial `DescribeEndpoints` call, resolved
    // from the region alone (verified against the pinned
    // `aws-sdk-timestreamwrite` 1.104.0's `config/endpoint.rs` for
    // us-east-1/no-FIPS/no-dualstack). Every other op is only ever sent to
    // `DISCOVERED_BASE` below, which `with_endpoint_discovery_enabled`
    // resolves from the `DescribeEndpoints` response's `Address` field, not
    // this one.
    const DISCOVERY_BASE: &str = "https://ingest.timestream.us-east-1.amazonaws.com";
    const DISCOVERED_BASE: &str = "https://discovered.timestream.us-east-1.amazonaws.com";

    /// `test_util::sdk_config` leaves `sleep_impl`/`time_source` unset —
    /// fine for every other `src/aws/*.rs` client, but `with_endpoint_
    /// discovery_enabled` panics-free-errors with "endpoint discovery
    /// requires the client config to have a sleep impl" without them (it's
    /// the only client in this sweep that calls it). No other file needs
    /// this, so it's kept local here rather than added to the shared helper.
    fn discovery_sdk_config(http_client: StaticReplayClient) -> SdkConfig {
        sdk_config(http_client)
            .to_builder()
            .sleep_impl(aws_smithy_async::rt::sleep::TokioSleep::new())
            .time_source(aws_smithy_async::time::SystemTimeSource::new())
            .build()
    }

    // Every wrapper method calls `self.client()` first, which performs
    // (and caches, per `TimestreamClient` instance) exactly one
    // `DescribeEndpoints` round trip before the real operation can be sent
    // — so every test needs this as its first `ReplayEvent`. `ser_
    // describe_endpoints_input` always serializes a literal `"{}"` body
    // (no input fields on this op).
    fn describe_endpoints_event() -> ReplayEvent {
        ReplayEvent::new(
            request(&format!("{DISCOVERY_BASE}/"), "{}"),
            json_response(
                200,
                r#"{"Endpoints":[{"Address":"discovered.timestream.us-east-1.amazonaws.com","CachePeriodInMinutes":1440}]}"#,
            ),
        )
    }

    #[tokio::test]
    async fn list_databases_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), "{}"),
                json_response(
                    200,
                    r#"{"Databases":[{"DatabaseName":"db1","Arn":"arn:aws:timestream:us-east-1:111111111111:database/db1","TableCount":3,"KmsKeyId":"key1","CreationTime":1700000000,"LastUpdatedTime":1700003600}]}"#,
                ),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client.list_databases(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].database_name, Some("db1".to_string()));
        assert_eq!(
            items[0].arn,
            Some("arn:aws:timestream:us-east-1:111111111111:database/db1".to_string())
        );
        assert_eq!(items[0].table_count, Some(3));
        assert_eq!(items[0].kms_key_id, Some("key1".to_string()));
        assert_eq!(
            items[0].creation_time,
            Some(aws_smithy_types::DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(
            items[0].last_updated_time,
            Some(aws_smithy_types::DateTime::from_secs(1_700_003_600))
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_databases_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Databases":[{"DatabaseName":"db1","TableCount":1}],"NextToken":"page2"}"#,
                ),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client.list_databases(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_databases_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"Databases":[{"DatabaseName":"db1","TableCount":1},{"DatabaseName":"db2","TableCount":2}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{DISCOVERED_BASE}/"),
                    r#"{"NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(200, r#"{"Databases":[{"DatabaseName":"db3","TableCount":3}]}"#),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client.list_databases(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_databases_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), r#"{"NextToken":"cursor-a"}"#),
                json_response(200, r#"{"Databases":[]}"#),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client
            .list_databases(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_databases_propagates_errors() {
        // `ValidationException`, not a throttling-classified code (see
        // memory gotcha 1: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), "{}"),
                json_error_response("ValidationException", "bad database name"),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let err = client.list_databases(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "bad database name");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_databases_missing_table_count_defaults_to_zero() {
        // The pinned crate's `DatabaseBuilder::build()` does `self.table_count
        // .unwrap_or_default()` unconditionally (bare `i64` field, not
        // `Option<i64>`) — a missing `TableCount` key deserializes to `0`,
        // not absence.
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), "{}"),
                json_response(200, r#"{"Databases":[{"DatabaseName":"db1"}]}"#),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, _token) = client.list_databases(None, None).await.unwrap();

        assert_eq!(items[0].table_count, Some(0));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), r#"{"DatabaseName":"db1"}"#),
                json_response(
                    200,
                    r#"{"Tables":[{"TableName":"t1","DatabaseName":"db1","Arn":"arn:aws:timestream:us-east-1:111111111111:database/db1/table/t1","TableStatus":"ACTIVE","RetentionProperties":{"MemoryStoreRetentionPeriodInHours":24,"MagneticStoreRetentionPeriodInDays":7},"CreationTime":1700000000,"LastUpdatedTime":1700003600}]}"#,
                ),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client.list_tables("db1", None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].table_name, Some("t1".to_string()));
        assert_eq!(items[0].database_name, Some("db1".to_string()));
        assert_eq!(items[0].table_status, Some("ACTIVE".to_string()));
        assert_eq!(
            items[0].arn,
            Some("arn:aws:timestream:us-east-1:111111111111:database/db1/table/t1".to_string())
        );
        let retention = items[0].retention_properties.as_ref().unwrap();
        assert_eq!(retention.memory_store_retention_period_in_hours, Some(24));
        assert_eq!(retention.magnetic_store_retention_period_in_days, Some(7));
        assert_eq!(
            items[0].creation_time,
            Some(aws_smithy_types::DateTime::from_secs(1_700_000_000))
        );
        assert_eq!(
            items[0].last_updated_time,
            Some(aws_smithy_types::DateTime::from_secs(1_700_003_600))
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_missing_retention_properties_is_none() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), r#"{"DatabaseName":"db1"}"#),
                json_response(
                    200,
                    r#"{"Tables":[{"TableName":"t1","DatabaseName":"db1"}]}"#,
                ),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, _token) = client.list_tables("db1", None, None).await.unwrap();

        assert!(items[0].retention_properties.is_none());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(
                    &format!("{DISCOVERED_BASE}/"),
                    r#"{"DatabaseName":"db1","MaxResults":1}"#,
                ),
                json_response(
                    200,
                    r#"{"Tables":[{"TableName":"t1","DatabaseName":"db1"}],"NextToken":"page2"}"#,
                ),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client.list_tables("db1", Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(
                    &format!("{DISCOVERED_BASE}/"),
                    r#"{"DatabaseName":"db1","MaxResults":10}"#,
                ),
                json_response(
                    200,
                    r#"{"Tables":[{"TableName":"t1","DatabaseName":"db1"},{"TableName":"t2","DatabaseName":"db1"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{DISCOVERED_BASE}/"),
                    r#"{"DatabaseName":"db1","NextToken":"p2","MaxResults":8}"#,
                ),
                json_response(200, r#"{"Tables":[{"TableName":"t3","DatabaseName":"db1"}]}"#),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client.list_tables("db1", Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(
                    &format!("{DISCOVERED_BASE}/"),
                    r#"{"DatabaseName":"db1","NextToken":"cursor-a"}"#,
                ),
                json_response(200, r#"{"Tables":[]}"#),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let (items, token) = client
            .list_tables("db1", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tables_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), r#"{"DatabaseName":"db1"}"#),
                json_error_response("ResourceNotFoundException", "database not found"),
            ),
        ]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let err = client.list_tables("db1", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "database not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn endpoint_discovery_failure_maps_to_invalid_input_error() {
        // `TimestreamClient::client()` maps a failed `with_endpoint_discovery_
        // enabled()` to `VaporError::InvalidInput`, not `VaporError::AwsSdk`
        // (there's no real operation to attribute an SDK error code to yet —
        // discovery itself never got a usable endpoint). Only 1 `ReplayEvent`
        // is consumed since the real op is never reached.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{DISCOVERY_BASE}/"), "{}"),
            json_error_response("InternalServerException", "discovery unavailable"),
        )]);
        let client = TimestreamClient::new(&discovery_sdk_config(http_client.clone()));

        let err = client.list_databases(None, None).await.unwrap_err();

        match err {
            VaporError::InvalidInput(message) => {
                assert!(message.contains("Timestream endpoint discovery failed"));
            }
            other => panic!("expected VaporError::InvalidInput, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}

