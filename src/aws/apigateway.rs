#[cfg(feature = "apigateway")]
use aws_config::SdkConfig;

#[cfg(feature = "apigateway")]
use crate::error::VaporError;

#[cfg(feature = "apigateway")]
pub struct ApiGatewayClient {
    rest: aws_sdk_apigateway::Client,
}

#[cfg(feature = "apigateway")]
impl ApiGatewayClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self { rest: aws_sdk_apigateway::Client::new(config) }
    }

    // ── REST API (v1) ─────────────────────────────────────────────────────────

    pub async fn list_rest_apis(
        &self,
    ) -> Result<Vec<aws_sdk_apigateway::types::RestApi>, VaporError> {
        let mut results = Vec::new();
        let mut position: Option<String> = None;
        loop {
            let mut req = self.rest.get_rest_apis().limit(500);
            if let Some(ref pos) = position {
                req = req.position(pos);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.position() {
                Some(p) => position = Some(p.to_string()),
                None => break,
            }
        }
        Ok(results)
    }

    pub async fn list_rest_stages(
        &self,
        api_id: &str,
    ) -> Result<Vec<aws_sdk_apigateway::types::Stage>, VaporError> {
        let output = self
            .rest
            .get_stages()
            .rest_api_id(api_id)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.item().to_vec())
    }

    pub async fn list_rest_resources(
        &self,
        api_id: &str,
    ) -> Result<Vec<aws_sdk_apigateway::types::Resource>, VaporError> {
        let mut results = Vec::new();
        let mut position: Option<String> = None;
        loop {
            let mut req = self.rest.get_resources().rest_api_id(api_id);
            if let Some(ref pos) = position {
                req = req.position(pos);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.position() {
                Some(p) => position = Some(p.to_string()),
                None => break,
            }
        }
        Ok(results)
    }

    pub async fn list_rest_deployments(
        &self,
        api_id: &str,
    ) -> Result<Vec<aws_sdk_apigateway::types::Deployment>, VaporError> {
        let mut results = Vec::new();
        let mut position: Option<String> = None;
        loop {
            let mut req = self.rest.get_deployments().rest_api_id(api_id);
            if let Some(ref pos) = position {
                req = req.position(pos);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.position() {
                Some(p) => position = Some(p.to_string()),
                None => break,
            }
        }
        Ok(results)
    }
}
