use aws_config::SdkConfig;
use aws_sdk_transfer::types::{ListedServer, ListedUser};

use crate::error::VaporError;

pub struct TransferClient {
    inner: aws_sdk_transfer::Client,
}

impl TransferClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_transfer::Client::new(config),
        }
    }

    pub async fn list_servers(&self) -> Result<Vec<ListedServer>, VaporError> {
        let mut servers = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_servers();
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            servers.extend(output.servers().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(servers)
    }

    pub async fn list_users(&self, server_id: &str) -> Result<Vec<ListedUser>, VaporError> {
        let mut users = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_users().server_id(server_id);
            if let Some(ref t) = next_token {
                req = req.next_token(t);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            users.extend(output.users().to_vec());

            match output.next_token() {
                Some(t) if !t.is_empty() => next_token = Some(t.to_string()),
                _ => break,
            }
        }

        Ok(users)
    }
}
