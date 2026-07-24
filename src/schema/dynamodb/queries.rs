use async_graphql::{Context, Object, Result};

use crate::aws::dynamodb::DynamodbClient;
use crate::schema::dynamodb::types::{DynamoScanResult, DynamoTable};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct DynamodbQuery;

#[Object]
impl DynamodbQuery {
    /// List DynamoDB table names in the account/region.
    async fn dynamo_tables(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<String>> {
        let client = ctx.data::<DynamodbClient>()?;
        let (items, next_token) = client.list_tables(limit, next_token).await?;
        Ok(Page { items, next_token })
    }

    /// Describe a single DynamoDB table by name, including indexes and stream settings.
    async fn dynamo_table(&self, ctx: &Context<'_>, name: String) -> Result<DynamoTable> {
        let client = ctx.data::<DynamodbClient>()?;
        let desc = client.describe_table(&name).await?;
        Ok(DynamoTable::from(desc))
    }

    /// Scan a DynamoDB table (single page). Items are returned as JSON strings.
    async fn dynamo_scan(
        &self,
        ctx: &Context<'_>,
        table: String,
        filter_expression: Option<String>,
        limit: Option<i32>,
    ) -> Result<DynamoScanResult> {
        let client = ctx.data::<DynamodbClient>()?;
        let output = client
            .scan(&table, filter_expression.as_deref(), limit)
            .await?;
        Ok(DynamoScanResult::from_scan_output(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    const ENDPOINT: &str = "https://dynamodb.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn dynamo_tables_maps_items_and_forwards_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Limit":1}"#),
            json_response(
                200,
                r#"{"TableNames":["t1"],"LastEvaluatedTableName":"t1"}"#,
            ),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DynamodbQuery).data(client).finish();

        let res = schema
            .execute(r#"{ dynamoTables(limit: 1) { items nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["dynamoTables"]["items"], serde_json::json!(["t1"]));
        assert_eq!(data["dynamoTables"]["nextToken"], "t1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn dynamo_table_maps_detail() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"TableName":"my-table"}"#),
            json_response(
                200,
                r#"{"Table":{"TableName":"my-table","TableStatus":"ACTIVE","ItemCount":42}}"#,
            ),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DynamodbQuery).data(client).finish();

        let res = schema
            .execute(r#"{ dynamoTable(name: "my-table") { name status itemCount } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["dynamoTable"]["name"], "my-table");
        assert_eq!(data["dynamoTable"]["status"], "ACTIVE");
        assert_eq!(data["dynamoTable"]["itemCount"], 42);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn dynamo_scan_maps_items_and_forwards_filter_and_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                r#"{"TableName":"my-table","Limit":5,"FilterExpression":"attr = :v"}"#,
            ),
            json_response(
                200,
                r#"{"Items":[{"id":{"S":"row1"}}],"Count":1,"ScannedCount":1}"#,
            ),
        )]);
        let client = DynamodbClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(DynamodbQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ dynamoScan(table: "my-table", filterExpression: "attr = :v", limit: 5) { items count scannedCount lastEvaluatedKey } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["dynamoScan"]["count"], 1);
        assert_eq!(data["dynamoScan"]["scannedCount"], 1);
        assert_eq!(
            data["dynamoScan"]["lastEvaluatedKey"],
            serde_json::Value::Null
        );
        let items = data["dynamoScan"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        let parsed: serde_json::Value = serde_json::from_str(items[0].as_str().unwrap()).unwrap();
        assert_eq!(parsed["id"], "row1");
        http_client.relaxed_requests_match();
    }
}
