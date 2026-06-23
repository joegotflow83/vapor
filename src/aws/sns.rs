use aws_config::SdkConfig;
use std::collections::HashMap;

use crate::error::VaporError;

pub struct SnsClient {
    inner: aws_sdk_sns::Client,
}

impl SnsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sns::Client::new(config),
        }
    }

    /// List all topic ARNs, then fetch attributes for each.
    pub async fn list_topics_with_attributes(
        &self,
    ) -> Result<Vec<(String, HashMap<String, String>)>, VaporError> {
        let mut arns: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_topics();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for topic in output.topics() {
                if let Some(arn) = topic.topic_arn() {
                    arns.push(arn.to_string());
                }
            }

            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None => break,
            }
        }

        let mut result = Vec::with_capacity(arns.len());
        for arn in arns {
            let attrs = self.get_topic_attributes(&arn).await?;
            result.push((arn, attrs));
        }

        Ok(result)
    }

    /// Fetch attributes for a single topic ARN.
    pub async fn get_topic_attributes(
        &self,
        topic_arn: &str,
    ) -> Result<HashMap<String, String>, VaporError> {
        let output = self
            .inner
            .get_topic_attributes()
            .topic_arn(topic_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.attributes().cloned().unwrap_or_default())
    }

    /// List subscriptions, optionally filtered by topic ARN.
    pub async fn list_subscriptions(
        &self,
        topic_arn: Option<&str>,
    ) -> Result<Vec<aws_sdk_sns::types::Subscription>, VaporError> {
        let mut all: Vec<aws_sdk_sns::types::Subscription> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let output = if let Some(arn) = topic_arn {
                let mut req = self.inner.list_subscriptions_by_topic().topic_arn(arn);
                if let Some(ref token) = next_token {
                    req = req.next_token(token);
                }
                let o = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                let tok = o.next_token().map(|s| s.to_string());
                all.extend(o.subscriptions().iter().cloned());
                tok
            } else {
                let mut req = self.inner.list_subscriptions();
                if let Some(ref token) = next_token {
                    req = req.next_token(token);
                }
                let o = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
                let tok = o.next_token().map(|s| s.to_string());
                all.extend(o.subscriptions().iter().cloned());
                tok
            };

            match output {
                Some(t) => next_token = Some(t),
                None => break,
            }
        }

        Ok(all)
    }
}
