use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use crate::aws::glue::GlueClient;
use crate::schema::glue::types::{GlueCrawler, GlueDatabase, GlueJob, GlueTable};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct GlueQuery;

#[Object]
impl GlueQuery {
    /// Lists Glue databases. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn glue_databases(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<GlueDatabase>> {
        let client = ctx.data::<GlueClient>()?;
        let (databases, next_token) = client.get_databases(limit, next_token).await?;
        Ok(Page {
            items: databases.iter().map(GlueDatabase::from).collect(),
            next_token,
        })
    }

    /// Lists tables in a Glue database. `limit` caps the total number of
    /// results (default unlimited); pass `nextToken` from a prior page to
    /// resume.
    async fn glue_tables(
        &self,
        ctx: &Context<'_>,
        database_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<GlueTable>> {
        let client = ctx.data::<GlueClient>()?;
        let (tables, next_token) = client
            .get_tables(&database_name, limit, next_token)
            .await?;
        Ok(Page {
            items: tables.iter().map(GlueTable::from).collect(),
            next_token,
        })
    }

    /// Lists Glue crawlers. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn glue_crawlers(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<GlueCrawler>> {
        let client = ctx.data::<GlueClient>()?;
        let (crawlers, next_token) = client.get_crawlers(limit, next_token).await?;
        Ok(Page {
            items: crawlers.iter().map(GlueCrawler::from).collect(),
            next_token,
        })
    }

    /// Lists Glue job definitions. `limit` caps the total number of results
    /// (default unlimited); pass `nextToken` from a prior page to resume.
    async fn glue_jobs(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<GlueJob>> {
        let client = ctx.data::<GlueClient>()?;
        let (jobs, next_token) = client.get_jobs(limit, next_token).await?;

        let futures: Vec<_> = jobs
            .iter()
            .map(|job| async {
                let name = job.name().unwrap_or_default();
                let runs = client.get_job_runs(name, 1).await;
                let last_status = runs.ok().and_then(|r| {
                    r.first()
                        .and_then(|run| run.job_run_state().map(|s| s.as_str().to_string()))
                });
                GlueJob::from_sdk(job, last_status)
            })
            .collect();

        Ok(Page {
            items: join_all(futures).await,
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::glue::GlueClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::GlueQuery;

    const ENDPOINT: &str = "https://glue.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn glue_databases_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"DatabaseList":[{"Name":"db-a","CatalogId":"111111111111","Description":"Test db","LocationUri":"s3://bucket/path","CreateTime":1700000000}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(GlueQuery)
            .data(GlueClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ glueDatabases(limit: 1) { items { name catalogId description locationUri createTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["glueDatabases"]["items"];
        assert_eq!(items[0]["name"], "db-a");
        assert_eq!(items[0]["catalogId"], "111111111111");
        assert_eq!(items[0]["description"], "Test db");
        assert_eq!(items[0]["locationUri"], "s3://bucket/path");
        assert!(items[0]["createTime"].is_string());
        assert_eq!(json["glueDatabases"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn glue_tables_forwards_database_name_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DatabaseName":"db-1","MaxResults":1}"#),
            json_response(
                200,
                r#"{"TableList":[{"Name":"tbl-a","DatabaseName":"db-1","TableType":"EXTERNAL_TABLE","StorageDescriptor":{"Location":"s3://bucket/table","Columns":[{"Name":"id","Type":"bigint"}]}}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(GlueQuery)
            .data(GlueClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ glueTables(databaseName: "db-1", limit: 1) { items { name databaseName tableType location columns { name colType } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["glueTables"]["items"];
        assert_eq!(items[0]["name"], "tbl-a");
        assert_eq!(items[0]["databaseName"], "db-1");
        assert_eq!(items[0]["tableType"], "EXTERNAL_TABLE");
        assert_eq!(items[0]["location"], "s3://bucket/table");
        assert_eq!(items[0]["columns"][0]["name"], "id");
        assert_eq!(items[0]["columns"][0]["colType"], "bigint");
        assert_eq!(json["glueTables"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn glue_crawlers_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Crawlers":[{"Name":"crawler-a","Role":"arn:aws:iam::111111111111:role/GlueRole","DatabaseName":"db-1","State":"READY","LastCrawl":{"Status":"SUCCEEDED","StartTime":1700000000},"CreationTime":1700000100}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(GlueQuery)
            .data(GlueClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ glueCrawlers(limit: 1) { items { name role databaseName state lastCrawlStatus lastCrawlTime creationTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["glueCrawlers"]["items"];
        assert_eq!(items[0]["name"], "crawler-a");
        assert_eq!(
            items[0]["role"],
            "arn:aws:iam::111111111111:role/GlueRole"
        );
        assert_eq!(items[0]["databaseName"], "db-1");
        assert_eq!(items[0]["state"], "READY");
        assert_eq!(items[0]["lastCrawlStatus"], "SUCCEEDED");
        assert!(items[0]["lastCrawlTime"].is_string());
        assert!(items[0]["creationTime"].is_string());
        assert_eq!(json["glueCrawlers"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn glue_jobs_fans_out_to_job_runs_and_maps_last_status() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"Jobs":[{"Name":"job-a","Role":"arn:aws:iam::111111111111:role/GlueRole","Command":{"Name":"glueetl"},"MaxCapacity":10.0,"GlueVersion":"3.0","CreatedOn":1700000000,"LastModifiedOn":1700000100}],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"JobName":"job-a","MaxResults":1}"#),
                json_response(200, r#"{"JobRuns":[{"Id":"run-1","JobRunState":"SUCCEEDED"}]}"#),
            ),
        ]);
        let schema = build_query_schema(GlueQuery)
            .data(GlueClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ glueJobs(limit: 1) { items { name role commandName maxCapacity glueVersion lastRunStatus createdOn lastModifiedOn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["glueJobs"]["items"];
        assert_eq!(items[0]["name"], "job-a");
        assert_eq!(
            items[0]["role"],
            "arn:aws:iam::111111111111:role/GlueRole"
        );
        assert_eq!(items[0]["commandName"], "glueetl");
        assert_eq!(items[0]["maxCapacity"], 10.0);
        assert_eq!(items[0]["glueVersion"], "3.0");
        assert_eq!(items[0]["lastRunStatus"], "SUCCEEDED");
        assert!(items[0]["createdOn"].is_string());
        assert!(items[0]["lastModifiedOn"].is_string());
        assert_eq!(json["glueJobs"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
