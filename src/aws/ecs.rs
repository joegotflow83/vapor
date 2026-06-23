use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct EcsClient {
    inner: aws_sdk_ecs::Client,
}

impl EcsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_ecs::Client::new(config),
        }
    }

    pub async fn describe_clusters(
        &self,
        cluster_arns: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_ecs::types::Cluster>, VaporError> {
        let arns = match cluster_arns {
            Some(arns) if !arns.is_empty() => arns,
            _ => {
                let mut all_arns: Vec<String> = Vec::new();
                let mut next_token: Option<String> = None;
                loop {
                    let mut req = self.inner.list_clusters();
                    if let Some(ref token) = next_token {
                        req = req.next_token(token);
                    }
                    let output = req
                        .send()
                        .await
                        .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                    all_arns.extend(output.cluster_arns().iter().map(|s| s.to_string()));
                    match output.next_token() {
                        Some(t) => next_token = Some(t.to_string()),
                        None => break,
                    }
                }
                all_arns
            }
        };

        if arns.is_empty() {
            return Ok(Vec::new());
        }

        let mut results: Vec<aws_sdk_ecs::types::Cluster> = Vec::new();
        for chunk in arns.chunks(100) {
            let output = self
                .inner
                .describe_clusters()
                .set_clusters(Some(chunk.to_vec()))
                .include(aws_sdk_ecs::types::ClusterField::Tags)
                .include(aws_sdk_ecs::types::ClusterField::Statistics)
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.clusters().iter().cloned());
        }
        Ok(results)
    }

    pub async fn describe_services(
        &self,
        cluster_arn: &str,
        service_arns: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_ecs::types::Service>, VaporError> {
        let arns = match service_arns {
            Some(arns) => arns,
            None => {
                let mut all_arns: Vec<String> = Vec::new();
                let mut next_token: Option<String> = None;
                loop {
                    let mut req = self.inner.list_services().cluster(cluster_arn);
                    if let Some(ref token) = next_token {
                        req = req.next_token(token);
                    }
                    let output = req
                        .send()
                        .await
                        .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                    all_arns.extend(output.service_arns().iter().map(|s| s.to_string()));
                    match output.next_token() {
                        Some(t) => next_token = Some(t.to_string()),
                        None => break,
                    }
                }
                all_arns
            }
        };

        if arns.is_empty() {
            return Ok(Vec::new());
        }

        let mut results: Vec<aws_sdk_ecs::types::Service> = Vec::new();
        for chunk in arns.chunks(10) {
            let output = self
                .inner
                .describe_services()
                .cluster(cluster_arn)
                .set_services(Some(chunk.to_vec()))
                .include(aws_sdk_ecs::types::ServiceField::Tags)
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.services().iter().cloned());
        }
        Ok(results)
    }

    pub async fn describe_tasks(
        &self,
        cluster_arn: &str,
        service_arn: Option<String>,
        desired_status: Option<String>,
    ) -> Result<Vec<aws_sdk_ecs::types::Task>, VaporError> {
        let mut all_task_arns: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_tasks().cluster(cluster_arn);
            if let Some(ref svc) = service_arn {
                req = req.service_name(svc);
            }
            if let Some(ref status) = desired_status {
                req = req.desired_status(aws_sdk_ecs::types::DesiredStatus::from(status.as_str()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_task_arns.extend(output.task_arns().iter().map(|s| s.to_string()));
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        if all_task_arns.is_empty() {
            return Ok(Vec::new());
        }

        let mut results: Vec<aws_sdk_ecs::types::Task> = Vec::new();
        for chunk in all_task_arns.chunks(100) {
            let output = self
                .inner
                .describe_tasks()
                .cluster(cluster_arn)
                .set_tasks(Some(chunk.to_vec()))
                .include(aws_sdk_ecs::types::TaskField::Tags)
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.tasks().iter().cloned());
        }
        Ok(results)
    }

    pub async fn describe_task_definition(
        &self,
        task_definition: &str,
    ) -> Result<Option<aws_sdk_ecs::types::TaskDefinition>, VaporError> {
        let result = self
            .inner
            .describe_task_definition()
            .task_definition(task_definition)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.task_definition().cloned()),
            Err(e) => {
                let is_not_found = e
                    .as_service_error()
                    .map(|se| se.is_client_exception() || se.is_invalid_parameter_exception())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(e.to_string()))
                }
            }
        }
    }

    pub async fn list_task_definitions(
        &self,
        family_prefix: Option<String>,
        status: Option<String>,
    ) -> Result<Vec<String>, VaporError> {
        let mut all_arns: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.list_task_definitions();
            if let Some(ref prefix) = family_prefix {
                req = req.family_prefix(prefix);
            }
            if let Some(ref s) = status {
                req = req
                    .status(aws_sdk_ecs::types::TaskDefinitionStatus::from(s.as_str()));
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            all_arns.extend(output.task_definition_arns().iter().map(|s| s.to_string()));
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(all_arns)
    }
}
