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
            .map_err(crate::error::sdk_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://sts.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn get_caller_identity_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=GetCallerIdentity&Version=2011-06-15"),
            xml_response(
                200,
                "<GetCallerIdentityResponse><GetCallerIdentityResult>\
                 <UserId>AIDACKCEVSQ6C2EXAMPLE</UserId>\
                 <Account>123456789012</Account>\
                 <Arn>arn:aws:iam::123456789012:user/Alice</Arn>\
                 </GetCallerIdentityResult></GetCallerIdentityResponse>",
            ),
        )]);
        let client = StsClient::new(&sdk_config(http_client.clone()));

        let output = client.get_caller_identity().await.unwrap();

        assert_eq!(output.user_id(), Some("AIDACKCEVSQ6C2EXAMPLE"));
        assert_eq!(output.account(), Some("123456789012"));
        assert_eq!(output.arn(), Some("arn:aws:iam::123456789012:user/Alice"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_caller_identity_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=GetCallerIdentity&Version=2011-06-15"),
            xml_error_response("AccessDenied", "not authorized to call GetCallerIdentity"),
        )]);
        let client = StsClient::new(&sdk_config(http_client.clone()));

        let err = client.get_caller_identity().await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code.as_deref(), Some("AccessDenied"));
                assert_eq!(message, "not authorized to call GetCallerIdentity");
            }
            other => panic!("expected AwsSdk error, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
