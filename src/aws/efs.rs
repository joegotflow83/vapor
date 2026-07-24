use aws_config::SdkConfig;
use aws_sdk_efs::types::{AccessPointDescription, FileSystemDescription, MountTargetDescription};

use crate::aws::pagination::apply_limit;
use crate::error::VaporError;

pub struct EfsClient {
    inner: aws_sdk_efs::Client,
}

impl EfsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_efs::Client::new(config),
        }
    }

    /// Lists EFS file systems, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `DescribeFileSystems` has
    /// both `max_items` (this operation's `max_results`-equivalent field
    /// name) and `marker`/`next_marker` (verified against pinned
    /// `aws-sdk-efs` 1.106.0's `_describe_file_systems_input.rs`/
    /// `_describe_file_systems_output.rs`), so `limit` is capped to the
    /// remaining budget on the request itself (acm/kinesis/mq pattern).
    pub async fn describe_file_systems(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<FileSystemDescription>, Option<String>), VaporError> {
        let mut results = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_file_systems();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.max_items(l - results.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.file_systems.unwrap_or_default());
            token = output.next_marker;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if results.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((results, token))
    }

    /// Lists mount targets for a file system, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`.
    /// `DescribeMountTargets` has `marker`/`next_marker`, but its `max_items`
    /// field is documented as fixed at 10 with "other values are ignored"
    /// (verified against pinned `aws-sdk-efs` 1.106.0's
    /// `_describe_mount_targets_input.rs` doc comment) — same caveat class
    /// as `codedeploy.rs::list_applications`: `limit` can only be enforced
    /// via client-side `apply_limit` truncation, so when that trips mid-page
    /// the returned `next_token` is still AWS's *next*-page token,
    /// permanently skipping whatever was truncated off the current page.
    pub async fn describe_mount_targets(
        &self,
        file_system_id: &str,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<MountTargetDescription>, Option<String>), VaporError> {
        let mut results = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self
                .inner
                .describe_mount_targets()
                .file_system_id(file_system_id);
            if let Some(ref t) = token {
                req = req.marker(t);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.mount_targets.unwrap_or_default());
            token = output.next_marker;

            if apply_limit(&mut results, limit) || token.is_none() {
                break;
            }
        }

        Ok((results, token))
    }

    /// Lists EFS access points, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `DescribeAccessPoints` has both `max_results` and `next_token`
    /// (verified against pinned `aws-sdk-efs` 1.106.0's
    /// `_describe_access_points_input.rs`/`_describe_access_points_output.rs`),
    /// so `limit` is capped to the remaining budget on the request itself.
    pub async fn describe_access_points(
        &self,
        file_system_id: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<AccessPointDescription>, Option<String>), VaporError> {
        let mut results = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_access_points();
            if let Some(id) = file_system_id {
                req = req.file_system_id(id);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - results.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            results.extend(output.access_points.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if results.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((results, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const FS: &str = "https://elasticfilesystem.us-east-1.amazonaws.com/2015-02-01/file-systems";
    const MT: &str = "https://elasticfilesystem.us-east-1.amazonaws.com/2015-02-01/mount-targets";
    const AP: &str = "https://elasticfilesystem.us-east-1.amazonaws.com/2015-02-01/access-points";

    fn file_system_json(id: &str) -> String {
        format!(
            r#"{{"OwnerId":"acct-1","CreationToken":"tok-{id}","FileSystemId":"{id}","CreationTime":1700000000,"LifeCycleState":"available","NumberOfMountTargets":0,"PerformanceMode":"generalPurpose"}}"#
        )
    }

    fn mount_target_json(id: &str, fs_id: &str) -> String {
        format!(
            r#"{{"MountTargetId":"{id}","FileSystemId":"{fs_id}","SubnetId":"subnet-1","LifeCycleState":"available"}}"#
        )
    }

    fn access_point_json(id: &str, fs_id: &str) -> String {
        format!(
            r#"{{"AccessPointId":"{id}","FileSystemId":"{fs_id}","LifeCycleState":"available"}}"#
        )
    }

    #[tokio::test]
    async fn describe_file_systems_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FS, ""),
            json_response(
                200,
                format!(
                    r#"{{"FileSystems":[{},{}]}}"#,
                    file_system_json("fs-1"),
                    file_system_json("fs-2")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client.describe_file_systems(None, None).await.unwrap();

        assert_eq!(systems.len(), 2);
        assert_eq!(systems[0].file_system_id, "fs-1");
        assert_eq!(systems[1].file_system_id, "fs-2");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FS}?Marker=cursor-a"), ""),
            json_response(
                200,
                format!(r#"{{"FileSystems":[{}]}}"#, file_system_json("fs-3")),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client
            .describe_file_systems(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(systems.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_stops_at_limit_and_returns_resume_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{FS}?MaxItems=2"), ""),
            json_response(
                200,
                format!(
                    r#"{{"FileSystems":[{},{}],"NextMarker":"page2"}}"#,
                    file_system_json("fs-1"),
                    file_system_json("fs-2")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client.describe_file_systems(Some(2), None).await.unwrap();

        assert_eq!(systems.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{FS}?MaxItems=10"), ""),
                json_response(
                    200,
                    format!(
                        r#"{{"FileSystems":[{},{}],"NextMarker":"page2"}}"#,
                        file_system_json("fs-1"),
                        file_system_json("fs-2")
                    ),
                ),
            ),
            ReplayEvent::new(
                request(&format!("{FS}?MaxItems=8&Marker=page2"), ""),
                json_response(
                    200,
                    format!(r#"{{"FileSystems":[{}]}}"#, file_system_json("fs-3")),
                ),
            ),
        ]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (systems, token) = client.describe_file_systems(Some(10), None).await.unwrap();

        assert_eq!(systems.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_file_systems_propagates_errors() {
        // `BadRequest` rather than a throttling-classified code (e.g.
        // `TooManyRequestsException`) — those are on the SDK's built-in
        // retry-classifier denylist and would consume a second (nonexistent)
        // replay event instead of exercising `sdk_err`'s mapping path.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(FS, ""),
            json_error_response("BadRequest", "invalid parameter"),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_file_systems(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("BadRequest".to_string()));
                assert_eq!(message, "invalid parameter");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_mount_targets_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{MT}?FileSystemId=fs-1"), ""),
            json_response(
                200,
                format!(
                    r#"{{"MountTargets":[{},{}]}}"#,
                    mount_target_json("mt-1", "fs-1"),
                    mount_target_json("mt-2", "fs-1")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .describe_mount_targets("fs-1", None, None)
            .await
            .unwrap();

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].mount_target_id, "mt-1");
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_mount_targets_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{MT}?Marker=cursor-a&FileSystemId=fs-1"), ""),
            json_response(
                200,
                format!(
                    r#"{{"MountTargets":[{}]}}"#,
                    mount_target_json("mt-3", "fs-1")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .describe_mount_targets("fs-1", None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    /// File-specific caveat (see the doc comment on `describe_mount_targets`):
    /// `DescribeMountTargets`'s `max_items` is documented as ignored by the
    /// service, so `limit` is enforced by client-side `apply_limit`
    /// truncation. When that trips mid-page, the returned token is still
    /// AWS's *next*-page token — the items truncated off the current page
    /// are silently skipped, not recoverable via the returned token.
    #[tokio::test]
    async fn describe_mount_targets_client_side_limit_truncates_but_keeps_aws_next_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{MT}?FileSystemId=fs-1"), ""),
            json_response(
                200,
                format!(
                    r#"{{"MountTargets":[{},{},{}],"NextMarker":"aws-next"}}"#,
                    mount_target_json("mt-1", "fs-1"),
                    mount_target_json("mt-2", "fs-1"),
                    mount_target_json("mt-3", "fs-1")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .describe_mount_targets("fs-1", Some(2), None)
            .await
            .unwrap();

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].mount_target_id, "mt-1");
        assert_eq!(targets[1].mount_target_id, "mt-2");
        assert_eq!(token, Some("aws-next".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_mount_targets_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{MT}?FileSystemId=fs-1"), ""),
                json_response(
                    200,
                    format!(
                        r#"{{"MountTargets":[{},{}],"NextMarker":"p2"}}"#,
                        mount_target_json("mt-1", "fs-1"),
                        mount_target_json("mt-2", "fs-1")
                    ),
                ),
            ),
            ReplayEvent::new(
                request(&format!("{MT}?Marker=p2&FileSystemId=fs-1"), ""),
                json_response(
                    200,
                    format!(
                        r#"{{"MountTargets":[{}]}}"#,
                        mount_target_json("mt-3", "fs-1")
                    ),
                ),
            ),
        ]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (targets, token) = client
            .describe_mount_targets("fs-1", Some(10), None)
            .await
            .unwrap();

        assert_eq!(targets.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_mount_targets_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{MT}?FileSystemId=fs-1"), ""),
            json_error_response("FileSystemNotFound", "no such file system"),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_mount_targets("fs-1", None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("FileSystemNotFound".to_string()));
                assert_eq!(message, "no such file system");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_access_points_lists_all_when_no_filter_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(AP, ""),
            json_response(
                200,
                format!(
                    r#"{{"AccessPoints":[{},{}]}}"#,
                    access_point_json("fsap-1", "fs-1"),
                    access_point_json("fsap-2", "fs-2")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (points, token) = client
            .describe_access_points(None, None, None)
            .await
            .unwrap();

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].access_point_id.as_deref(), Some("fsap-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_access_points_filters_by_file_system_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{AP}?FileSystemId=fs-1"), ""),
            json_response(
                200,
                format!(
                    r#"{{"AccessPoints":[{}]}}"#,
                    access_point_json("fsap-1", "fs-1")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (points, token) = client
            .describe_access_points(Some("fs-1"), None, None)
            .await
            .unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_access_points_resumes_from_provided_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{AP}?NextToken=cursor-a"), ""),
            json_response(
                200,
                format!(
                    r#"{{"AccessPoints":[{}]}}"#,
                    access_point_json("fsap-2", "fs-2")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (points, token) = client
            .describe_access_points(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_access_points_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{AP}?MaxResults=1"), ""),
            json_response(
                200,
                format!(
                    r#"{{"AccessPoints":[{}],"NextToken":"page2"}}"#,
                    access_point_json("fsap-1", "fs-1")
                ),
            ),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (points, token) = client
            .describe_access_points(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_access_points_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{AP}?MaxResults=10"), ""),
                json_response(
                    200,
                    format!(
                        r#"{{"AccessPoints":[{},{}],"NextToken":"p2"}}"#,
                        access_point_json("fsap-1", "fs-1"),
                        access_point_json("fsap-2", "fs-2")
                    ),
                ),
            ),
            ReplayEvent::new(
                request(&format!("{AP}?MaxResults=8&NextToken=p2"), ""),
                json_response(
                    200,
                    format!(
                        r#"{{"AccessPoints":[{}]}}"#,
                        access_point_json("fsap-3", "fs-3")
                    ),
                ),
            ),
        ]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let (points, token) = client
            .describe_access_points(None, Some(10), None)
            .await
            .unwrap();

        assert_eq!(points.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_access_points_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(AP, ""),
            json_error_response("InternalServerError", "internal failure"),
        )]);
        let client = EfsClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_access_points(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InternalServerError".to_string()));
                assert_eq!(message, "internal failure");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
