use async_graphql::{Context, Object, Result};

use crate::aws::qldb::QldbClient;
use crate::schema::qldb::types::{QldbJournalExport, QldbLedger};

#[derive(Default)]
pub struct QldbQuery;

#[Object]
impl QldbQuery {
    async fn qldb_ledgers(&self, ctx: &Context<'_>) -> Result<Vec<QldbLedger>> {
        let client = ctx.data::<QldbClient>()?;
        let ledgers = client.list_ledgers().await?;
        Ok(ledgers.into_iter().map(QldbLedger::from).collect())
    }

    async fn qldb_ledger(&self, ctx: &Context<'_>, name: String) -> Result<Option<QldbLedger>> {
        let client = ctx.data::<QldbClient>()?;
        let ledger = client.describe_ledger(&name).await?;
        Ok(ledger.map(QldbLedger::from))
    }

    async fn qldb_journal_exports(
        &self,
        ctx: &Context<'_>,
        ledger_name: String,
    ) -> Result<Vec<QldbJournalExport>> {
        let client = ctx.data::<QldbClient>()?;
        let exports = client.list_journal_s3_exports(&ledger_name).await?;
        Ok(exports.into_iter().map(QldbJournalExport::from).collect())
    }
}
