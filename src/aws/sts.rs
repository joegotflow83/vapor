use aws_config::SdkConfig;
use aws_sdk_sts::operation::get_caller_identity::GetCallerIdentityOutput;

use crate::error::VaporError;

pub struct StsClient {
    inner: aws_sdk_sts::Client,
}

impl StsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_sts::Client::new(config),
        }
    }

    pub async fn get_caller_identity(&self) -> Result<GetCallerIdentityOutput, VaporError> {
        self.inner
            .get_caller_identity()
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }
}
