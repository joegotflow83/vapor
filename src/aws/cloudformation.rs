use aws_config::SdkConfig;
use aws_sdk_cloudformation::types::{Export, Stack, StackResourceSummary, StackStatus, StackSummary};

use crate::error::VaporError;

pub struct CloudFormationClient {
    inner: aws_sdk_cloudformation::Client,
}

impl CloudFormationClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudformation::Client::new(config),
        }
    }

    /// Returns full stack details. If `stack_name` is Some, fetches exactly that stack
    /// (returns empty Vec if not found). Without a name, returns all non-DELETE_COMPLETE stacks.
    pub async fn describe_stacks(
        &self,
        stack_name: Option<&str>,
    ) -> Result<Vec<Stack>, VaporError> {
        let mut stacks: Vec<Stack> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.describe_stacks();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            if let Some(name) = stack_name {
                req = req.stack_name(name);
            }
            let output = match req.send().await {
                Ok(o) => o,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("does not exist") {
                        return Ok(vec![]);
                    }
                    return Err(VaporError::AwsSdk(msg));
                }
            };
            for s in output.stacks() {
                stacks.push(s.clone());
            }
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(stacks)
    }

    /// Returns stack summaries. If `status_filter` is empty, defaults to all active statuses
    /// (everything except DELETE_COMPLETE).
    pub async fn list_stacks(
        &self,
        status_filter: Vec<String>,
    ) -> Result<Vec<StackSummary>, VaporError> {
        let filters: Vec<StackStatus> = if status_filter.is_empty() {
            vec![
                StackStatus::CreateInProgress,
                StackStatus::CreateFailed,
                StackStatus::CreateComplete,
                StackStatus::RollbackInProgress,
                StackStatus::RollbackFailed,
                StackStatus::RollbackComplete,
                StackStatus::UpdateInProgress,
                StackStatus::UpdateCompleteCleanupInProgress,
                StackStatus::UpdateComplete,
                StackStatus::UpdateFailed,
                StackStatus::UpdateRollbackInProgress,
                StackStatus::UpdateRollbackFailed,
                StackStatus::UpdateRollbackCompleteCleanupInProgress,
                StackStatus::UpdateRollbackComplete,
                StackStatus::ReviewInProgress,
                StackStatus::ImportInProgress,
                StackStatus::ImportComplete,
                StackStatus::ImportRollbackInProgress,
                StackStatus::ImportRollbackFailed,
                StackStatus::ImportRollbackComplete,
            ]
        } else {
            status_filter
                .iter()
                .map(|s| StackStatus::from(s.as_str()))
                .collect()
        };

        let mut summaries: Vec<StackSummary> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_stacks();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            for f in &filters {
                req = req.stack_status_filter(f.clone());
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for s in output.stack_summaries() {
                summaries.push(s.clone());
            }
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(summaries)
    }

    /// Returns all resources for a given stack.
    pub async fn list_stack_resources(
        &self,
        stack_name: &str,
    ) -> Result<Vec<StackResourceSummary>, VaporError> {
        let mut resources: Vec<StackResourceSummary> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_stack_resources().stack_name(stack_name);
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for r in output.stack_resource_summaries() {
                resources.push(r.clone());
            }
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(resources)
    }

    /// Returns all CloudFormation exports (cross-stack references) with pagination.
    pub async fn list_exports(&self) -> Result<Vec<Export>, VaporError> {
        let mut exports: Vec<Export> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_exports();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for e in output.exports() {
                exports.push(e.clone());
            }
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(exports)
    }
}
