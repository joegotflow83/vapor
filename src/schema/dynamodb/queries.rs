use async_graphql::{Context, Object, Result};

use crate::aws::dynamodb::DynamodbClient;
use crate::schema::dynamodb::types::{DynamoScanResult, DynamoTable};

#[derive(Default)]
pub struct DynamodbQuery;

#[Object]
impl DynamodbQuery {
    /// List all DynamoDB table names in the account/region.
    async fn dynamo_tables(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let client = ctx.data::<DynamodbClient>()?;
        Ok(client.list_tables().await?)
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
