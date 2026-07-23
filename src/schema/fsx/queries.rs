use async_graphql::{Context, Object, Result};

use crate::aws::fsx::FsxClient;
use crate::schema::fsx::types::{FsxBackup, FsxFileSystem, FsxStorageVirtualMachine};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct FsxQuery;

#[Object]
impl FsxQuery {
    /// Lists FSx file systems, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn fsx_file_systems(
        &self,
        ctx: &Context<'_>,
        file_system_ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<FsxFileSystem>> {
        let client = ctx.data::<FsxClient>()?;
        let (items, next_token) = client
            .describe_file_systems(file_system_ids, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(FsxFileSystem::from).collect(),
            next_token,
        })
    }

    /// Lists FSx backups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn fsx_backups(
        &self,
        ctx: &Context<'_>,
        backup_ids: Option<Vec<String>>,
        file_system_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<FsxBackup>> {
        let client = ctx.data::<FsxClient>()?;
        let (items, next_token) = client
            .describe_backups(backup_ids, file_system_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(FsxBackup::from).collect(),
            next_token,
        })
    }

    /// Lists FSx storage virtual machines, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    async fn fsx_storage_virtual_machines(
        &self,
        ctx: &Context<'_>,
        file_system_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<FsxStorageVirtualMachine>> {
        let client = ctx.data::<FsxClient>()?;
        let (items, next_token) = client
            .describe_storage_virtual_machines(file_system_id, limit, next_token)
            .await?;
        Ok(Page {
            items: items.into_iter().map(FsxStorageVirtualMachine::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::fsx::FsxClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::FsxQuery;

    const ENDPOINT: &str = "https://fsx.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn fsx_file_systems_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"FileSystems":[{"FileSystemId":"fs-1","FileSystemType":"LUSTRE","Lifecycle":"AVAILABLE","StorageCapacity":1200,"StorageType":"SSD","VpcId":"vpc-1","SubnetIds":["subnet-1"],"DNSName":"fs-1.fsx.us-east-1.amazonaws.com","KmsKeyId":"key-1","CreationTime":1700000000,"Tags":[{"Key":"env","Value":"prod"}]}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(FsxQuery)
            .data(FsxClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ fsxFileSystems(limit: 1) { items { fileSystemId fileSystemType lifecycle storageCapacityGiB storageType vpcId subnetIds dnsName kmsKeyId creationTime tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["fsxFileSystems"]["items"];
        assert_eq!(items[0]["fileSystemId"], "fs-1");
        assert_eq!(items[0]["fileSystemType"], "LUSTRE");
        assert_eq!(items[0]["lifecycle"], "AVAILABLE");
        assert_eq!(items[0]["storageCapacityGiB"], 1200);
        assert_eq!(items[0]["storageType"], "SSD");
        assert_eq!(items[0]["vpcId"], "vpc-1");
        assert_eq!(items[0]["subnetIds"][0], "subnet-1");
        assert_eq!(items[0]["dnsName"], "fs-1.fsx.us-east-1.amazonaws.com");
        assert_eq!(items[0]["kmsKeyId"], "key-1");
        assert!(items[0]["creationTime"].is_string());
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["fsxFileSystems"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn fsx_backups_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"Backups":[{"BackupId":"backup-1","Lifecycle":"AVAILABLE","Type":"USER_INITIATED","CreationTime":1700000000,"ResourceARN":"arn:aws:fsx:us-east-1:1:backup/backup-1","FileSystem":{"FileSystemId":"fs-1"},"Tags":[{"Key":"env","Value":"prod"}]}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(FsxQuery)
            .data(FsxClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ fsxBackups(limit: 1) { items { backupId lifecycle backupType creationTime fileSystemId resourceArn tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["fsxBackups"]["items"];
        assert_eq!(items[0]["backupId"], "backup-1");
        assert_eq!(items[0]["lifecycle"], "AVAILABLE");
        assert_eq!(items[0]["backupType"], "USER_INITIATED");
        assert!(items[0]["creationTime"].is_string());
        assert_eq!(items[0]["fileSystemId"], "fs-1");
        assert_eq!(
            items[0]["resourceArn"],
            "arn:aws:fsx:us-east-1:1:backup/backup-1"
        );
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["fsxBackups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn fsx_storage_virtual_machines_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"StorageVirtualMachines":[{"StorageVirtualMachineId":"svm-1","Name":"svm-one","FileSystemId":"fs-1","Lifecycle":"CREATED","Subtype":"DEFAULT","CreationTime":1700000000,"Tags":[{"Key":"env","Value":"prod"}]}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(FsxQuery)
            .data(FsxClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ fsxStorageVirtualMachines(limit: 1) { items { storageVirtualMachineId name fileSystemId lifecycle subtype creationTime tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["fsxStorageVirtualMachines"]["items"];
        assert_eq!(items[0]["storageVirtualMachineId"], "svm-1");
        assert_eq!(items[0]["name"], "svm-one");
        assert_eq!(items[0]["fileSystemId"], "fs-1");
        assert_eq!(items[0]["lifecycle"], "CREATED");
        assert_eq!(items[0]["subtype"], "DEFAULT");
        assert!(items[0]["creationTime"].is_string());
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["fsxStorageVirtualMachines"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
