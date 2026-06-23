use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct AthenaClient {
    inner: aws_sdk_athena::Client,
}

impl AthenaClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_athena::Client::new(config),
        }
    }

    pub async fn list_work_groups(
        &self,
    ) -> Result<Vec<aws_sdk_athena::types::WorkGroupSummary>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_work_groups();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.work_groups().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(results)
    }

    pub async fn get_work_group(
        &self,
        name: &str,
    ) -> Result<aws_sdk_athena::types::WorkGroup, VaporError> {
        let output = self
            .inner
            .get_work_group()
            .work_group(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        output
            .work_group()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk("WorkGroup not found".to_string()))
    }

    pub async fn list_named_queries(
        &self,
        workgroup: Option<&str>,
    ) -> Result<Vec<String>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_named_queries();
            if let Some(wg) = workgroup {
                req = req.work_group(wg);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.named_query_ids().iter().cloned());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(results)
    }

    pub async fn batch_get_named_query(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<aws_sdk_athena::types::NamedQuery>, VaporError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let output = self
            .inner
            .batch_get_named_query()
            .set_named_query_ids(Some(ids))
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.named_queries().to_vec())
    }

    pub async fn list_query_executions(
        &self,
        workgroup: Option<&str>,
        max_results: Option<i32>,
    ) -> Result<Vec<String>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;
        let limit = max_results.unwrap_or(50) as usize;

        loop {
            let mut req = self.inner.list_query_executions();
            if let Some(wg) = workgroup {
                req = req.work_group(wg);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.query_execution_ids().iter().cloned());
            if results.len() >= limit {
                results.truncate(limit);
                break;
            }
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(results)
    }

    pub async fn batch_get_query_execution(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<aws_sdk_athena::types::QueryExecution>, VaporError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let output = self
            .inner
            .batch_get_query_execution()
            .set_query_execution_ids(Some(ids))
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.query_executions().to_vec())
    }
}
