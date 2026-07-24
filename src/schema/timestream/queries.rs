use async_graphql::{Context, Object, Result};

use crate::aws::timestream::TimestreamClient;
use crate::schema::pagination::Page;
use crate::schema::timestream::types::{TimestreamDatabase, TimestreamTable};

#[derive(Default)]
pub struct TimestreamQuery;

#[Object]
impl TimestreamQuery {
    /// Lists Timestream databases, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn timestream_databases(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TimestreamDatabase>> {
        let client = ctx.data::<TimestreamClient>()?;
        let (dbs, next_token) = client.list_databases(limit, next_token).await?;
        Ok(Page {
            items: dbs.into_iter().map(TimestreamDatabase::from).collect(),
            next_token,
        })
    }

    /// Lists tables in a database, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn timestream_tables(
        &self,
        ctx: &Context<'_>,
        database_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TimestreamTable>> {
        let client = ctx.data::<TimestreamClient>()?;
        let (tables, next_token) = client
            .list_tables(&database_name, limit, next_token)
            .await?;
        Ok(Page {
            items: tables.into_iter().map(TimestreamTable::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use aws_config::SdkConfig;

    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::aws::timestream::TimestreamClient;
    use crate::schema::test_util::build_query_schema;

    use super::TimestreamQuery;

    // Base endpoint used for the initial `DescribeEndpoints` call, resolved
    // from the region alone; every other op is sent to `DISCOVERED_BASE`
    // instead, resolved from that call's `Address` field (precedent:
    // `src/aws/timestream.rs`'s own test module).
    const DISCOVERY_BASE: &str = "https://ingest.timestream.us-east-1.amazonaws.com";
    const DISCOVERED_BASE: &str = "https://discovered.timestream.us-east-1.amazonaws.com";

    // `sdk_config` alone lacks a `sleep_impl`/`time_source`, which
    // `with_endpoint_discovery_enabled` requires (only client in the sweep
    // that calls it) — same fix as the aws-layer test module.
    fn discovery_sdk_config(http_client: StaticReplayClient) -> SdkConfig {
        sdk_config(http_client)
            .to_builder()
            .sleep_impl(aws_smithy_async::rt::sleep::TokioSleep::new())
            .time_source(aws_smithy_async::time::SystemTimeSource::new())
            .build()
    }

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
    async fn timestream_databases_forwards_limit_and_maps_fields() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(&format!("{DISCOVERED_BASE}/"), r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Databases":[{"DatabaseName":"db1","Arn":"arn:aws:timestream:us-east-1:111111111111:database/db1","TableCount":3,"KmsKeyId":"key1","CreationTime":1700000000,"LastUpdatedTime":1700003600}],"NextToken":"page2"}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(TimestreamQuery)
            .data(TimestreamClient::new(&discovery_sdk_config(
                http_client.clone(),
            )))
            .finish();

        let res = schema
            .execute(
                r#"{ timestreamDatabases(limit: 1) { items { databaseName arn tableCount kmsKeyId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(
            json["timestreamDatabases"]["items"][0]["databaseName"],
            "db1"
        );
        assert_eq!(
            json["timestreamDatabases"]["items"][0]["arn"],
            "arn:aws:timestream:us-east-1:111111111111:database/db1"
        );
        assert_eq!(json["timestreamDatabases"]["items"][0]["tableCount"], 3);
        assert_eq!(json["timestreamDatabases"]["items"][0]["kmsKeyId"], "key1");
        assert_eq!(json["timestreamDatabases"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn timestream_tables_forwards_database_name_and_maps_fields() {
        let http_client = StaticReplayClient::new(vec![
            describe_endpoints_event(),
            ReplayEvent::new(
                request(
                    &format!("{DISCOVERED_BASE}/"),
                    r#"{"DatabaseName":"db1","MaxResults":1}"#,
                ),
                json_response(
                    200,
                    r#"{"Tables":[{"TableName":"t1","DatabaseName":"db1","TableStatus":"ACTIVE","RetentionProperties":{"MemoryStoreRetentionPeriodInHours":24,"MagneticStoreRetentionPeriodInDays":7}}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(TimestreamQuery)
            .data(TimestreamClient::new(&discovery_sdk_config(
                http_client.clone(),
            )))
            .finish();

        let res = schema
            .execute(
                r#"{ timestreamTables(databaseName: "db1", limit: 1) { items { tableName databaseName tableStatus retentionProperties { memoryStoreRetentionPeriodInHours magneticStoreRetentionPeriodInDays } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["timestreamTables"]["items"][0]["tableName"], "t1");
        assert_eq!(json["timestreamTables"]["items"][0]["databaseName"], "db1");
        assert_eq!(
            json["timestreamTables"]["items"][0]["tableStatus"],
            "ACTIVE"
        );
        assert_eq!(
            json["timestreamTables"]["items"][0]["retentionProperties"]
                ["memoryStoreRetentionPeriodInHours"],
            24
        );
        assert!(json["timestreamTables"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
