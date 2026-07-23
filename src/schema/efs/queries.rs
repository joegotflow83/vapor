use async_graphql::{Context, Object, Result};

use crate::aws::efs::EfsClient;
use crate::schema::efs::types::{EfsAccessPoint, EfsFileSystem, EfsMountTarget};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct EfsQuery;

#[Object]
impl EfsQuery {
    /// Lists EFS file systems. `limit` caps the number of results (default
    /// unlimited); `nextToken` resumes from a previous page.
    async fn efs_file_systems(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EfsFileSystem>> {
        let client = ctx.data::<EfsClient>()?;
        let (file_systems, token) = client.describe_file_systems(limit, next_token).await?;
        Ok(Page {
            items: file_systems.iter().map(EfsFileSystem::from).collect(),
            next_token: token,
        })
    }

    /// Lists mount targets for a file system. `limit` caps the number of
    /// results (default unlimited); `nextToken` resumes from a previous page.
    async fn efs_mount_targets(
        &self,
        ctx: &Context<'_>,
        file_system_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EfsMountTarget>> {
        let client = ctx.data::<EfsClient>()?;
        let (targets, token) = client
            .describe_mount_targets(&file_system_id, limit, next_token)
            .await?;
        Ok(Page {
            items: targets.iter().map(EfsMountTarget::from).collect(),
            next_token: token,
        })
    }

    /// Lists EFS access points. `limit` caps the number of results (default
    /// unlimited); `nextToken` resumes from a previous page.
    async fn efs_access_points(
        &self,
        ctx: &Context<'_>,
        file_system_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EfsAccessPoint>> {
        let client = ctx.data::<EfsClient>()?;
        let (points, token) = client
            .describe_access_points(file_system_id.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: points.iter().map(EfsAccessPoint::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::efs::EfsClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::EfsQuery;

    const FS: &str = "https://elasticfilesystem.us-east-1.amazonaws.com/2015-02-01/file-systems";
    const MT: &str = "https://elasticfilesystem.us-east-1.amazonaws.com/2015-02-01/mount-targets";
    const AP: &str = "https://elasticfilesystem.us-east-1.amazonaws.com/2015-02-01/access-points";

    #[tokio::test]
    async fn efs_file_systems_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FS}?MaxItems=1"), ""),
            json_response(
                200,
                r#"{"FileSystems":[{"OwnerId":"acct-1","CreationToken":"tok-fs-1","FileSystemId":"fs-1","CreationTime":1700000000,"LifeCycleState":"available","NumberOfMountTargets":1,"PerformanceMode":"generalPurpose","Encrypted":true}],"NextMarker":"page2"}"#,
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EfsQuery).data(client).finish();

        let res = schema
            .execute(r#"{ efsFileSystems(limit: 1) { items { fileSystemId lifeCycleState performanceMode encrypted } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["efsFileSystems"]["items"][0]["fileSystemId"], "fs-1");
        assert_eq!(data["efsFileSystems"]["items"][0]["lifeCycleState"], "available");
        assert_eq!(data["efsFileSystems"]["items"][0]["performanceMode"], "generalPurpose");
        assert_eq!(data["efsFileSystems"]["items"][0]["encrypted"], true);
        assert_eq!(data["efsFileSystems"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn efs_mount_targets_forwards_file_system_id_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{MT}?FileSystemId=fs-1"), ""),
            json_response(
                200,
                r#"{"MountTargets":[{"MountTargetId":"mt-1","FileSystemId":"fs-1","SubnetId":"subnet-1","IpAddress":"10.0.0.1","LifeCycleState":"available","AvailabilityZoneName":"us-east-1a"}]}"#,
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EfsQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ efsMountTargets(fileSystemId: "fs-1") { items { mountTargetId fileSystemId subnetId ipAddress availabilityZone } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["efsMountTargets"]["items"][0]["mountTargetId"], "mt-1");
        assert_eq!(data["efsMountTargets"]["items"][0]["fileSystemId"], "fs-1");
        assert_eq!(data["efsMountTargets"]["items"][0]["subnetId"], "subnet-1");
        assert_eq!(data["efsMountTargets"]["items"][0]["ipAddress"], "10.0.0.1");
        assert_eq!(data["efsMountTargets"]["items"][0]["availabilityZone"], "us-east-1a");
        assert_eq!(data["efsMountTargets"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn efs_access_points_forwards_file_system_id_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{AP}?FileSystemId=fs-1"), ""),
            json_response(
                200,
                r#"{"AccessPoints":[{"AccessPointId":"fsap-1","AccessPointArn":"arn:aws:elasticfilesystem:us-east-1:1:access-point/fsap-1","FileSystemId":"fs-1","Name":"my-ap","LifeCycleState":"available"}]}"#,
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));
        let schema = build_query_schema(EfsQuery).data(client).finish();

        let res = schema
            .execute(
                r#"{ efsAccessPoints(fileSystemId: "fs-1") { items { accessPointId arn fileSystemId name lifeCycleState } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "{:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["efsAccessPoints"]["items"][0]["accessPointId"], "fsap-1");
        assert_eq!(
            data["efsAccessPoints"]["items"][0]["arn"],
            "arn:aws:elasticfilesystem:us-east-1:1:access-point/fsap-1"
        );
        assert_eq!(data["efsAccessPoints"]["items"][0]["fileSystemId"], "fs-1");
        assert_eq!(data["efsAccessPoints"]["items"][0]["name"], "my-ap");
        assert_eq!(data["efsAccessPoints"]["items"][0]["lifeCycleState"], "available");
        http_client.relaxed_requests_match();
    }
}
