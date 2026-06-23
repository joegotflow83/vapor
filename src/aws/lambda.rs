#[cfg(feature = "lambda")]
use aws_config::SdkConfig;
#[cfg(feature = "lambda")]
use aws_sdk_lambda::types::{
    AliasConfiguration, EventSourceMappingConfiguration, FunctionConfiguration, LayersListItem,
};
#[cfg(feature = "lambda")]
use std::collections::HashMap;

#[cfg(feature = "lambda")]
use crate::error::VaporError;

#[cfg(feature = "lambda")]
pub struct LambdaClient {
    inner: aws_sdk_lambda::Client,
}

impl LambdaClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_lambda::Client::new(config),
        }
    }

    pub async fn list_functions(&self) -> Result<Vec<FunctionConfiguration>, VaporError> {
        let mut functions = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_functions();
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for f in output.functions() {
                functions.push(f.clone());
            }
            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(functions)
    }

    pub async fn list_aliases(
        &self,
        function_name: &str,
    ) -> Result<Vec<AliasConfiguration>, VaporError> {
        let mut aliases = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_aliases().function_name(function_name);
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for a in output.aliases() {
                aliases.push(a.clone());
            }
            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(aliases)
    }

    pub async fn list_event_source_mappings(
        &self,
        function_name: Option<&str>,
    ) -> Result<Vec<EventSourceMappingConfiguration>, VaporError> {
        let mut mappings = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_event_source_mappings();
            if let Some(name) = function_name {
                req = req.function_name(name);
            }
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for m in output.event_source_mappings() {
                mappings.push(m.clone());
            }
            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(mappings)
    }

    pub async fn list_layers(&self) -> Result<Vec<LayersListItem>, VaporError> {
        let mut layers = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_layers();
            if let Some(ref marker) = next_marker {
                req = req.marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for l in output.layers() {
                layers.push(l.clone());
            }
            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(layers)
    }

    /// Returns tags for a Lambda function (keyed by ARN).
    pub async fn list_tags(
        &self,
        function_arn: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .list_tags()
            .resource(function_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.tags().cloned().unwrap_or_default())
    }

    /// Returns the resource-based policy JSON for a Lambda function.
    /// Returns None when no policy is attached (ResourceNotFoundException).
    pub async fn get_function_policy(
        &self,
        function_name: &str,
    ) -> Result<Option<String>, VaporError> {
        match self
            .inner
            .get_policy()
            .function_name(function_name)
            .send()
            .await
        {
            Ok(output) => Ok(output.policy().map(|s| s.to_string())),
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_resource_not_found_exception() {
                    Ok(None)
                } else {
                    Err(VaporError::AwsSdk(svc_err.to_string()))
                }
            }
        }
    }
}
