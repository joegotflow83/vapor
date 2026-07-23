use aws_config::SdkConfig;
use aws_sdk_redshift::types::{Cluster, Snapshot};

use crate::error::VaporError;

pub struct RedshiftClient {
    inner: aws_sdk_redshift::Client,
}

impl RedshiftClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_redshift::Client::new(config),
        }
    }

    /// Lists clusters, optionally capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. `limit` is handed to AWS via
    /// `DescribeClustersInput::max_records` so a capped page boundary lands
    /// exactly on the returned marker, matching
    /// `specs/plan-2-schema-v2-pagination-timestamps.md`'s client-layer pattern.
    pub async fn describe_clusters(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Cluster>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_clusters();
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(l) = limit {
                req = req.max_records(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.clusters().to_vec());
            marker = output.marker().filter(|m| !m.is_empty()).map(|m| m.to_string());

            match (&marker, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, marker))
    }

    /// Lists cluster snapshots, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `limit` is handed to AWS via
    /// `DescribeClusterSnapshotsInput::max_records`, same pattern as
    /// `describe_clusters`.
    pub async fn describe_cluster_snapshots(
        &self,
        cluster_identifier: Option<String>,
        snapshot_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<Snapshot>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_cluster_snapshots();
            if let Some(ref id) = cluster_identifier {
                req = req.cluster_identifier(id);
            }
            if let Some(ref st) = snapshot_type {
                req = req.snapshot_type(st);
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            if let Some(l) = limit {
                req = req.max_records(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.snapshots().to_vec());
            marker = output.marker().filter(|m| !m.is_empty()).map(|m| m.to_string());

            match (&marker, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, marker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://redshift.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_clusters_happy_path_no_limit_no_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusters&Version=2012-12-01"),
            xml_response(
                200,
                "<DescribeClustersResponse><DescribeClustersResult><Clusters>\
                 <Cluster><ClusterIdentifier>my-cluster</ClusterIdentifier>\
                 <NodeType>dc2.large</NodeType><ClusterStatus>available</ClusterStatus>\
                 </Cluster></Clusters></DescribeClustersResult></DescribeClustersResponse>",
            ),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_clusters(None, None).await.unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_identifier(), Some("my-cluster"));
        assert_eq!(clusters[0].node_type(), Some("dc2.large"));
        assert_eq!(clusters[0].cluster_status(), Some("available"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusters&Version=2012-12-01&Marker=cursor-a"),
            xml_response(
                200,
                "<DescribeClustersResponse><DescribeClustersResult><Clusters>\
                 </Clusters></DescribeClustersResult></DescribeClustersResponse>",
            ),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .describe_clusters(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(clusters.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_stops_at_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusters&Version=2012-12-01&MaxRecords=2"),
            xml_response(
                200,
                "<DescribeClustersResponse><DescribeClustersResult><Clusters>\
                 <Cluster><ClusterIdentifier>a</ClusterIdentifier></Cluster>\
                 <Cluster><ClusterIdentifier>b</ClusterIdentifier></Cluster>\
                 </Clusters><Marker>page2</Marker></DescribeClustersResult></DescribeClustersResponse>",
            ),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_clusters(Some(2), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeClusters&Version=2012-12-01&MaxRecords=100"),
                xml_response(
                    200,
                    "<DescribeClustersResponse><DescribeClustersResult><Clusters>\
                     <Cluster><ClusterIdentifier>a</ClusterIdentifier></Cluster>\
                     </Clusters><Marker>p2</Marker></DescribeClustersResult></DescribeClustersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeClusters&Version=2012-12-01&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeClustersResponse><DescribeClustersResult><Clusters>\
                     <Cluster><ClusterIdentifier>b</ClusterIdentifier></Cluster>\
                     </Clusters></DescribeClustersResult></DescribeClustersResponse>",
                ),
            ),
        ]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_clusters(Some(100), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_clusters_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusters&Version=2012-12-01"),
            xml_error_response("ClusterNotFound", "cluster not found"),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_clusters(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ClusterNotFound".to_string()));
                assert_eq!(message, "cluster not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_snapshots_happy_path_with_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeClusterSnapshots&Version=2012-12-01&ClusterIdentifier=my-cluster&SnapshotType=manual",
            ),
            xml_response(
                200,
                "<DescribeClusterSnapshotsResponse><DescribeClusterSnapshotsResult><Snapshots>\
                 <Snapshot><SnapshotIdentifier>snap-1</SnapshotIdentifier>\
                 <ClusterIdentifier>my-cluster</ClusterIdentifier><Status>available</Status>\
                 <SnapshotType>manual</SnapshotType><NodeType>dc2.large</NodeType>\
                 </Snapshot></Snapshots></DescribeClusterSnapshotsResult></DescribeClusterSnapshotsResponse>",
            ),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client
            .describe_cluster_snapshots(Some("my-cluster".to_string()), Some("manual".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].snapshot_identifier(), Some("snap-1"));
        assert_eq!(snapshots[0].cluster_identifier(), Some("my-cluster"));
        assert_eq!(snapshots[0].status(), Some("available"));
        assert_eq!(snapshots[0].snapshot_type(), Some("manual"));
        assert_eq!(snapshots[0].node_type(), Some("dc2.large"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_snapshots_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusterSnapshots&Version=2012-12-01&Marker=cursor-b"),
            xml_response(
                200,
                "<DescribeClusterSnapshotsResponse><DescribeClusterSnapshotsResult><Snapshots>\
                 </Snapshots></DescribeClusterSnapshotsResult></DescribeClusterSnapshotsResponse>",
            ),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client
            .describe_cluster_snapshots(None, None, None, Some("cursor-b".to_string()))
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_snapshots_stops_at_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusterSnapshots&Version=2012-12-01&MaxRecords=1"),
            xml_response(
                200,
                "<DescribeClusterSnapshotsResponse><DescribeClusterSnapshotsResult><Snapshots>\
                 <Snapshot><SnapshotIdentifier>snap-1</SnapshotIdentifier></Snapshot>\
                 </Snapshots><Marker>next</Marker></DescribeClusterSnapshotsResult></DescribeClusterSnapshotsResponse>",
            ),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client.describe_cluster_snapshots(None, None, Some(1), None).await.unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(marker, Some("next".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_snapshots_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeClusterSnapshots&Version=2012-12-01&MaxRecords=50"),
                xml_response(
                    200,
                    "<DescribeClusterSnapshotsResponse><DescribeClusterSnapshotsResult><Snapshots>\
                     <Snapshot><SnapshotIdentifier>snap-1</SnapshotIdentifier></Snapshot>\
                     </Snapshots><Marker>p2</Marker></DescribeClusterSnapshotsResult></DescribeClusterSnapshotsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeClusterSnapshots&Version=2012-12-01&MaxRecords=49&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeClusterSnapshotsResponse><DescribeClusterSnapshotsResult><Snapshots>\
                     <Snapshot><SnapshotIdentifier>snap-2</SnapshotIdentifier></Snapshot>\
                     </Snapshots></DescribeClusterSnapshotsResult></DescribeClusterSnapshotsResponse>",
                ),
            ),
        ]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client.describe_cluster_snapshots(None, None, Some(50), None).await.unwrap();

        assert_eq!(snapshots.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cluster_snapshots_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusterSnapshots&Version=2012-12-01"),
            xml_error_response("ClusterSnapshotNotFound", "snapshot not found"),
        )]);
        let client = RedshiftClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_cluster_snapshots(None, None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ClusterSnapshotNotFound".to_string()));
                assert_eq!(message, "snapshot not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
