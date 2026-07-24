use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct BackupClient {
    inner: aws_sdk_backup::Client,
}

impl BackupClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_backup::Client::new(config),
        }
    }

    /// Lists backup vaults, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListBackupVaults` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-backup` 1.114.0's
    /// `operation/list_backup_vaults/_list_backup_vaults_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself,
    /// matching `global_accelerator.rs`'s `list_accelerators` pattern.
    pub async fn list_backup_vaults(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_backup::types::BackupVaultListMember>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut vaults = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_backup_vaults();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - vaults.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            vaults.extend(output.backup_vault_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if vaults.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((vaults, token))
    }

    /// Lists backup plans, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListBackupPlans` has both
    /// `max_results` and `next_token` (verified against pinned
    /// `aws-sdk-backup` 1.114.0's
    /// `operation/list_backup_plans/_list_backup_plans_input.rs`), same
    /// pattern.
    pub async fn list_backup_plans(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_backup::types::BackupPlansListMember>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut plans = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_backup_plans();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - plans.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            plans.extend(output.backup_plans_list.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if plans.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((plans, token))
    }

    /// Lists recovery points in a backup vault, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `ListRecoveryPointsByBackupVault` has both `max_results` and
    /// `next_token` (verified against pinned `aws-sdk-backup` 1.114.0's
    /// `operation/list_recovery_points_by_backup_vault/
    /// _list_recovery_points_by_backup_vault_input.rs`), same pattern.
    pub async fn list_recovery_points_by_backup_vault(
        &self,
        vault_name: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<
        (
            Vec<aws_sdk_backup::types::RecoveryPointByBackupVault>,
            Option<String>,
        ),
        VaporError,
    > {
        let mut points = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .list_recovery_points_by_backup_vault()
                .backup_vault_name(vault_name);
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - points.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            points.extend(output.recovery_points.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if points.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((points, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const VAULTS: &str = "https://backup.us-east-1.amazonaws.com/backup-vaults";
    const PLANS: &str = "https://backup.us-east-1.amazonaws.com/backup/plans";

    #[tokio::test]
    async fn list_backup_vaults_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(VAULTS, ""),
            json_response(
                200,
                r#"{"BackupVaultList":[{"BackupVaultName":"v1","BackupVaultArn":"arn:v1"},{"BackupVaultName":"v2","BackupVaultArn":"arn:v2"}]}"#,
            ),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (vaults, token) = client.list_backup_vaults(None, None).await.unwrap();

        assert_eq!(vaults.len(), 2);
        assert_eq!(vaults[0].backup_vault_name(), Some("v1"));
        assert_eq!(vaults[1].backup_vault_arn(), Some("arn:v2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_backup_vaults_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{VAULTS}?nextToken=cursor-a"), ""),
            json_response(
                200,
                r#"{"BackupVaultList":[{"BackupVaultName":"v3","BackupVaultArn":"arn:v3"}]}"#,
            ),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (vaults, token) = client
            .list_backup_vaults(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(vaults.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_backup_vaults_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{VAULTS}?maxResults=2"), ""),
            json_response(
                200,
                r#"{"BackupVaultList":[{"BackupVaultName":"v1","BackupVaultArn":"arn:v1"},{"BackupVaultName":"v2","BackupVaultArn":"arn:v2"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (vaults, token) = client.list_backup_vaults(Some(2), None).await.unwrap();

        assert_eq!(vaults.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_backup_vaults_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{VAULTS}?maxResults=10"), ""),
                json_response(
                    200,
                    r#"{"BackupVaultList":[{"BackupVaultName":"v1","BackupVaultArn":"arn:v1"},{"BackupVaultName":"v2","BackupVaultArn":"arn:v2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{VAULTS}?nextToken=p2&maxResults=8"), ""),
                json_response(
                    200,
                    r#"{"BackupVaultList":[{"BackupVaultName":"v3","BackupVaultArn":"arn:v3"}]}"#,
                ),
            ),
        ]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (vaults, token) = client.list_backup_vaults(Some(10), None).await.unwrap();

        assert_eq!(vaults.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_backup_vaults_propagates_errors() {
        // `InvalidParameterValueException` (a modeled, non-throttling error for
        // this op) rather than a built-in throttling-exception name — see
        // apigateway.rs's precedent: throttling names get retried by the SDK's
        // default retry strategy, consuming a second replay event this
        // single-event client doesn't have.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(VAULTS, ""),
            json_error_response("InvalidParameterValueException", "bad vault type"),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let err = client.list_backup_vaults(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterValueException".to_string()));
                assert_eq!(message, "bad vault type");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_backup_plans_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(PLANS, ""),
            json_response(
                200,
                r#"{"BackupPlansList":[{"BackupPlanId":"p1","BackupPlanArn":"arn:p1","BackupPlanName":"one"}]}"#,
            ),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (plans, token) = client.list_backup_plans(None, None).await.unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].backup_plan_id(), Some("p1"));
        assert_eq!(plans[0].backup_plan_name(), Some("one"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_backup_plans_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{PLANS}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"BackupPlansList":[{"BackupPlanId":"p1","BackupPlanArn":"arn:p1"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (plans, token) = client.list_backup_plans(Some(1), None).await.unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_recovery_points_by_backup_vault_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://backup.us-east-1.amazonaws.com/backup-vaults/my-vault/recovery-points",
                "",
            ),
            json_response(
                200,
                r#"{"RecoveryPoints":[{"RecoveryPointArn":"arn:rp1","BackupVaultName":"my-vault"}]}"#,
            ),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (points, token) = client
            .list_recovery_points_by_backup_vault("my-vault", None, None)
            .await
            .unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].recovery_point_arn(), Some("arn:rp1"));
        assert_eq!(points[0].backup_vault_name(), Some("my-vault"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_recovery_points_by_backup_vault_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                "https://backup.us-east-1.amazonaws.com/backup-vaults/my-vault/recovery-points?maxResults=1",
                "",
            ),
            json_response(
                200,
                r#"{"RecoveryPoints":[{"RecoveryPointArn":"arn:rp1","BackupVaultName":"my-vault"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = BackupClient::new(&sdk_config(http_client.clone()));

        let (points, token) = client
            .list_recovery_points_by_backup_vault("my-vault", Some(1), None)
            .await
            .unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }
}
