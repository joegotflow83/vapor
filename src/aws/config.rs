use aws_config::retry::RetryConfig;
use aws_config::timeout::TimeoutConfig;
use aws_config::BehaviorVersion;
use aws_config::Region;
use std::time::Duration;

pub async fn load_aws_config(region: Option<&str>) -> aws_config::SdkConfig {
    let retry_config = RetryConfig::standard().with_max_attempts(3);
    let timeout_config = TimeoutConfig::builder()
        .operation_attempt_timeout(Duration::from_secs(30))
        .build();

    let mut loader = aws_config::defaults(BehaviorVersion::latest())
        .retry_config(retry_config)
        .timeout_config(timeout_config);
    if let Some(r) = region {
        loader = loader.region(Region::new(r.to_string()));
    }
    loader.load().await
}
