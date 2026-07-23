use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct RdsClient {
    inner: aws_sdk_rds::Client,
}

impl RdsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_rds::Client::new(config),
        }
    }

    /// Describe DB instances. If `ids` is given, each identifier is fetched
    /// independently (a non-resumable fan-out, `next_token` always `None`,
    /// still capped by `limit`); otherwise lists one resumable page via
    /// `next_token`. `DescribeDBInstances` has both `max_records` and
    /// `marker` (verified against pinned `aws-sdk-rds` 1.137.1), with AWS
    /// enforcing `MaxRecords` in [20, 100] — clamp + truncate client-side for
    /// a `limit` outside that range (documentdb/neptune precedent).
    pub async fn describe_db_instances(
        &self,
        ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_rds::types::DbInstance>, Option<String>), VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbInstance> = Vec::new();

        if let Some(ids) = ids {
            for id in ids {
                let output = self
                    .inner
                    .describe_db_instances()
                    .db_instance_identifier(&id)
                    .send()
                    .await
                    .map_err(crate::error::sdk_err)?;
                all_items.extend(output.db_instances().iter().cloned());
            }
            if let Some(l) = limit {
                all_items.truncate(l.max(0) as usize);
            }
            return Ok((all_items, None));
        }

        let mut marker = next_token;
        loop {
            let mut request = self.inner.describe_db_instances();
            if let Some(l) = limit {
                request = request.max_records((l - all_items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_items.extend(output.db_instances().iter().cloned());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| all_items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            all_items.truncate(l.max(0) as usize);
        }

        Ok((all_items, marker))
    }

    /// Describe DB clusters. Same dual-mode shape as `describe_db_instances`:
    /// `ids` given → non-resumable fan-out (still capped by `limit`); `ids`
    /// omitted → resumable one-page list. `DescribeDBClusters` has both
    /// `max_records` and `marker` (verified against pinned `aws-sdk-rds`
    /// 1.137.1), same [20, 100] `MaxRecords` constraint.
    pub async fn describe_db_clusters(
        &self,
        ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_rds::types::DbCluster>, Option<String>), VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbCluster> = Vec::new();

        if let Some(ids) = ids {
            for id in ids {
                let output = self
                    .inner
                    .describe_db_clusters()
                    .db_cluster_identifier(&id)
                    .send()
                    .await
                    .map_err(crate::error::sdk_err)?;
                all_items.extend(output.db_clusters().iter().cloned());
            }
            if let Some(l) = limit {
                all_items.truncate(l.max(0) as usize);
            }
            return Ok((all_items, None));
        }

        let mut marker = next_token;
        loop {
            let mut request = self.inner.describe_db_clusters();
            if let Some(l) = limit {
                request = request.max_records((l - all_items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_items.extend(output.db_clusters().iter().cloned());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| all_items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            all_items.truncate(l.max(0) as usize);
        }

        Ok((all_items, marker))
    }

    /// Describe DB parameter groups, resumable via `next_token`. `name` is
    /// rebuilt into the request each loop iteration (fsx/transcribe/ses
    /// precedent). `DescribeDBParameterGroups` has both `max_records` and
    /// `marker` (verified against pinned `aws-sdk-rds` 1.137.1), same
    /// [20, 100] `MaxRecords` constraint.
    pub async fn describe_db_parameter_groups(
        &self,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_rds::types::DbParameterGroup>, Option<String>), VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbParameterGroup> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut request = self.inner.describe_db_parameter_groups();
            if let Some(ref n) = name {
                request = request.db_parameter_group_name(n);
            }
            if let Some(l) = limit {
                request = request.max_records((l - all_items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_items.extend(output.db_parameter_groups().iter().cloned());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| all_items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            all_items.truncate(l.max(0) as usize);
        }

        Ok((all_items, marker))
    }

    /// Describe DB subnet groups, resumable via `next_token`. `name` is
    /// rebuilt into the request each loop iteration (fsx/transcribe/ses
    /// precedent). `DescribeDBSubnetGroups` has both `max_records` and
    /// `marker` (verified against pinned `aws-sdk-rds` 1.137.1), same
    /// [20, 100] `MaxRecords` constraint.
    pub async fn describe_db_subnet_groups(
        &self,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_rds::types::DbSubnetGroup>, Option<String>), VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbSubnetGroup> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut request = self.inner.describe_db_subnet_groups();
            if let Some(ref n) = name {
                request = request.db_subnet_group_name(n);
            }
            if let Some(l) = limit {
                request = request.max_records((l - all_items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }
            let output = request.send().await.map_err(crate::error::sdk_err)?;
            all_items.extend(output.db_subnet_groups().iter().cloned());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| all_items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            all_items.truncate(l.max(0) as usize);
        }

        Ok((all_items, marker))
    }

    /// Describe DB snapshots, resumable via `next_token`. `db_instance_id`/
    /// `snapshot_type` filters are rebuilt into the request each loop
    /// iteration (fsx/transcribe/ses precedent). `DescribeDBSnapshots` has
    /// both `max_records` and `marker` (verified against pinned
    /// `aws-sdk-rds` 1.137.1), same [20, 100] `MaxRecords` constraint.
    pub async fn describe_db_snapshots(
        &self,
        db_instance_id: Option<String>,
        snapshot_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_rds::types::DbSnapshot>, Option<String>), VaporError> {
        let mut all_items: Vec<aws_sdk_rds::types::DbSnapshot> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut request = self.inner.describe_db_snapshots();

            if let Some(ref id) = db_instance_id {
                request = request.db_instance_identifier(id);
            }
            if let Some(ref st) = snapshot_type {
                request = request.snapshot_type(st);
            }
            if let Some(l) = limit {
                request = request.max_records((l - all_items.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                request = request.marker(m);
            }

            let output = request.send().await.map_err(crate::error::sdk_err)?;

            all_items.extend(output.db_snapshots().iter().cloned());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| all_items.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            all_items.truncate(l.max(0) as usize);
        }

        Ok((all_items, marker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://rds.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_db_instances_happy_path_no_ids_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31"),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 <DBInstance><DBInstanceIdentifier>my-instance</DBInstanceIdentifier>\
                 <DBInstanceClass>db.r5.large</DBInstanceClass><Engine>postgres</Engine>\
                 <DBInstanceStatus>available</DBInstanceStatus><AvailabilityZone>us-east-1a</AvailabilityZone>\
                 </DBInstance></DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client.describe_db_instances(None, None, None).await.unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].db_instance_identifier(), Some("my-instance"));
        assert_eq!(instances[0].db_instance_class(), Some("db.r5.large"));
        assert_eq!(instances[0].engine(), Some("postgres"));
        assert_eq!(instances[0].db_instance_status(), Some("available"));
        assert_eq!(instances[0].availability_zone(), Some("us-east-1a"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31&Marker=cursor-a"),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client
            .describe_db_instances(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(instances.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31&MaxRecords=20"),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 <DBInstance><DBInstanceIdentifier>a</DBInstanceIdentifier></DBInstance>\
                 <DBInstance><DBInstanceIdentifier>b</DBInstanceIdentifier></DBInstance>\
                 <DBInstance><DBInstanceIdentifier>c</DBInstanceIdentifier></DBInstance>\
                 </DBInstances><Marker>page2</Marker></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client.describe_db_instances(None, Some(2), None).await.unwrap();

        assert_eq!(instances.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31&MaxRecords=100"),
                xml_response(
                    200,
                    "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                     <DBInstance><DBInstanceIdentifier>a</DBInstanceIdentifier></DBInstance>\
                     </DBInstances><Marker>p2</Marker></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeDBInstances&Version=2014-10-31&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                     <DBInstance><DBInstanceIdentifier>b</DBInstanceIdentifier></DBInstance>\
                     </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client.describe_db_instances(None, Some(100), None).await.unwrap();

        assert_eq!(instances.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_fan_out_with_ids() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31&DBInstanceIdentifier=a"),
                xml_response(
                    200,
                    "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                     <DBInstance><DBInstanceIdentifier>a</DBInstanceIdentifier></DBInstance>\
                     </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31&DBInstanceIdentifier=b"),
                xml_response(
                    200,
                    "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                     <DBInstance><DBInstanceIdentifier>b</DBInstanceIdentifier></DBInstance>\
                     </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client
            .describe_db_instances(Some(vec!["a".to_string(), "b".to_string()]), None, None)
            .await
            .unwrap();

        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].db_instance_identifier(), Some("a"));
        assert_eq!(instances[1].db_instance_identifier(), Some("b"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_fan_out_with_ids_respects_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31&DBInstanceIdentifier=a"),
                xml_response(
                    200,
                    "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                     <DBInstance><DBInstanceIdentifier>a</DBInstanceIdentifier></DBInstance>\
                     </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31&DBInstanceIdentifier=b"),
                xml_response(
                    200,
                    "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                     <DBInstance><DBInstanceIdentifier>b</DBInstanceIdentifier></DBInstance>\
                     </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client
            .describe_db_instances(Some(vec!["a".to_string(), "b".to_string()]), Some(1), None)
            .await
            .unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].db_instance_identifier(), Some("a"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31"),
            xml_error_response("DBInstanceNotFound", "no such instance"),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_db_instances(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DBInstanceNotFound".to_string()));
                assert_eq!(message, "no such instance");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_happy_path_no_ids_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31"),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 <DBCluster><DBClusterIdentifier>my-cluster</DBClusterIdentifier><Status>available</Status>\
                 <Engine>aurora-postgresql</Engine><EngineVersion>15.4</EngineVersion><Port>5432</Port>\
                 <MultiAZ>true</MultiAZ></DBCluster>\
                 </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_db_clusters(None, None, None).await.unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].db_cluster_identifier(), Some("my-cluster"));
        assert_eq!(clusters[0].status(), Some("available"));
        assert_eq!(clusters[0].engine(), Some("aurora-postgresql"));
        assert_eq!(clusters[0].engine_version(), Some("15.4"));
        assert_eq!(clusters[0].port(), Some(5432));
        assert_eq!(clusters[0].multi_az(), Some(true));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&Marker=cursor-b"),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .describe_db_clusters(None, None, Some("cursor-b".to_string()))
            .await
            .unwrap();

        assert_eq!(clusters.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&MaxRecords=20"),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 <DBCluster><DBClusterIdentifier>a</DBClusterIdentifier></DBCluster>\
                 <DBCluster><DBClusterIdentifier>b</DBClusterIdentifier></DBCluster>\
                 <DBCluster><DBClusterIdentifier>c</DBClusterIdentifier></DBCluster>\
                 </DBClusters><Marker>page2</Marker></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_db_clusters(None, Some(2), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&MaxRecords=100"),
                xml_response(
                    200,
                    "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                     <DBCluster><DBClusterIdentifier>a</DBClusterIdentifier></DBCluster>\
                     </DBClusters><Marker>p2</Marker></DescribeDBClustersResult></DescribeDBClustersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeDBClusters&Version=2014-10-31&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                     <DBCluster><DBClusterIdentifier>b</DBClusterIdentifier></DBCluster>\
                     </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_db_clusters(None, Some(100), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_fan_out_with_ids() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&DBClusterIdentifier=a"),
                xml_response(
                    200,
                    "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                     <DBCluster><DBClusterIdentifier>a</DBClusterIdentifier></DBCluster>\
                     </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&DBClusterIdentifier=b"),
                xml_response(
                    200,
                    "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                     <DBCluster><DBClusterIdentifier>b</DBClusterIdentifier></DBCluster>\
                     </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .describe_db_clusters(Some(vec!["a".to_string(), "b".to_string()]), None, None)
            .await
            .unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].db_cluster_identifier(), Some("a"));
        assert_eq!(clusters[1].db_cluster_identifier(), Some("b"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_fan_out_with_ids_respects_limit() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&DBClusterIdentifier=a"),
                xml_response(
                    200,
                    "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                     <DBCluster><DBClusterIdentifier>a</DBClusterIdentifier></DBCluster>\
                     </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&DBClusterIdentifier=b"),
                xml_response(
                    200,
                    "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                     <DBCluster><DBClusterIdentifier>b</DBClusterIdentifier></DBCluster>\
                     </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .describe_db_clusters(Some(vec!["a".to_string(), "b".to_string()]), Some(1), None)
            .await
            .unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].db_cluster_identifier(), Some("a"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31"),
            xml_error_response("DBClusterNotFoundFault", "no such cluster"),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_db_clusters(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DBClusterNotFoundFault".to_string()));
                assert_eq!(message, "no such cluster");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_parameter_groups_happy_path_with_name() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBParameterGroups&Version=2014-10-31&DBParameterGroupName=my-family",
            ),
            xml_response(
                200,
                "<DescribeDBParameterGroupsResponse><DescribeDBParameterGroupsResult><DBParameterGroups>\
                 <DBParameterGroup><DBParameterGroupName>my-family</DBParameterGroupName>\
                 <DBParameterGroupFamily>postgres15</DBParameterGroupFamily>\
                 <Description>custom params</Description></DBParameterGroup>\
                 </DBParameterGroups></DescribeDBParameterGroupsResult></DescribeDBParameterGroupsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client
            .describe_db_parameter_groups(Some("my-family".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].db_parameter_group_name(), Some("my-family"));
        assert_eq!(groups[0].db_parameter_group_family(), Some("postgres15"));
        assert_eq!(groups[0].description(), Some("custom params"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_parameter_groups_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBParameterGroups&Version=2014-10-31&Marker=cursor-c"),
            xml_response(
                200,
                "<DescribeDBParameterGroupsResponse><DescribeDBParameterGroupsResult><DBParameterGroups>\
                 </DBParameterGroups></DescribeDBParameterGroupsResult></DescribeDBParameterGroupsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client
            .describe_db_parameter_groups(None, None, Some("cursor-c".to_string()))
            .await
            .unwrap();

        assert_eq!(groups.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_parameter_groups_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBParameterGroups&Version=2014-10-31&MaxRecords=20"),
            xml_response(
                200,
                "<DescribeDBParameterGroupsResponse><DescribeDBParameterGroupsResult><DBParameterGroups>\
                 <DBParameterGroup><DBParameterGroupName>a</DBParameterGroupName></DBParameterGroup>\
                 <DBParameterGroup><DBParameterGroupName>b</DBParameterGroupName></DBParameterGroup>\
                 <DBParameterGroup><DBParameterGroupName>c</DBParameterGroupName></DBParameterGroup>\
                 </DBParameterGroups><Marker>page2</Marker></DescribeDBParameterGroupsResult></DescribeDBParameterGroupsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.describe_db_parameter_groups(None, Some(2), None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_parameter_groups_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBParameterGroups&Version=2014-10-31&MaxRecords=100"),
                xml_response(
                    200,
                    "<DescribeDBParameterGroupsResponse><DescribeDBParameterGroupsResult><DBParameterGroups>\
                     <DBParameterGroup><DBParameterGroupName>a</DBParameterGroupName></DBParameterGroup>\
                     </DBParameterGroups><Marker>p2</Marker></DescribeDBParameterGroupsResult></DescribeDBParameterGroupsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeDBParameterGroups&Version=2014-10-31&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeDBParameterGroupsResponse><DescribeDBParameterGroupsResult><DBParameterGroups>\
                     <DBParameterGroup><DBParameterGroupName>b</DBParameterGroupName></DBParameterGroup>\
                     </DBParameterGroups></DescribeDBParameterGroupsResult></DescribeDBParameterGroupsResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.describe_db_parameter_groups(None, Some(100), None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_parameter_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBParameterGroups&Version=2014-10-31"),
            xml_error_response("DBParameterGroupNotFound", "no such parameter group"),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_db_parameter_groups(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DBParameterGroupNotFound".to_string()));
                assert_eq!(message, "no such parameter group");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_subnet_groups_happy_path_with_name() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBSubnetGroups&Version=2014-10-31&DBSubnetGroupName=my-subnet-group",
            ),
            xml_response(
                200,
                "<DescribeDBSubnetGroupsResponse><DescribeDBSubnetGroupsResult><DBSubnetGroups>\
                 <DBSubnetGroup><DBSubnetGroupName>my-subnet-group</DBSubnetGroupName>\
                 <DBSubnetGroupDescription>default vpc subnets</DBSubnetGroupDescription>\
                 <VpcId>vpc-123</VpcId><SubnetGroupStatus>Complete</SubnetGroupStatus></DBSubnetGroup>\
                 </DBSubnetGroups></DescribeDBSubnetGroupsResult></DescribeDBSubnetGroupsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client
            .describe_db_subnet_groups(Some("my-subnet-group".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].db_subnet_group_name(), Some("my-subnet-group"));
        assert_eq!(groups[0].db_subnet_group_description(), Some("default vpc subnets"));
        assert_eq!(groups[0].vpc_id(), Some("vpc-123"));
        assert_eq!(groups[0].subnet_group_status(), Some("Complete"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_subnet_groups_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBSubnetGroups&Version=2014-10-31&Marker=cursor-d"),
            xml_response(
                200,
                "<DescribeDBSubnetGroupsResponse><DescribeDBSubnetGroupsResult><DBSubnetGroups>\
                 </DBSubnetGroups></DescribeDBSubnetGroupsResult></DescribeDBSubnetGroupsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client
            .describe_db_subnet_groups(None, None, Some("cursor-d".to_string()))
            .await
            .unwrap();

        assert_eq!(groups.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_subnet_groups_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBSubnetGroups&Version=2014-10-31&MaxRecords=20"),
            xml_response(
                200,
                "<DescribeDBSubnetGroupsResponse><DescribeDBSubnetGroupsResult><DBSubnetGroups>\
                 <DBSubnetGroup><DBSubnetGroupName>a</DBSubnetGroupName></DBSubnetGroup>\
                 <DBSubnetGroup><DBSubnetGroupName>b</DBSubnetGroupName></DBSubnetGroup>\
                 <DBSubnetGroup><DBSubnetGroupName>c</DBSubnetGroupName></DBSubnetGroup>\
                 </DBSubnetGroups><Marker>page2</Marker></DescribeDBSubnetGroupsResult></DescribeDBSubnetGroupsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.describe_db_subnet_groups(None, Some(2), None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_subnet_groups_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBSubnetGroups&Version=2014-10-31&MaxRecords=100"),
                xml_response(
                    200,
                    "<DescribeDBSubnetGroupsResponse><DescribeDBSubnetGroupsResult><DBSubnetGroups>\
                     <DBSubnetGroup><DBSubnetGroupName>a</DBSubnetGroupName></DBSubnetGroup>\
                     </DBSubnetGroups><Marker>p2</Marker></DescribeDBSubnetGroupsResult></DescribeDBSubnetGroupsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeDBSubnetGroups&Version=2014-10-31&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeDBSubnetGroupsResponse><DescribeDBSubnetGroupsResult><DBSubnetGroups>\
                     <DBSubnetGroup><DBSubnetGroupName>b</DBSubnetGroupName></DBSubnetGroup>\
                     </DBSubnetGroups></DescribeDBSubnetGroupsResult></DescribeDBSubnetGroupsResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.describe_db_subnet_groups(None, Some(100), None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_subnet_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBSubnetGroups&Version=2014-10-31"),
            xml_error_response("DBSubnetGroupNotFoundFault", "no such subnet group"),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_db_subnet_groups(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DBSubnetGroupNotFoundFault".to_string()));
                assert_eq!(message, "no such subnet group");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_snapshots_happy_path_no_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBSnapshots&Version=2014-10-31"),
            xml_response(
                200,
                "<DescribeDBSnapshotsResponse><DescribeDBSnapshotsResult><DBSnapshots>\
                 <DBSnapshot><DBSnapshotIdentifier>snap-1</DBSnapshotIdentifier>\
                 <DBInstanceIdentifier>my-instance</DBInstanceIdentifier><Engine>postgres</Engine>\
                 <Status>available</Status><Port>5432</Port><AvailabilityZone>us-east-1a</AvailabilityZone>\
                 <SnapshotType>automated</SnapshotType></DBSnapshot>\
                 </DBSnapshots></DescribeDBSnapshotsResult></DescribeDBSnapshotsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client.describe_db_snapshots(None, None, None, None).await.unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].db_snapshot_identifier(), Some("snap-1"));
        assert_eq!(snapshots[0].db_instance_identifier(), Some("my-instance"));
        assert_eq!(snapshots[0].engine(), Some("postgres"));
        assert_eq!(snapshots[0].status(), Some("available"));
        assert_eq!(snapshots[0].port(), Some(5432));
        assert_eq!(snapshots[0].availability_zone(), Some("us-east-1a"));
        assert_eq!(snapshots[0].snapshot_type(), Some("automated"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_snapshots_happy_path_with_filters() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBSnapshots&Version=2014-10-31&DBInstanceIdentifier=my-instance&SnapshotType=manual",
            ),
            xml_response(
                200,
                "<DescribeDBSnapshotsResponse><DescribeDBSnapshotsResult><DBSnapshots>\
                 <DBSnapshot><DBSnapshotIdentifier>snap-2</DBSnapshotIdentifier>\
                 <DBInstanceIdentifier>my-instance</DBInstanceIdentifier><SnapshotType>manual</SnapshotType>\
                 </DBSnapshot></DBSnapshots></DescribeDBSnapshotsResult></DescribeDBSnapshotsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client
            .describe_db_snapshots(Some("my-instance".to_string()), Some("manual".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].db_snapshot_identifier(), Some("snap-2"));
        assert_eq!(snapshots[0].snapshot_type(), Some("manual"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_snapshots_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBSnapshots&Version=2014-10-31&Marker=cursor-e"),
            xml_response(
                200,
                "<DescribeDBSnapshotsResponse><DescribeDBSnapshotsResult><DBSnapshots>\
                 </DBSnapshots></DescribeDBSnapshotsResult></DescribeDBSnapshotsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client
            .describe_db_snapshots(None, None, None, Some("cursor-e".to_string()))
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_snapshots_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBSnapshots&Version=2014-10-31&MaxRecords=20"),
            xml_response(
                200,
                "<DescribeDBSnapshotsResponse><DescribeDBSnapshotsResult><DBSnapshots>\
                 <DBSnapshot><DBSnapshotIdentifier>a</DBSnapshotIdentifier></DBSnapshot>\
                 <DBSnapshot><DBSnapshotIdentifier>b</DBSnapshotIdentifier></DBSnapshot>\
                 <DBSnapshot><DBSnapshotIdentifier>c</DBSnapshotIdentifier></DBSnapshot>\
                 </DBSnapshots><Marker>page2</Marker></DescribeDBSnapshotsResult></DescribeDBSnapshotsResponse>",
            ),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client.describe_db_snapshots(None, None, Some(2), None).await.unwrap();

        assert_eq!(snapshots.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_snapshots_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=DescribeDBSnapshots&Version=2014-10-31&MaxRecords=100"),
                xml_response(
                    200,
                    "<DescribeDBSnapshotsResponse><DescribeDBSnapshotsResult><DBSnapshots>\
                     <DBSnapshot><DBSnapshotIdentifier>a</DBSnapshotIdentifier></DBSnapshot>\
                     </DBSnapshots><Marker>p2</Marker></DescribeDBSnapshotsResult></DescribeDBSnapshotsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeDBSnapshots&Version=2014-10-31&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeDBSnapshotsResponse><DescribeDBSnapshotsResult><DBSnapshots>\
                     <DBSnapshot><DBSnapshotIdentifier>b</DBSnapshotIdentifier></DBSnapshot>\
                     </DBSnapshots></DescribeDBSnapshotsResult></DescribeDBSnapshotsResponse>",
                ),
            ),
        ]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let (snapshots, marker) = client.describe_db_snapshots(None, None, Some(100), None).await.unwrap();

        assert_eq!(snapshots.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_snapshots_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBSnapshots&Version=2014-10-31"),
            xml_error_response("DBSnapshotNotFound", "no such snapshot"),
        )]);
        let client = RdsClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_db_snapshots(None, None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DBSnapshotNotFound".to_string()));
                assert_eq!(message, "no such snapshot");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
