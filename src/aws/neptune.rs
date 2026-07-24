use aws_config::SdkConfig;
use aws_sdk_neptune::types::{DbCluster, DbInstance, Filter};

use crate::error::VaporError;

pub struct NeptuneClient {
    inner: aws_sdk_neptune::Client,
}

impl NeptuneClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_neptune::Client::new(config),
        }
    }

    pub async fn describe_db_clusters(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DbCluster>, Option<String>), VaporError> {
        let mut clusters = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_db_clusters();
            if let Some(l) = limit {
                // AWS enforces MaxRecords in [20, 100] for this op; a `limit`
                // below 20 can't be requested exactly, so we clamp and
                // truncate client-side below. When that happens the returned
                // marker points past the whole fetched page (not just the
                // truncated prefix we return), so resuming skips whatever was
                // truncated off — same caveat class as
                // cost_explorer.rs::get_cost_and_usage.
                req = req.max_records((l - clusters.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            clusters.extend(output.db_clusters().to_vec());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| clusters.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            clusters.truncate(l.max(0) as usize);
        }

        Ok((clusters, marker))
    }

    pub async fn describe_db_instances(
        &self,
        cluster_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<DbInstance>, Option<String>), VaporError> {
        let mut instances = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_db_instances();

            if let Some(ref cid) = cluster_id {
                let filter = Filter::builder().name("db-cluster-id").values(cid).build();
                req = req.filters(filter);
            }

            if let Some(l) = limit {
                // See the matching comment in describe_db_clusters: AWS
                // enforces MaxRecords in [20, 100] here too.
                req = req.max_records((l - instances.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            instances.extend(output.db_instances().to_vec());

            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };

            if marker.is_none() || limit.is_some_and(|l| instances.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            instances.truncate(l.max(0) as usize);
        }

        Ok((instances, marker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://rds.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_db_clusters_happy_path_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31"),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 <DBCluster><DBClusterIdentifier>my-cluster</DBClusterIdentifier><Status>available</Status>\
                 <Engine>neptune</Engine><EngineVersion>1.3.2.0</EngineVersion><Port>8182</Port>\
                 <MultiAZ>false</MultiAZ></DBCluster>\
                 </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_db_clusters(None, None).await.unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].db_cluster_identifier(), Some("my-cluster"));
        assert_eq!(clusters[0].status(), Some("available"));
        assert_eq!(clusters[0].engine(), Some("neptune"));
        assert_eq!(clusters[0].port(), Some(8182));
        assert_eq!(clusters[0].multi_az(), Some(false));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBClusters&Version=2014-10-31&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .describe_db_clusters(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(clusters.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_stops_at_limit_below_min_max_records_clamp() {
        // `limit` of 2 is below the [20,100] MaxRecords floor, so the request
        // still asks for 20 records; the client truncates locally and keeps
        // AWS's marker even though it points past the whole fetched page.
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
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_db_clusters(Some(2), None).await.unwrap();

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
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_db_clusters(Some(100), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_clusters_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31"),
            xml_error_response("DBClusterNotFoundFault", "no such cluster"),
        )]);
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_db_clusters(None, None).await.unwrap_err();

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
    async fn describe_db_instances_happy_path_with_cluster_id_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBInstances&Version=2014-10-31&Filters.Filter.1.Name=db-cluster-id&\
                 Filters.Filter.1.Values.Value.1=my-cluster",
            ),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 <DBInstance><DBInstanceIdentifier>my-instance</DBInstanceIdentifier>\
                 <DBInstanceClass>db.r5.large</DBInstanceClass><Engine>neptune</Engine>\
                 <DBInstanceStatus>available</DBInstanceStatus><AvailabilityZone>us-east-1a</AvailabilityZone>\
                 <DBClusterIdentifier>my-cluster</DBClusterIdentifier></DBInstance>\
                 </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client
            .describe_db_instances(Some("my-cluster".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].db_instance_identifier(), Some("my-instance"));
        assert_eq!(instances[0].db_instance_class(), Some("db.r5.large"));
        assert_eq!(instances[0].db_instance_status(), Some("available"));
        assert_eq!(instances[0].availability_zone(), Some("us-east-1a"));
        assert_eq!(instances[0].db_cluster_identifier(), Some("my-cluster"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_happy_path_without_filter() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31"),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client
            .describe_db_instances(None, None, None)
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
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client
            .describe_db_instances(None, Some(2), None)
            .await
            .unwrap();

        assert_eq!(instances.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBInstances&Version=2014-10-31&Marker=cursor-b",
            ),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 </DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let (instances, marker) = client
            .describe_db_instances(None, None, Some("cursor-b".to_string()))
            .await
            .unwrap();

        assert_eq!(instances.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_db_instances_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBInstances&Version=2014-10-31"),
            xml_error_response("DBInstanceNotFound", "no such instance"),
        )]);
        let client = NeptuneClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_db_instances(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("DBInstanceNotFound".to_string()));
                assert_eq!(message, "no such instance");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
