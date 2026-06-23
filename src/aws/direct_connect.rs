use aws_config::SdkConfig;
use aws_sdk_directconnect::types::{Connection, VirtualInterface};

use crate::error::VaporError;

pub struct DirectConnectClient {
    inner: aws_sdk_directconnect::Client,
}

impl DirectConnectClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_directconnect::Client::new(config),
        }
    }

    pub async fn describe_connections(&self) -> Result<Vec<Connection>, VaporError> {
        let output = self
            .inner
            .describe_connections()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.connections().to_vec())
    }

    pub async fn describe_virtual_interfaces(
        &self,
        connection_id: Option<&str>,
    ) -> Result<Vec<VirtualInterface>, VaporError> {
        let mut req = self.inner.describe_virtual_interfaces();
        if let Some(id) = connection_id {
            req = req.connection_id(id);
        }
        let output = req
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.virtual_interfaces().to_vec())
    }
}
