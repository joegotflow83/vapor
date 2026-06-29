use async_graphql::{Context, Object, Result};

use crate::aws::timestream::TimestreamClient;
use crate::schema::timestream::types::{TimestreamDatabase, TimestreamTable};

#[derive(Default)]
pub struct TimestreamQuery;

#[Object]
impl TimestreamQuery {
    async fn timestream_databases(&self, ctx: &Context<'_>) -> Result<Vec<TimestreamDatabase>> {
        let client = ctx.data::<TimestreamClient>()?;
        let dbs = client.list_databases().await?;
        Ok(dbs.into_iter().map(TimestreamDatabase::from).collect())
    }

    async fn timestream_tables(
        &self,
        ctx: &Context<'_>,
        database_name: String,
    ) -> Result<Vec<TimestreamTable>> {
        let client = ctx.data::<TimestreamClient>()?;
        let tables = client.list_tables(&database_name).await?;
        Ok(tables.into_iter().map(TimestreamTable::from).collect())
    }
}
