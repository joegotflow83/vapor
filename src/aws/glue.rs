use aws_config::SdkConfig;
use aws_sdk_glue::types::{Crawler, Database, Job, JobRun, Table};

use crate::error::VaporError;

pub struct GlueClient {
    inner: aws_sdk_glue::Client,
}

impl GlueClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_glue::Client::new(config),
        }
    }

    /// Lists Glue databases, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `GetDatabasesInput::max_results` (confirmed `Option<i32>`, verified
    /// against pinned `aws-sdk-glue` 1.152.0's
    /// `operation/get_databases/_get_databases_input.rs`) so capping is
    /// precise and server-side, unlike the client-side-truncation-only class
    /// (e.g. `xray.rs`).
    pub async fn get_databases(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Database>, Option<String>), VaporError> {
        let mut databases = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_databases();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - databases.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            databases.extend(output.database_list);
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if databases.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((databases, token))
    }

    /// Lists tables in a Glue database, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `limit` maps to
    /// `GetTablesInput::max_results` (same server-side-capping class as
    /// `get_databases` above).
    pub async fn get_tables(
        &self,
        database_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Table>, Option<String>), VaporError> {
        let mut tables = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_tables().database_name(database_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - tables.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            tables.extend(output.table_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if tables.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((tables, token))
    }

    /// Lists Glue crawlers, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` maps to
    /// `GetCrawlersInput::max_results` (same server-side-capping class as
    /// `get_databases` above).
    pub async fn get_crawlers(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Crawler>, Option<String>), VaporError> {
        let mut crawlers = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_crawlers();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - crawlers.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            crawlers.extend(output.crawlers.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if crawlers.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((crawlers, token))
    }

    /// Lists Glue job definitions, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `limit` maps to
    /// `GetJobsInput::max_results` (same server-side-capping class as
    /// `get_databases` above).
    pub async fn get_jobs(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Job>, Option<String>), VaporError> {
        let mut jobs = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.get_jobs();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - jobs.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;

            jobs.extend(output.jobs.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if jobs.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((jobs, token))
    }

    pub async fn get_job_runs(
        &self,
        job_name: &str,
        max_results: i32,
    ) -> Result<Vec<JobRun>, VaporError> {
        let output = self
            .inner
            .get_job_runs()
            .job_name(job_name)
            .max_results(max_results)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.job_runs().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://glue.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn get_databases_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"DatabaseList":[{"Name":"db-a"},{"Name":"db-b"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (databases, token) = client.get_databases(None, None).await.unwrap();

        assert_eq!(databases.len(), 2);
        assert_eq!(databases[0].name(), "db-a");
        assert_eq!(databases[1].name(), "db-b");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_databases_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"tok-1"}"#),
            json_response(200, r#"{"DatabaseList":[{"Name":"db-b"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (databases, token) = client
            .get_databases(None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(databases.len(), 1);
        assert_eq!(databases[0].name(), "db-b");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_databases_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"DatabaseList":[{"Name":"db-a"},{"Name":"db-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (databases, token) = client.get_databases(Some(2), None).await.unwrap();

        assert_eq!(databases.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_databases_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"DatabaseList":[{"Name":"db-a"},{"Name":"db-b"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"tok-3","MaxResults":1}"#),
                json_response(200, r#"{"DatabaseList":[{"Name":"db-c"}]}"#),
            ),
        ]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (databases, token) = client.get_databases(Some(3), None).await.unwrap();

        assert_eq!(databases.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_databases_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "invalid input"),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let err = client.get_databases(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidInputException"));
                assert_eq!(message, "invalid input");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_tables_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DatabaseName":"db-1"}"#),
            json_response(200, r#"{"TableList":[{"Name":"tbl-a"},{"Name":"tbl-b"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (tables, token) = client.get_tables("db-1", None, None).await.unwrap();

        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].name(), "tbl-a");
        assert_eq!(tables[1].name(), "tbl-b");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_tables_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DatabaseName":"db-1","NextToken":"tok-1"}"#),
            json_response(200, r#"{"TableList":[{"Name":"tbl-b"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (tables, token) = client
            .get_tables("db-1", None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name(), "tbl-b");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_tables_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DatabaseName":"db-1","MaxResults":2}"#),
            json_response(
                200,
                r#"{"TableList":[{"Name":"tbl-a"},{"Name":"tbl-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (tables, token) = client.get_tables("db-1", Some(2), None).await.unwrap();

        assert_eq!(tables.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_tables_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"DatabaseName":"db-1","MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"TableList":[{"Name":"tbl-a"},{"Name":"tbl-b"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    r#"{"DatabaseName":"db-1","NextToken":"tok-3","MaxResults":1}"#,
                ),
                json_response(200, r#"{"TableList":[{"Name":"tbl-c"}]}"#),
            ),
        ]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (tables, token) = client.get_tables("db-1", Some(3), None).await.unwrap();

        assert_eq!(tables.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_tables_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"DatabaseName":"db-1"}"#),
            json_error_response("InvalidInputException", "invalid input"),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let err = client.get_tables("db-1", None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("InvalidInputException"));
                assert_eq!(message, "invalid input");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_crawlers_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(
                200,
                r#"{"Crawlers":[{"Name":"crawler-a"},{"Name":"crawler-b"}]}"#,
            ),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (crawlers, token) = client.get_crawlers(None, None).await.unwrap();

        assert_eq!(crawlers.len(), 2);
        assert_eq!(crawlers[0].name(), Some("crawler-a"));
        assert_eq!(crawlers[1].name(), Some("crawler-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_crawlers_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"tok-1"}"#),
            json_response(200, r#"{"Crawlers":[{"Name":"crawler-b"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (crawlers, token) = client
            .get_crawlers(None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(crawlers.len(), 1);
        assert_eq!(crawlers[0].name(), Some("crawler-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_crawlers_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Crawlers":[{"Name":"crawler-a"},{"Name":"crawler-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (crawlers, token) = client.get_crawlers(Some(2), None).await.unwrap();

        assert_eq!(crawlers.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_crawlers_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"Crawlers":[{"Name":"crawler-a"},{"Name":"crawler-b"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"tok-3","MaxResults":1}"#),
                json_response(200, r#"{"Crawlers":[{"Name":"crawler-c"}]}"#),
            ),
        ]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (crawlers, token) = client.get_crawlers(Some(3), None).await.unwrap();

        assert_eq!(crawlers.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_crawlers_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("OperationTimeoutException", "operation timed out"),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let err = client.get_crawlers(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("OperationTimeoutException"));
                assert_eq!(message, "operation timed out");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    // `Job` (unlike `Database`/`Table`/`Crawler`/`JobRun` in this same file)
    // has no `#[derive(Debug)]` in `aws-sdk-glue` 1.152.0's `types/_job.rs`,
    // so `Result<Vec<Job>, VaporError>::unwrap_err()` won't compile here
    // (needs `Debug` on the `Ok` type too). Match the `Result` directly
    // instead, per the durable gotcha recorded for `comprehend.rs`.
    #[tokio::test]
    async fn get_jobs_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_response(200, r#"{"Jobs":[{"Name":"job-a"},{"Name":"job-b"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client.get_jobs(None, None).await.unwrap();

        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].name(), Some("job-a"));
        assert_eq!(jobs[1].name(), Some("job-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_jobs_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"tok-1"}"#),
            json_response(200, r#"{"Jobs":[{"Name":"job-b"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client
            .get_jobs(None, Some("tok-1".to_string()))
            .await
            .unwrap();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name(), Some("job-b"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_jobs_stops_at_limit_and_returns_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"Jobs":[{"Name":"job-a"},{"Name":"job-b"}],"NextToken":"tok-2"}"#,
            ),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client.get_jobs(Some(2), None).await.unwrap();

        assert_eq!(jobs.len(), 2);
        assert_eq!(token, Some("tok-2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_jobs_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":3}"#),
                json_response(
                    200,
                    r#"{"Jobs":[{"Name":"job-a"},{"Name":"job-b"}],"NextToken":"tok-3"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"tok-3","MaxResults":1}"#),
                json_response(200, r#"{"Jobs":[{"Name":"job-c"}]}"#),
            ),
        ]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let (jobs, token) = client.get_jobs(Some(3), None).await.unwrap();

        assert_eq!(jobs.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_jobs_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "{}"),
            json_error_response("InvalidInputException", "invalid input"),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let result = client.get_jobs(None, None).await;

        match result {
            Err(VaporError::AwsSdk { code, message }) => {
                assert_eq!(code.as_deref(), Some("InvalidInputException"));
                assert_eq!(message, "invalid input");
            }
            Ok(_) => panic!("expected AwsSdk error, got Ok"),
            Err(other) => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_job_runs_returns_job_runs() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"JobName":"job-1","MaxResults":10}"#),
            json_response(200, r#"{"JobRuns":[{"Id":"run-1"},{"Id":"run-2"}]}"#),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let job_runs = client.get_job_runs("job-1", 10).await.unwrap();

        assert_eq!(job_runs.len(), 2);
        assert_eq!(job_runs[0].id(), Some("run-1"));
        assert_eq!(job_runs[1].id(), Some("run-2"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_job_runs_returns_empty_when_no_runs() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"JobName":"job-1","MaxResults":10}"#),
            json_response(200, "{}"),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let job_runs = client.get_job_runs("job-1", 10).await.unwrap();

        assert_eq!(job_runs.len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_job_runs_error_propagates() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"JobName":"job-1","MaxResults":10}"#),
            json_error_response("EntityNotFoundException", "job not found"),
        )]);
        let client = GlueClient::new(&sdk_config(http_client.clone()));

        let err = client.get_job_runs("job-1", 10).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("EntityNotFoundException"));
                assert_eq!(message, "job not found");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
