use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct DetectiveClient {
    inner: aws_sdk_detective::Client,
}

pub struct DatasourcePackageInfo {
    pub datasource_package: Option<String>,
    pub ingest_state: Option<String>,
}

impl DetectiveClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_detective::Client::new(config),
        }
    }

    pub async fn list_graphs(
        &self,
    ) -> Result<Vec<aws_sdk_detective::types::Graph>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_graphs();
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.graph_list().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_members(
        &self,
        graph_arn: String,
    ) -> Result<Vec<aws_sdk_detective::types::MemberDetail>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_members().graph_arn(&graph_arn);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.member_details().to_vec());
            match output.next_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_datasource_packages(
        &self,
        graph_arn: String,
    ) -> Result<Vec<DatasourcePackageInfo>, VaporError> {
        let output = self
            .inner
            .list_datasource_packages()
            .graph_arn(&graph_arn)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        let packages = match output.datasource_packages() {
            Some(map) => map
                .iter()
                .map(|(pkg, details)| DatasourcePackageInfo {
                    datasource_package: Some(pkg.as_str().to_string()),
                    ingest_state: details
                        .datasource_package_ingest_state()
                        .map(|s| s.as_str().to_string()),
                })
                .collect(),
            None => Vec::new(),
        };

        Ok(packages)
    }
}
