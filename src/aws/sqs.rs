#[cfg(feature = "sqs")]
use aws_config::SdkConfig;
#[cfg(feature = "sqs")]
use aws_sdk_sqs::types::QueueAttributeName;
#[cfg(feature = "sqs")]
use std::collections::HashMap;

#[cfg(feature = "sqs")]
use crate::error::VaporError;

#[cfg(feature = "sqs")]
pub struct SqsClient {
    inner: aws_sdk_sqs::Client,
}

impl SqsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sqs::Client::new(config),
        }
    }

    pub async fn list_queues(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, VaporError> {
        let mut all_urls: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_queues();
            if let Some(p) = prefix {
                req = req.queue_name_prefix(p);
            }
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_urls.extend(output.queue_urls().iter().map(|s| s.to_string()));

            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        Ok(all_urls)
    }

    pub async fn get_queue_attributes(
        &self,
        queue_url: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(QueueAttributeName::All)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output
            .attributes()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.clone()))
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn list_queue_tags(
        &self,
        queue_url: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .list_queue_tags()
            .queue_url(queue_url)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.tags().cloned().unwrap_or_default())
    }
}
