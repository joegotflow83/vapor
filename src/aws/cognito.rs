use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct CognitoClient {
    inner: aws_sdk_cognitoidentityprovider::Client,
}

impl CognitoClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cognitoidentityprovider::Client::new(config),
        }
    }

    pub async fn list_user_pools(
        &self,
    ) -> Result<Vec<aws_sdk_cognitoidentityprovider::types::UserPoolDescriptionType>, VaporError>
    {
        let mut pools = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_user_pools().max_results(60);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            pools.extend(output.user_pools().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(pools)
    }

    pub async fn describe_user_pool(
        &self,
        user_pool_id: &str,
    ) -> Result<aws_sdk_cognitoidentityprovider::types::UserPoolType, VaporError> {
        let output = self
            .inner
            .describe_user_pool()
            .user_pool_id(user_pool_id)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        output
            .user_pool
            .ok_or_else(|| VaporError::AwsSdk("No user pool returned".to_string()))
    }

    pub async fn list_user_pool_clients(
        &self,
        user_pool_id: &str,
    ) -> Result<Vec<aws_sdk_cognitoidentityprovider::types::UserPoolClientDescription>, VaporError>
    {
        let mut clients = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_user_pool_clients()
                .user_pool_id(user_pool_id)
                .max_results(60);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            clients.extend(output.user_pool_clients().to_vec());
            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(clients)
    }

    pub async fn describe_user_pool_client(
        &self,
        user_pool_id: &str,
        client_id: &str,
    ) -> Result<aws_sdk_cognitoidentityprovider::types::UserPoolClientType, VaporError> {
        let output = self
            .inner
            .describe_user_pool_client()
            .user_pool_id(user_pool_id)
            .client_id(client_id)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        output
            .user_pool_client
            .ok_or_else(|| VaporError::AwsSdk("No user pool client returned".to_string()))
    }
}
