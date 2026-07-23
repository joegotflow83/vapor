use async_graphql::{Context, Object, Result};

use crate::aws::backup::BackupClient;
use crate::schema::backup::types::{BackupPlan, BackupVault, RecoveryPoint};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct BackupQuery;

#[Object]
impl BackupQuery {
    /// Lists backup vaults, optionally capped at `limit` results (default
    /// unlimited) and resumed from `nextToken`.
    async fn backup_vaults(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BackupVault>> {
        let client = ctx.data::<BackupClient>()?;
        let (vaults, next_token) = client.list_backup_vaults(limit, next_token).await?;
        Ok(Page {
            items: vaults.into_iter().map(BackupVault::from).collect(),
            next_token,
        })
    }

    /// Lists backup plans, optionally capped at `limit` results (default
    /// unlimited) and resumed from `nextToken`.
    async fn backup_plans(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<BackupPlan>> {
        let client = ctx.data::<BackupClient>()?;
        let (plans, next_token) = client.list_backup_plans(limit, next_token).await?;
        Ok(Page {
            items: plans.into_iter().map(BackupPlan::from).collect(),
            next_token,
        })
    }

    /// Lists recovery points in a backup vault, optionally capped at `limit`
    /// results (default unlimited) and resumed from `nextToken`.
    async fn backup_recovery_points(
        &self,
        ctx: &Context<'_>,
        vault_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RecoveryPoint>> {
        let client = ctx.data::<BackupClient>()?;
        let (points, next_token) = client
            .list_recovery_points_by_backup_vault(&vault_name, limit, next_token)
            .await?;
        Ok(Page {
            items: points.into_iter().map(RecoveryPoint::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `BackupClient` method each (see `src/aws/backup.rs`'s own test module for
// the pagination/limit/error-mapping behavior) — only light smoke tests are
// needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::backup::BackupClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::BackupQuery;

    const VAULTS: &str = "https://backup.us-east-1.amazonaws.com/backup-vaults";
    const PLANS: &str = "https://backup.us-east-1.amazonaws.com/backup/plans";

    #[tokio::test]
    async fn backup_vaults_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{VAULTS}?maxResults=1"), ""),
            json_response(
                200,
                r#"{"BackupVaultList":[{"BackupVaultName":"v1","BackupVaultArn":"arn:v1","NumberOfRecoveryPoints":3,"Locked":true}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(BackupQuery)
            .data(BackupClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ backupVaults(limit: 1) { items { name arn recoveryPoints locked } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["backupVaults"]["items"];
        assert_eq!(items[0]["name"], "v1");
        assert_eq!(items[0]["arn"], "arn:v1");
        assert_eq!(items[0]["recoveryPoints"], 3);
        assert_eq!(items[0]["locked"], true);
        assert_eq!(json["backupVaults"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn backup_plans_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(PLANS, ""),
            json_response(
                200,
                r#"{"BackupPlansList":[{"BackupPlanId":"p1","BackupPlanArn":"arn:p1","BackupPlanName":"DailyBackup","VersionId":"v1"}]}"#,
            ),
        )]);
        let schema = build_query_schema(BackupQuery)
            .data(BackupClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ backupPlans { items { planId planName arn versionId } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["backupPlans"]["items"];
        assert_eq!(items[0]["planId"], "p1");
        assert_eq!(items[0]["planName"], "DailyBackup");
        assert_eq!(items[0]["arn"], "arn:p1");
        assert_eq!(items[0]["versionId"], "v1");
        assert!(json["backupPlans"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn backup_recovery_points_maps_items_for_given_vault() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{VAULTS}/my-vault/recovery-points"),
                "",
            ),
            json_response(
                200,
                r#"{"RecoveryPoints":[{"RecoveryPointArn":"arn:rp1","ResourceArn":"arn:vol1","ResourceType":"EBS","Status":"COMPLETED","IsEncrypted":true,"BackupSizeInBytes":1024}]}"#,
            ),
        )]);
        let schema = build_query_schema(BackupQuery)
            .data(BackupClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ backupRecoveryPoints(vaultName: "my-vault") { items { recoveryPointArn resourceArn resourceType status encrypted backupSizeBytes } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["backupRecoveryPoints"]["items"];
        assert_eq!(items[0]["recoveryPointArn"], "arn:rp1");
        assert_eq!(items[0]["resourceArn"], "arn:vol1");
        assert_eq!(items[0]["resourceType"], "EBS");
        assert_eq!(items[0]["status"], "COMPLETED");
        assert_eq!(items[0]["encrypted"], true);
        assert_eq!(items[0]["backupSizeBytes"], 1024);
        assert!(json["backupRecoveryPoints"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
