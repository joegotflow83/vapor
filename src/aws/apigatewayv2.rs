#[cfg(feature = "apigatewayv2")]
use aws_config::SdkConfig;

#[cfg(feature = "apigatewayv2")]
use crate::error::VaporError;

#[cfg(feature = "apigatewayv2")]
pub struct ApiGatewayV2Client {
    inner: aws_sdk_apigatewayv2::Client,
}

impl ApiGatewayV2Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_apigatewayv2::Client::new(config),
        }
    }

    pub async fn get_apis(&self) -> Result<Vec<aws_sdk_apigatewayv2::types::Api>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_apis();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(results)
    }

    pub async fn get_stages(
        &self,
        api_id: &str,
    ) -> Result<Vec<aws_sdk_apigatewayv2::types::Stage>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_stages().api_id(api_id);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(results)
    }

    pub async fn get_routes(
        &self,
        api_id: &str,
    ) -> Result<Vec<aws_sdk_apigatewayv2::types::Route>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_routes().api_id(api_id);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(results)
    }

    pub async fn get_domain_names(
        &self,
    ) -> Result<Vec<aws_sdk_apigatewayv2::types::DomainName>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_domain_names();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(results)
    }

    pub async fn get_vpc_links(
        &self,
    ) -> Result<Vec<aws_sdk_apigatewayv2::types::VpcLink>, VaporError> {
        let mut results = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut req = self.inner.get_vpc_links();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            results.extend(output.items().iter().cloned());
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }
        Ok(results)
    }
}
