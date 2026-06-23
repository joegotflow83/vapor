use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct Elbv2Client {
    inner: aws_sdk_elasticloadbalancingv2::Client,
}

impl Elbv2Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_elasticloadbalancingv2::Client::new(config),
        }
    }

    pub async fn describe_load_balancers(
        &self,
        arns: Option<Vec<String>>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_elasticloadbalancingv2::types::LoadBalancer>, VaporError> {
        let mut all_items: Vec<aws_sdk_elasticloadbalancingv2::types::LoadBalancer> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = self.inner.describe_load_balancers();

            request = request.set_load_balancer_arns(arns.clone());
            request = request.set_names(names.clone());

            if let Some(ref marker) = next_marker {
                request = request.marker(marker);
            }

            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_items.extend(output.load_balancers().iter().cloned());

            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }

    pub async fn describe_target_groups(
        &self,
        arns: Option<Vec<String>>,
        load_balancer_arn: Option<String>,
    ) -> Result<Vec<aws_sdk_elasticloadbalancingv2::types::TargetGroup>, VaporError> {
        let mut all_items: Vec<aws_sdk_elasticloadbalancingv2::types::TargetGroup> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = self.inner.describe_target_groups();

            request = request.set_target_group_arns(arns.clone());

            if let Some(ref lb_arn) = load_balancer_arn {
                request = request.load_balancer_arn(lb_arn);
            }

            if let Some(ref marker) = next_marker {
                request = request.marker(marker);
            }

            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_items.extend(output.target_groups().iter().cloned());

            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }

    pub async fn describe_target_health(
        &self,
        target_group_arn: String,
    ) -> Result<Vec<aws_sdk_elasticloadbalancingv2::types::TargetHealthDescription>, VaporError>
    {
        let output = self
            .inner
            .describe_target_health()
            .target_group_arn(target_group_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.target_health_descriptions().to_vec())
    }

    pub async fn describe_listeners(
        &self,
        load_balancer_arn: String,
    ) -> Result<Vec<aws_sdk_elasticloadbalancingv2::types::Listener>, VaporError> {
        let mut all_items: Vec<aws_sdk_elasticloadbalancingv2::types::Listener> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = self
                .inner
                .describe_listeners()
                .load_balancer_arn(&load_balancer_arn);

            if let Some(ref marker) = next_marker {
                request = request.marker(marker);
            }

            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_items.extend(output.listeners().iter().cloned());

            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }

    pub async fn describe_rules(
        &self,
        listener_arn: String,
    ) -> Result<Vec<aws_sdk_elasticloadbalancingv2::types::Rule>, VaporError> {
        let mut all_items: Vec<aws_sdk_elasticloadbalancingv2::types::Rule> = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = self
                .inner
                .describe_rules()
                .listener_arn(&listener_arn);

            if let Some(ref marker) = next_marker {
                request = request.marker(marker);
            }

            let output = request
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            all_items.extend(output.rules().iter().cloned());

            match output.next_marker() {
                Some(marker) => next_marker = Some(marker.to_string()),
                None => break,
            }
        }

        Ok(all_items)
    }
}
