use async_graphql::{Context, Object, Result};

use crate::aws::qldb::QldbClient;
use crate::schema::pagination::Page;
use crate::schema::qldb::types::{QldbJournalExport, QldbLedger};

#[derive(Default)]
pub struct QldbQuery;

#[Object]
impl QldbQuery {
    /// Lists ledgers, optionally capped at `limit` results (default unlimited).
    async fn qldb_ledgers(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<QldbLedger>> {
        let client = ctx.data::<QldbClient>()?;
        let (ledgers, next_token) = client.list_ledgers(limit, next_token).await?;
        Ok(Page {
            items: ledgers.into_iter().map(QldbLedger::from).collect(),
            next_token,
        })
    }

    async fn qldb_ledger(&self, ctx: &Context<'_>, name: String) -> Result<Option<QldbLedger>> {
        let client = ctx.data::<QldbClient>()?;
        let ledger = client.describe_ledger(&name).await?;
        Ok(ledger.map(QldbLedger::from))
    }

    /// Lists journal S3 exports for a ledger, optionally capped at `limit` results (default unlimited).
    async fn qldb_journal_exports(
        &self,
        ctx: &Context<'_>,
        ledger_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<QldbJournalExport>> {
        let client = ctx.data::<QldbClient>()?;
        let (exports, next_token) = client.list_journal_s3_exports(&ledger_name, limit, next_token).await?;
        Ok(Page {
            items: exports.into_iter().map(QldbJournalExport::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `QldbClient` method each (see `src/aws/qldb.rs`'s own test module for the
// discovery-then-fan-out/pagination/limit/error-mapping behavior) — only
// light smoke tests are needed here per the resolver-layer sweep's stated
// scope.
#[cfg(test)]
mod tests {
    use crate::aws::qldb::QldbClient;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::QldbQuery;

    const BASE: &str = "https://qldb.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn qldb_ledgers_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers?max_results=1"), ""),
                json_response(
                    200,
                    r#"{"Ledgers":[{"Name":"ledger-a","State":"ACTIVE","CreationDateTime":1700000000}],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/ledgers/ledger-a"), ""),
                json_response(
                    200,
                    r#"{"Name":"ledger-a","Arn":"arn:aws:qldb:us-east-1:123456789012:ledger/ledger-a","State":"ACTIVE","CreationDateTime":1700000000,"PermissionsMode":"STANDARD","DeletionProtection":true}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    &format!("{BASE}/tags/arn%3Aaws%3Aqldb%3Aus-east-1%3A123456789012%3Aledger%2Fledger-a"),
                    "",
                ),
                json_response(200, r#"{"Tags":{"env":"prod"}}"#),
            ),
        ]);
        let schema = build_query_schema(QldbQuery)
            .data(QldbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ qldbLedgers(limit: 1) { items { name arn state permissionsMode deletionProtection tags { key value } } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["qldbLedgers"]["items"];
        assert_eq!(items[0]["name"], "ledger-a");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:qldb:us-east-1:123456789012:ledger/ledger-a"
        );
        assert_eq!(items[0]["state"], "ACTIVE");
        assert_eq!(items[0]["permissionsMode"], "STANDARD");
        assert_eq!(items[0]["deletionProtection"], true);
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["qldbLedgers"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn qldb_ledger_returns_none_when_not_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers/missing-ledger"), ""),
            json_error_response("ResourceNotFoundException", "no such ledger"),
        )]);
        let schema = build_query_schema(QldbQuery)
            .data(QldbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ qldbLedger(name: "missing-ledger") { name } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert!(json["qldbLedger"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn qldb_journal_exports_maps_items_for_given_ledger() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/ledgers/ledger-j/journal-s3-exports"), ""),
            json_response(
                200,
                r#"{"JournalS3Exports":[{"LedgerName":"ledger-j","ExportId":"export-1","ExportCreationTime":1700000000,"Status":"COMPLETED","InclusiveStartTime":1699990000,"ExclusiveEndTime":1700000000,"OutputFormat":"JSON"}]}"#,
            ),
        )]);
        let schema = build_query_schema(QldbQuery)
            .data(QldbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ qldbJournalExports(ledgerName: "ledger-j") { items { ledgerName exportId status outputFormat } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["qldbJournalExports"]["items"];
        assert_eq!(items[0]["ledgerName"], "ledger-j");
        assert_eq!(items[0]["exportId"], "export-1");
        assert_eq!(items[0]["status"], "COMPLETED");
        assert_eq!(items[0]["outputFormat"], "JSON");
        assert!(json["qldbJournalExports"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
