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

    pub async fn get_databases(&self) -> Result<Vec<Database>, VaporError> {
        let mut databases = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_databases();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            databases.extend(output.database_list().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(databases)
    }

    pub async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>, VaporError> {
        let mut tables = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_tables().database_name(database_name);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            tables.extend(output.table_list().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(tables)
    }

    pub async fn get_crawlers(&self) -> Result<Vec<Crawler>, VaporError> {
        let mut crawlers = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_crawlers();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            crawlers.extend(output.crawlers().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(crawlers)
    }

    pub async fn get_jobs(&self) -> Result<Vec<Job>, VaporError> {
        let mut jobs = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_jobs();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            jobs.extend(output.jobs().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }
        Ok(jobs)
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
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.job_runs().to_vec())
    }
}
