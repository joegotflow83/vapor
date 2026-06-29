use async_graphql::{Context, Object, Result};

use crate::aws::keyspaces::KeyspacesClient;
use crate::schema::keyspaces::types::{KeyspacesKeyspace, KeyspacesTable};

#[derive(Default)]
pub struct KeyspacesQuery;

#[Object]
impl KeyspacesQuery {
    async fn keyspaces_keyspaces(&self, ctx: &Context<'_>) -> Result<Vec<KeyspacesKeyspace>> {
        let client = ctx.data::<KeyspacesClient>()?;
        let keyspaces = client.list_keyspaces().await?;
        Ok(keyspaces.into_iter().map(KeyspacesKeyspace::from).collect())
    }

    async fn keyspaces_tables(
        &self,
        ctx: &Context<'_>,
        keyspace_name: String,
    ) -> Result<Vec<KeyspacesTable>> {
        let client = ctx.data::<KeyspacesClient>()?;
        let tables = client.list_tables(&keyspace_name).await?;
        Ok(tables.into_iter().map(KeyspacesTable::from).collect())
    }

    async fn keyspaces_table(
        &self,
        ctx: &Context<'_>,
        keyspace_name: String,
        table_name: String,
    ) -> Result<Option<KeyspacesTable>> {
        let client = ctx.data::<KeyspacesClient>()?;
        let table = client.get_table(&keyspace_name, &table_name).await?;
        Ok(table.map(KeyspacesTable::from))
    }
}
