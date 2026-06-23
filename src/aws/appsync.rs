use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct AppSyncClient {
    inner: aws_sdk_appsync::Client,
}

impl AppSyncClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_appsync::Client::new(config),
        }
    }

    pub async fn list_graphql_apis(
        &self,
    ) -> Result<Vec<aws_sdk_appsync::types::GraphqlApi>, VaporError> {
        let mut apis = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_graphql_apis();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            apis.extend(output.graphql_apis().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(apis)
    }

    pub async fn list_data_sources(
        &self,
        api_id: &str,
    ) -> Result<Vec<aws_sdk_appsync::types::DataSource>, VaporError> {
        let mut sources = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_data_sources().api_id(api_id);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            sources.extend(output.data_sources().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(sources)
    }
}
