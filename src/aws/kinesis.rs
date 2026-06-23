use aws_config::SdkConfig;
use aws_sdk_kinesis::types::Shard;

use crate::error::VaporError;

pub struct KinesisClient {
    inner: aws_sdk_kinesis::Client,
}

impl KinesisClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_kinesis::Client::new(config),
        }
    }

    pub async fn list_streams(&self) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_streams();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.stream_names().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(token) if output.has_more_streams() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn describe_stream_summary(
        &self,
        name: &str,
    ) -> Result<aws_sdk_kinesis::types::StreamDescriptionSummary, VaporError> {
        let output = self
            .inner
            .describe_stream_summary()
            .stream_name(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        output
            .stream_description_summary()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk("No stream description summary".to_string()))
    }

    pub async fn list_shards(&self, name: &str) -> Result<Vec<Shard>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_shards();
            if next_token.is_some() {
                req = req.set_next_token(next_token.clone());
            } else {
                req = req.stream_name(name);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.shards().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(items)
    }

    pub async fn list_tags_for_stream(&self, name: &str) -> Result<Vec<aws_sdk_kinesis::types::Tag>, VaporError> {
        let mut tags = Vec::new();
        let mut exclusive_start_key: Option<String> = None;

        loop {
            let mut req = self.inner.list_tags_for_stream().stream_name(name);
            if let Some(ref key) = exclusive_start_key {
                req = req.exclusive_start_tag_key(key);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            let page_tags = output.tags();
            if page_tags.is_empty() {
                break;
            }
            let last_key = page_tags.last().map(|t| t.key().to_string());
            tags.extend(page_tags.to_vec());

            if output.has_more_tags() {
                exclusive_start_key = last_key;
            } else {
                break;
            }
        }

        Ok(tags)
    }
}
