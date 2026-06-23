use aws_config::SdkConfig;
use aws_sdk_cloudtrail::primitives::DateTime;
use aws_sdk_cloudtrail::types::LookupAttribute;

use crate::error::VaporError;

pub struct CloudTrailClient {
    inner: aws_sdk_cloudtrail::Client,
}

impl CloudTrailClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_cloudtrail::Client::new(config),
        }
    }

    pub async fn describe_trails(
        &self,
    ) -> Result<Vec<aws_sdk_cloudtrail::types::Trail>, VaporError> {
        let output = self
            .inner
            .describe_trails()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
        Ok(output.trail_list().to_vec())
    }

    pub async fn get_trail_status(
        &self,
        name: &str,
    ) -> Result<aws_sdk_cloudtrail::operation::get_trail_status::GetTrailStatusOutput, VaporError>
    {
        self.inner
            .get_trail_status()
            .name(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn lookup_events(
        &self,
        start_time: DateTime,
        end_time: DateTime,
        attributes: Vec<LookupAttribute>,
    ) -> Result<Vec<aws_sdk_cloudtrail::types::Event>, VaporError> {
        let mut events = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .lookup_events()
                .start_time(start_time.clone())
                .end_time(end_time.clone());

            for attr in &attributes {
                req = req.lookup_attributes(attr.clone());
            }

            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            events.extend(output.events().to_vec());

            match output.next_token() {
                Some(token) => next_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(events)
    }
}
