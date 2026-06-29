use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct QuickSightUserInfo {
    pub user_name: Option<String>,
    pub arn: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub identity_type: Option<String>,
    pub active: bool,
    pub principal_id: Option<String>,
}

pub struct QuickSightDashboardInfo {
    pub dashboard_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub created_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub published_version_number: Option<i64>,
    pub last_published_time: Option<String>,
}

pub struct QuickSightDataSetInfo {
    pub data_set_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub created_time: Option<String>,
    pub last_updated_time: Option<String>,
    pub import_mode: Option<String>,
}

pub struct QuickSightDataSourceInfo {
    pub data_source_id: Option<String>,
    pub arn: Option<String>,
    pub name: Option<String>,
    pub type_: Option<String>,
    pub status: Option<String>,
    pub created_time: Option<String>,
    pub last_updated_time: Option<String>,
}

pub struct QuickSightClient {
    inner: aws_sdk_quicksight::Client,
}

impl QuickSightClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_quicksight::Client::new(config),
        }
    }

    pub async fn list_users(
        &self,
        aws_account_id: String,
        namespace: Option<String>,
    ) -> Result<Vec<QuickSightUserInfo>, VaporError> {
        let ns = namespace.unwrap_or_else(|| "default".to_string());
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_users()
                .aws_account_id(&aws_account_id)
                .namespace(&ns);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for u in output.user_list() {
                items.push(QuickSightUserInfo {
                    user_name: u.user_name().map(|s| s.to_string()),
                    arn: u.arn().map(|s| s.to_string()),
                    email: u.email().map(|s| s.to_string()),
                    role: u.role().map(|r| r.as_str().to_string()),
                    identity_type: u.identity_type().map(|t| t.as_str().to_string()),
                    active: u.active(),
                    principal_id: u.principal_id().map(|s| s.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_dashboards(
        &self,
        aws_account_id: String,
    ) -> Result<Vec<QuickSightDashboardInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_dashboards()
                .aws_account_id(&aws_account_id);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for d in output.dashboard_summary_list() {
                items.push(QuickSightDashboardInfo {
                    dashboard_id: d.dashboard_id().map(|s| s.to_string()),
                    arn: d.arn().map(|s| s.to_string()),
                    name: d.name().map(|s| s.to_string()),
                    created_time: d.created_time().map(|t| t.to_string()),
                    last_updated_time: d.last_updated_time().map(|t| t.to_string()),
                    published_version_number: d.published_version_number(),
                    last_published_time: d.last_published_time().map(|t| t.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_data_sets(
        &self,
        aws_account_id: String,
    ) -> Result<Vec<QuickSightDataSetInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_data_sets()
                .aws_account_id(&aws_account_id);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for ds in output.data_set_summaries() {
                items.push(QuickSightDataSetInfo {
                    data_set_id: ds.data_set_id().map(|s| s.to_string()),
                    arn: ds.arn().map(|s| s.to_string()),
                    name: ds.name().map(|s| s.to_string()),
                    created_time: ds.created_time().map(|t| t.to_string()),
                    last_updated_time: ds.last_updated_time().map(|t| t.to_string()),
                    import_mode: ds.import_mode().map(|m| m.as_str().to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn list_data_sources(
        &self,
        aws_account_id: String,
    ) -> Result<Vec<QuickSightDataSourceInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_data_sources()
                .aws_account_id(&aws_account_id);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

            for src in output.data_sources() {
                items.push(QuickSightDataSourceInfo {
                    data_source_id: src.data_source_id().map(|s| s.to_string()),
                    arn: src.arn().map(|s| s.to_string()),
                    name: src.name().map(|s| s.to_string()),
                    type_: src.r#type().map(|t| t.as_str().to_string()),
                    status: src.status().map(|s| s.as_str().to_string()),
                    created_time: src.created_time().map(|t| t.to_string()),
                    last_updated_time: src.last_updated_time().map(|t| t.to_string()),
                });
            }

            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
