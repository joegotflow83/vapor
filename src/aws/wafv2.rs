use aws_config::SdkConfig;
use aws_sdk_wafv2::types::Scope;

use crate::error::VaporError;

pub struct WafV2Client {
    inner: aws_sdk_wafv2::Client,
}

impl WafV2Client {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_wafv2::Client::new(config),
        }
    }

    pub async fn list_web_acls(
        &self,
        scope: Scope,
    ) -> Result<Vec<aws_sdk_wafv2::types::WebAclSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_web_acls().scope(scope.clone()).limit(100);
            if let Some(ref marker) = next_marker {
                req = req.next_marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.web_acls().to_vec());
            match output.next_marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }
        Ok(items)
    }

    pub async fn get_web_acl(
        &self,
        name: &str,
        id: &str,
        scope: Scope,
    ) -> Result<aws_sdk_wafv2::operation::get_web_acl::GetWebAclOutput, VaporError> {
        self.inner
            .get_web_acl()
            .name(name)
            .id(id)
            .scope(scope)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_ip_sets(
        &self,
        scope: Scope,
    ) -> Result<Vec<aws_sdk_wafv2::types::IpSetSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_ip_sets().scope(scope.clone()).limit(100);
            if let Some(ref marker) = next_marker {
                req = req.next_marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.ip_sets().to_vec());
            match output.next_marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }
        Ok(items)
    }

    pub async fn get_ip_set(
        &self,
        name: &str,
        id: &str,
        scope: Scope,
    ) -> Result<aws_sdk_wafv2::operation::get_ip_set::GetIpSetOutput, VaporError> {
        self.inner
            .get_ip_set()
            .name(name)
            .id(id)
            .scope(scope)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))
    }

    pub async fn list_rule_groups(
        &self,
        scope: Scope,
    ) -> Result<Vec<aws_sdk_wafv2::types::RuleGroupSummary>, VaporError> {
        let mut items = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_rule_groups().scope(scope.clone()).limit(100);
            if let Some(ref marker) = next_marker {
                req = req.next_marker(marker);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.rule_groups().to_vec());
            match output.next_marker() {
                Some(m) => next_marker = Some(m.to_string()),
                None => break,
            }
        }
        Ok(items)
    }
}
