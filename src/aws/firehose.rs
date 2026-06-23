use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct FirehoseClient {
    inner: aws_sdk_firehose::Client,
}

impl FirehoseClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_firehose::Client::new(config),
        }
    }

    pub async fn list_delivery_streams(&self) -> Result<Vec<String>, VaporError> {
        let mut items = Vec::new();
        let mut exclusive_start: Option<String> = None;

        loop {
            let mut req = self.inner.list_delivery_streams();
            if let Some(ref name) = exclusive_start {
                req = req.exclusive_start_delivery_stream_name(name);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            let names = output.delivery_stream_names();
            if names.is_empty() {
                break;
            }
            let last = names.last().map(|s| s.to_string());
            items.extend(names.iter().map(|s| s.to_string()));

            if output.has_more_delivery_streams() {
                exclusive_start = last;
            } else {
                break;
            }
        }

        Ok(items)
    }

    pub async fn describe_delivery_stream(
        &self,
        name: &str,
    ) -> Result<aws_sdk_firehose::types::DeliveryStreamDescription, VaporError> {
        let output = self
            .inner
            .describe_delivery_stream()
            .delivery_stream_name(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        output
            .delivery_stream_description()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk("No delivery stream description".to_string()))
    }

    pub async fn list_tags_for_delivery_stream(
        &self,
        name: &str,
    ) -> Result<Vec<aws_sdk_firehose::types::Tag>, VaporError> {
        let mut tags = Vec::new();
        let mut exclusive_start_key: Option<String> = None;

        loop {
            let mut req = self.inner.list_tags_for_delivery_stream().delivery_stream_name(name);
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
