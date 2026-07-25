use std::collections::HashMap;

use aws_config::SdkConfig;

use crate::error::VaporError;

#[derive(Debug)]
pub struct StorageGatewayInfo {
    pub gateway_id: Option<String>,
    pub gateway_arn: Option<String>,
    pub gateway_type: Option<String>,
    pub gateway_name: Option<String>,
    pub gateway_operational_state: Option<String>,
    pub gateway_region: Option<String>,
    pub ec2_instance_id: Option<String>,
}

#[derive(Debug)]
pub struct StorageGatewayVolumeInfo {
    pub volume_arn: Option<String>,
    pub volume_id: Option<String>,
    pub gateway_arn: Option<String>,
    pub volume_type: Option<String>,
    pub volume_size_in_bytes: Option<i64>,
    pub volume_status: Option<String>,
}

#[derive(Debug)]
pub struct StorageGatewayFileShareInfo {
    pub file_share_arn: Option<String>,
    pub file_share_id: Option<String>,
    pub file_share_type: Option<String>,
    pub gateway_arn: Option<String>,
    pub path: Option<String>,
    pub file_share_status: Option<String>,
    pub location_arn: Option<String>,
}

pub struct StorageGatewayClient {
    inner: aws_sdk_storagegateway::Client,
}

impl StorageGatewayClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_storagegateway::Client::new(config),
        }
    }

    /// Lists gateways, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `limit` is handed to AWS via
    /// `ListGatewaysInput::limit` (verified against pinned `aws-sdk-storagegateway`
    /// 1.110.0) so a capped page boundary lands exactly on the returned token
    /// (kinesis/mq pattern); the continuation field is named `marker` on both
    /// request and response for this op (not `next_token`).
    pub async fn list_gateways(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<StorageGatewayInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_gateways();
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for gw in output.gateways() {
                items.push(StorageGatewayInfo {
                    gateway_id: gw.gateway_id().map(|s| s.to_string()),
                    gateway_arn: gw.gateway_arn().map(|s| s.to_string()),
                    gateway_type: gw.gateway_type().map(|s| s.to_string()),
                    gateway_name: gw.gateway_name().map(|s| s.to_string()),
                    gateway_operational_state: gw
                        .gateway_operational_state()
                        .map(|s| s.to_string()),
                    gateway_region: gw.ec2_instance_region().map(|s| s.to_string()),
                    ec2_instance_id: gw.ec2_instance_id().map(|s| s.to_string()),
                });
            }
            token = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists volumes attached to `gateway_arn`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`. `limit` is
    /// handed to AWS via `ListVolumesInput::limit` (verified against pinned
    /// `aws-sdk-storagegateway` 1.110.0) so a capped page boundary lands
    /// exactly on the returned token (kinesis/mq pattern).
    ///
    /// VTL gateways expose tapes instead of iSCSI volumes (see `list_tapes`),
    /// and `ListTapes` has no per-gateway filter, so tapes can't be given a
    /// real per-gateway resumption token. To avoid ever duplicating or
    /// dropping tapes across a stream of resumed calls, they're only
    /// appended once `ListVolumes` itself is confirmed exhausted (`token` is
    /// `None`) — that point occurs exactly once across a full resumed
    /// sequence for a given gateway. When appended, the combined list is
    /// truncated to `limit` client-side (`StorageGatewayClient::list_tapes`
    /// itself has no way to cap server-side either); any tape overflow past
    /// `limit` is silently dropped with no token to resume it — same
    /// documented caveat class as `cost_explorer.rs::get_cost_and_usage` /
    /// `polly.rs::describe_voices`.
    pub async fn list_volumes(
        &self,
        gateway_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<StorageGatewayVolumeInfo>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_volumes().gateway_arn(&gateway_arn);
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - items.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for vol in output.volume_infos() {
                items.push(StorageGatewayVolumeInfo {
                    volume_arn: vol.volume_arn().map(|s| s.to_string()),
                    volume_id: vol.volume_id().map(|s| s.to_string()),
                    gateway_arn: vol.gateway_arn().map(|s| s.to_string()),
                    volume_type: vol.volume_type().map(|s| s.to_string()),
                    volume_size_in_bytes: Some(vol.volume_size_in_bytes()),
                    volume_status: vol.volume_attachment_status().map(|s| s.to_string()),
                });
            }
            token = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        if token.is_none() && limit.is_none_or(|l| (items.len() as i32) < l) {
            items.extend(self.list_tapes(&gateway_arn).await?);
            if let Some(l) = limit {
                items.truncate(l as usize);
            }
        }

        Ok((items, token))
    }

    /// VTL gateways expose virtual tapes instead of iSCSI volumes.
    /// `ListTapes` has no gateway filter (it lists all tapes in the account's
    /// VTL/VTS), so results are filtered client-side by `gateway_arn`.
    async fn list_tapes(
        &self,
        gateway_arn: &str,
    ) -> Result<Vec<StorageGatewayVolumeInfo>, VaporError> {
        let mut items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = self.inner.list_tapes();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for tape in output.tape_infos() {
                if tape.gateway_arn() != Some(gateway_arn) {
                    continue;
                }
                items.push(StorageGatewayVolumeInfo {
                    volume_arn: tape.tape_arn().map(|s| s.to_string()),
                    volume_id: tape.tape_barcode().map(|s| s.to_string()),
                    gateway_arn: tape.gateway_arn().map(|s| s.to_string()),
                    volume_type: Some("VTL_TAPE".to_string()),
                    volume_size_in_bytes: tape.tape_size_in_bytes(),
                    volume_status: tape.tape_status().map(|s| s.to_string()),
                });
            }
            match output.marker() {
                Some(m) if !m.is_empty() => marker = Some(m.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    /// Lists file shares for `gateway_arn`, optionally capped at `limit`
    /// results (default unlimited) and resumed from `next_token`. `limit` is
    /// handed to AWS via `ListFileSharesInput::limit` (verified against
    /// pinned `aws-sdk-storagegateway` 1.110.0) so a capped page boundary
    /// lands exactly on the returned token (kinesis/mq pattern); this op's
    /// continuation field is `next_marker` on the response (distinct from
    /// `marker`, which just echoes the request marker back) fed into the
    /// next request's `marker` field. Per plan-2's "list-then-describe
    /// fan-out" convention, only this page's ARNs are sent to the batch
    /// NFS/SMB describe calls below, not the full account-wide list.
    pub async fn list_file_shares(
        &self,
        gateway_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<StorageGatewayFileShareInfo>, Option<String>), VaporError> {
        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_file_shares().gateway_arn(&gateway_arn);
            if let Some(ref t) = token {
                req = req.marker(t);
            }
            if let Some(l) = limit {
                req = req.limit(l - summaries.len() as i32);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for share in output.file_share_info_list() {
                summaries.push((
                    share.file_share_arn().map(|s| s.to_string()),
                    share.file_share_id().map(|s| s.to_string()),
                    share.file_share_type().map(|t| t.as_str().to_string()),
                    share.gateway_arn().map(|s| s.to_string()),
                    share.file_share_status().map(|s| s.to_string()),
                ));
            }
            token = match output.next_marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        if summaries.is_empty() {
            return Ok((Vec::new(), token));
        }

        // Collect ARNs by type for batch describe calls
        let mut nfs_arns: Vec<String> = Vec::new();
        let mut smb_arns: Vec<String> = Vec::new();
        for (arn, _, share_type, _, _) in &summaries {
            if let Some(arn) = arn {
                match share_type.as_deref() {
                    Some("NFS") => nfs_arns.push(arn.clone()),
                    Some("SMB") => smb_arns.push(arn.clone()),
                    _ => {}
                }
            }
        }

        // Batch describe NFS shares for path and location_arn
        let mut nfs_details: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
        if !nfs_arns.is_empty() {
            let mut req = self.inner.describe_nfs_file_shares();
            for arn in &nfs_arns {
                req = req.file_share_arn_list(arn);
            }
            if let Ok(output) = req.send().await {
                for info in output.nfs_file_share_info_list() {
                    if let Some(arn) = info.file_share_arn() {
                        nfs_details.insert(
                            arn.to_string(),
                            (
                                info.path().map(|s| s.to_string()),
                                info.location_arn().map(|s| s.to_string()),
                            ),
                        );
                    }
                }
            }
        }

        // Batch describe SMB shares for path and location_arn
        let mut smb_details: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
        if !smb_arns.is_empty() {
            let mut req = self.inner.describe_smb_file_shares();
            for arn in &smb_arns {
                req = req.file_share_arn_list(arn);
            }
            if let Ok(output) = req.send().await {
                for info in output.smb_file_share_info_list() {
                    if let Some(arn) = info.file_share_arn() {
                        smb_details.insert(
                            arn.to_string(),
                            (
                                info.path().map(|s| s.to_string()),
                                info.location_arn().map(|s| s.to_string()),
                            ),
                        );
                    }
                }
            }
        }

        let mut items = Vec::new();
        for (arn, id, share_type, gw_arn, status) in summaries {
            let (path, location_arn) = arn
                .as_deref()
                .and_then(|a| nfs_details.get(a).or_else(|| smb_details.get(a)))
                .cloned()
                .unwrap_or((None, None));

            items.push(StorageGatewayFileShareInfo {
                file_share_arn: arn,
                file_share_id: id,
                file_share_type: share_type,
                gateway_arn: gw_arn,
                path,
                file_share_status: status,
                location_arn,
            });
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const BASE: &str = "https://storagegateway.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn list_gateways_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"Gateways":[{"GatewayId":"gw-1","GatewayARN":"arn:aws:storagegateway:us-east-1:111111111111:gateway/gw-1","GatewayType":"CACHED","GatewayOperationalState":"ACTIVE","GatewayName":"main","Ec2InstanceId":"i-1","Ec2InstanceRegion":"us-east-1"}]}"#,
            ),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (gateways, token) = client.list_gateways(None, None).await.unwrap();

        assert_eq!(gateways.len(), 1);
        assert_eq!(gateways[0].gateway_id.as_deref(), Some("gw-1"));
        assert_eq!(
            gateways[0].gateway_arn.as_deref(),
            Some("arn:aws:storagegateway:us-east-1:111111111111:gateway/gw-1")
        );
        assert_eq!(gateways[0].gateway_type.as_deref(), Some("CACHED"));
        assert_eq!(
            gateways[0].gateway_operational_state.as_deref(),
            Some("ACTIVE")
        );
        assert_eq!(gateways[0].gateway_name.as_deref(), Some("main"));
        assert_eq!(gateways[0].ec2_instance_id.as_deref(), Some("i-1"));
        assert_eq!(gateways[0].gateway_region.as_deref(), Some("us-east-1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_gateways_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Marker":"cursor-a"}"#),
            json_response(200, r#"{"Gateways":[]}"#),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (gateways, token) = client
            .list_gateways(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(gateways.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_gateways_stops_at_limit_and_returns_resume_token() {
        // ListGatewaysInput has a `Limit` field forwarded straight to AWS
        // (verified against pinned `aws-sdk-storagegateway` 1.110.0's
        // `ser_list_gateways_input_input`) with no client-side truncation
        // afterwards, so the canned response must return exactly `limit`
        // items.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Limit":1}"#),
            json_response(
                200,
                r#"{"Gateways":[{"GatewayId":"gw-1"}],"Marker":"page2-token"}"#,
            ),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (gateways, token) = client.list_gateways(Some(1), None).await.unwrap();

        assert_eq!(gateways.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_gateways_propagates_errors() {
        // `InvalidGatewayRequestException`, not a throttling-classified code
        // (those get silently retried by the SDK's retry classifier,
        // exhausting the single replay event).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidGatewayRequestException", "bad request"),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let err = client.list_gateways(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidGatewayRequestException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_volumes_appends_tapes_once_volumes_are_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"GatewayARN":"gw-1"}"#),
                json_response(
                    200,
                    r#"{"VolumeInfos":[{"VolumeARN":"vol-arn-1","VolumeId":"vol-1","GatewayARN":"gw-1","VolumeType":"CACHED iSCSI","VolumeSizeInBytes":1024,"VolumeAttachmentStatus":"ATTACHED"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(
                    200,
                    r#"{"TapeInfos":[{"TapeARN":"tape-arn-1","TapeBarcode":"BARCODE1","GatewayARN":"gw-1","TapeSizeInBytes":2048,"TapeStatus":"AVAILABLE"}]}"#,
                ),
            ),
        ]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (volumes, token) = client
            .list_volumes("gw-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(volumes.len(), 2);
        assert_eq!(volumes[0].volume_id.as_deref(), Some("vol-1"));
        assert_eq!(volumes[0].volume_size_in_bytes, Some(1024));
        assert_eq!(volumes[1].volume_id.as_deref(), Some("BARCODE1"));
        assert_eq!(volumes[1].volume_type.as_deref(), Some("VTL_TAPE"));
        assert_eq!(volumes[1].volume_size_in_bytes, Some(2048));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_volumes_does_not_append_tapes_when_more_pages_remain() {
        // Capped exactly at `limit` with a resume token still outstanding —
        // the tape append only fires once `ListVolumes` itself is confirmed
        // exhausted (token `None`), so only 1 `ReplayEvent` should be
        // consumed here.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1","Limit":1}"#),
            json_response(
                200,
                r#"{"VolumeInfos":[{"VolumeId":"vol-1"}],"Marker":"page2-token"}"#,
            ),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (volumes, token) = client
            .list_volumes("gw-1".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(volumes.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_volumes_does_not_append_tapes_when_limit_reached_and_exhausted() {
        // Volumes loop hits both "token is None" and "limit reached" on the
        // same page — the append guard (`limit.map_or(true, |l| items.len()
        // < l)`) must still suppress the tape call even though the token
        // condition alone would otherwise allow it.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1","Limit":1}"#),
            json_response(200, r#"{"VolumeInfos":[{"VolumeId":"vol-1"}]}"#),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (volumes, token) = client
            .list_volumes("gw-1".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(volumes.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_volumes_truncates_appended_tapes_to_remaining_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"GatewayARN":"gw-1","Limit":2}"#),
                json_response(200, r#"{"VolumeInfos":[{"VolumeId":"vol-1"}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(
                    200,
                    r#"{"TapeInfos":[{"TapeARN":"tape-1","TapeBarcode":"BARCODE1","GatewayARN":"gw-1"},{"TapeARN":"tape-2","TapeBarcode":"BARCODE2","GatewayARN":"gw-1"},{"TapeARN":"tape-3","TapeBarcode":"BARCODE3","GatewayARN":"gw-1"}]}"#,
                ),
            ),
        ]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (volumes, token) = client
            .list_volumes("gw-1".to_string(), Some(2), None)
            .await
            .unwrap();

        assert_eq!(volumes.len(), 2);
        assert_eq!(volumes[0].volume_id.as_deref(), Some("vol-1"));
        assert_eq!(volumes[1].volume_id.as_deref(), Some("BARCODE1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_volumes_filters_appended_tapes_to_matching_gateway_arn() {
        // `ListTapes` has no per-gateway filter (lists all tapes in the
        // account's VTL/VTS), so the vapor wrapper filters client-side —
        // confirm a tape belonging to a different gateway is dropped.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"GatewayARN":"gw-1"}"#),
                json_response(200, r#"{"VolumeInfos":[]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(
                    200,
                    r#"{"TapeInfos":[{"TapeARN":"tape-1","TapeBarcode":"BARCODE1","GatewayARN":"gw-1"},{"TapeARN":"tape-2","TapeBarcode":"BARCODE2","GatewayARN":"gw-2"}]}"#,
                ),
            ),
        ]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (volumes, token) = client
            .list_volumes("gw-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].volume_id.as_deref(), Some("BARCODE1"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_volumes_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1"}"#),
            json_error_response("InvalidGatewayRequestException", "no such gateway"),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_volumes("gw-1".to_string(), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidGatewayRequestException".to_string()));
                assert_eq!(message, "no such gateway");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_file_shares_fans_out_to_nfs_and_smb_describe_calls() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"GatewayARN":"gw-1"}"#),
                json_response(
                    200,
                    r#"{"FileShareInfoList":[{"FileShareType":"NFS","FileShareARN":"arn-nfs-1","FileShareId":"fsid-1","FileShareStatus":"AVAILABLE","GatewayARN":"gw-1"},{"FileShareType":"SMB","FileShareARN":"arn-smb-1","FileShareId":"fsid-2","FileShareStatus":"AVAILABLE","GatewayARN":"gw-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"FileShareARNList":["arn-nfs-1"]}"#),
                json_response(
                    200,
                    r#"{"NFSFileShareInfoList":[{"FileShareARN":"arn-nfs-1","Path":"/nfs/path","LocationARN":"arn:aws:s3:::bucket/nfs"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"FileShareARNList":["arn-smb-1"]}"#),
                json_response(
                    200,
                    r#"{"SMBFileShareInfoList":[{"FileShareARN":"arn-smb-1","Path":"/smb/path","LocationARN":"arn:aws:s3:::bucket/smb"}]}"#,
                ),
            ),
        ]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_file_shares("gw-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0].file_share_arn.as_deref(), Some("arn-nfs-1"));
        assert_eq!(shares[0].file_share_type.as_deref(), Some("NFS"));
        assert_eq!(shares[0].path.as_deref(), Some("/nfs/path"));
        assert_eq!(
            shares[0].location_arn.as_deref(),
            Some("arn:aws:s3:::bucket/nfs")
        );
        assert_eq!(shares[1].file_share_arn.as_deref(), Some("arn-smb-1"));
        assert_eq!(shares[1].file_share_type.as_deref(), Some("SMB"));
        assert_eq!(shares[1].path.as_deref(), Some("/smb/path"));
        assert_eq!(
            shares[1].location_arn.as_deref(),
            Some("arn:aws:s3:::bucket/smb")
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_file_shares_returns_empty_early_without_any_fan_out_calls() {
        // Only 1 `ReplayEvent` is supplied — if the wrapper mistakenly
        // issued a describe fan-out call for an empty page, this test would
        // fail when the client has no more canned responses to hand out.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1"}"#),
            json_response(200, r#"{"FileShareInfoList":[]}"#),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_file_shares("gw-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_file_shares_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"GatewayARN":"gw-1","Limit":1}"#),
                json_response(
                    200,
                    r#"{"FileShareInfoList":[{"FileShareType":"NFS","FileShareARN":"arn-nfs-1"}],"NextMarker":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"FileShareARNList":["arn-nfs-1"]}"#),
                json_response(200, r#"{"NFSFileShareInfoList":[]}"#),
            ),
        ]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_file_shares("gw-1".to_string(), Some(1), None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_file_shares_swallows_fan_out_errors_leaving_path_none() {
        // The NFS/SMB describe fan-out calls fold their `Result` through
        // `if let Ok(output) = ...`, silently swallowing per-batch errors
        // instead of propagating them (same shape as gotcha 10's
        // `.ok()`-style fan-outs elsewhere in the sweep).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"GatewayARN":"gw-1"}"#),
                json_response(
                    200,
                    r#"{"FileShareInfoList":[{"FileShareType":"NFS","FileShareARN":"arn-nfs-1","FileShareId":"fsid-1"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"FileShareARNList":["arn-nfs-1"]}"#),
                json_error_response("InternalServerError", "internal failure"),
            ),
        ]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_file_shares("gw-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].file_share_id.as_deref(), Some("fsid-1"));
        assert_eq!(shares[0].path, None);
        assert_eq!(shares[0].location_arn, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_file_shares_leaves_path_none_for_unrecognized_share_type() {
        // An unrecognized `FileShareType` (smithy open enum) doesn't match
        // either `"NFS"`/`"SMB"` arm, so its ARN never joins either batch —
        // only 1 `ReplayEvent` is supplied to confirm no fan-out call fires.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1"}"#),
            json_response(
                200,
                r#"{"FileShareInfoList":[{"FileShareType":"OTHER","FileShareARN":"arn-1","FileShareId":"fsid-1"}]}"#,
            ),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let (shares, token) = client
            .list_file_shares("gw-1".to_string(), None, None)
            .await
            .unwrap();

        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].path, None);
        assert_eq!(shares[0].location_arn, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_file_shares_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1"}"#),
            json_error_response("InvalidGatewayRequestException", "no such gateway"),
        )]);
        let client = StorageGatewayClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_file_shares("gw-1".to_string(), None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidGatewayRequestException".to_string()));
                assert_eq!(message, "no such gateway");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
