use async_graphql::{Context, Object, Result};

use crate::aws::storage_gateway::StorageGatewayClient;
use crate::schema::pagination::Page;
use crate::schema::storage_gateway::types::{
    StorageGatewayFileShare, StorageGatewayGateway, StorageGatewayVolume,
};

#[derive(Default)]
pub struct StorageGatewayQuery;

#[Object]
impl StorageGatewayQuery {
    /// Lists gateways, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn storage_gateways(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<StorageGatewayGateway>> {
        let client = ctx.data::<StorageGatewayClient>()?;
        let (gateways, token) = client.list_gateways(limit, next_token).await?;
        Ok(Page {
            items: gateways.into_iter().map(StorageGatewayGateway::from).collect(),
            next_token: token,
        })
    }

    /// Lists volumes attached to a specific gateway, optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    async fn storage_gateway_volumes(
        &self,
        ctx: &Context<'_>,
        gateway_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<StorageGatewayVolume>> {
        let client = ctx.data::<StorageGatewayClient>()?;
        let (volumes, token) = client.list_volumes(gateway_arn, limit, next_token).await?;
        Ok(Page {
            items: volumes.into_iter().map(StorageGatewayVolume::from).collect(),
            next_token: token,
        })
    }

    /// Lists file shares for a gateway (NFS and SMB), optionally capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    async fn storage_gateway_file_shares(
        &self,
        ctx: &Context<'_>,
        gateway_arn: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<StorageGatewayFileShare>> {
        let client = ctx.data::<StorageGatewayClient>()?;
        let (shares, token) = client.list_file_shares(gateway_arn, limit, next_token).await?;
        Ok(Page {
            items: shares.into_iter().map(StorageGatewayFileShare::from).collect(),
            next_token: token,
        })
    }
}

// All 3 resolvers are 1:1 passthroughs to an already-tested
// `StorageGatewayClient` method each (including `storage_gateway_file_shares`'
// own NFS/SMB describe fan-out, which lives entirely inside
// `src/aws/storage_gateway.rs` — sso_admin/step_functions precedent), so
// they're exercised end-to-end here rather than given bespoke branch
// coverage. See that file's test module for the underlying
// pagination/limit/tape-append/error-mapping behavior.
#[cfg(test)]
mod tests {
    use crate::aws::storage_gateway::StorageGatewayClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::StorageGatewayQuery;

    const BASE: &str = "https://storagegateway.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn storage_gateways_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"Limit":1}"#),
            json_response(
                200,
                r#"{"Gateways":[{"GatewayId":"gw-1","GatewayARN":"arn:aws:storagegateway:us-east-1:111111111111:gateway/gw-1","GatewayType":"CACHED","GatewayOperationalState":"ACTIVE","GatewayName":"main","Ec2InstanceId":"i-1","Ec2InstanceRegion":"us-east-1"}],"Marker":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(StorageGatewayQuery)
            .data(StorageGatewayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ storageGateways(limit: 1) { items { gatewayId gatewayArn gatewayType gatewayName gatewayOperationalState gatewayRegion ec2InstanceId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["storageGateways"]["items"];
        assert_eq!(items[0]["gatewayId"], "gw-1");
        assert_eq!(
            items[0]["gatewayArn"],
            "arn:aws:storagegateway:us-east-1:111111111111:gateway/gw-1"
        );
        assert_eq!(items[0]["gatewayType"], "CACHED");
        assert_eq!(items[0]["gatewayName"], "main");
        assert_eq!(items[0]["gatewayOperationalState"], "ACTIVE");
        assert_eq!(items[0]["gatewayRegion"], "us-east-1");
        assert_eq!(items[0]["ec2InstanceId"], "i-1");
        assert_eq!(json["storageGateways"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn storage_gateway_volumes_maps_items_and_forwards_gateway_arn() {
        // `Marker` is present in the response, so `list_volumes`' tape-append
        // guard (`token.is_none() && ...`) never fires — only 1 `ReplayEvent`
        // is needed here (the tape-append branch is covered at the aws layer).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1","Limit":1}"#),
            json_response(
                200,
                r#"{"VolumeInfos":[{"VolumeARN":"vol-arn-1","VolumeId":"vol-1","GatewayARN":"gw-1","VolumeType":"CACHED iSCSI","VolumeSizeInBytes":1024,"VolumeAttachmentStatus":"ATTACHED"}],"Marker":"page2-token"}"#,
            ),
        )]);
        let schema = build_query_schema(StorageGatewayQuery)
            .data(StorageGatewayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ storageGatewayVolumes(gatewayArn: "gw-1", limit: 1) { items { volumeArn volumeId gatewayArn volumeType volumeSizeInBytes volumeStatus } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["storageGatewayVolumes"]["items"];
        assert_eq!(items[0]["volumeArn"], "vol-arn-1");
        assert_eq!(items[0]["volumeId"], "vol-1");
        assert_eq!(items[0]["gatewayArn"], "gw-1");
        assert_eq!(items[0]["volumeType"], "CACHED iSCSI");
        assert_eq!(items[0]["volumeSizeInBytes"], 1024);
        assert_eq!(items[0]["volumeStatus"], "ATTACHED");
        assert_eq!(
            json["storageGatewayVolumes"]["nextToken"],
            "page2-token"
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn storage_gateway_file_shares_maps_items() {
        // `FileShareType` "OTHER" doesn't match the NFS/SMB fan-out arms in
        // `list_file_shares` (aws/storage_gateway.rs precedent), so only 1
        // `ReplayEvent` is needed here — the NFS/SMB describe fan-out itself
        // is covered at the aws layer.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"GatewayARN":"gw-1","Limit":1}"#),
            json_response(
                200,
                r#"{"FileShareInfoList":[{"FileShareType":"OTHER","FileShareARN":"arn-1","FileShareId":"fsid-1","GatewayARN":"gw-1","FileShareStatus":"AVAILABLE"}],"NextMarker":"page2-token"}"#,
            ),
        )]);
        let schema = build_query_schema(StorageGatewayQuery)
            .data(StorageGatewayClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ storageGatewayFileShares(gatewayArn: "gw-1", limit: 1) { items { fileShareArn fileShareId fileShareType gatewayArn path fileShareStatus locationArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["storageGatewayFileShares"]["items"];
        assert_eq!(items[0]["fileShareArn"], "arn-1");
        assert_eq!(items[0]["fileShareId"], "fsid-1");
        assert_eq!(items[0]["fileShareType"], "OTHER");
        assert_eq!(items[0]["gatewayArn"], "gw-1");
        assert!(items[0]["path"].is_null());
        assert_eq!(items[0]["fileShareStatus"], "AVAILABLE");
        assert!(items[0]["locationArn"].is_null());
        assert_eq!(
            json["storageGatewayFileShares"]["nextToken"],
            "page2-token"
        );
        http_client.relaxed_requests_match();
    }
}
