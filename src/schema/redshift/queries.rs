use async_graphql::{Context, Object, Result};

use crate::aws::redshift::RedshiftClient;
use crate::schema::pagination::Page;
use crate::schema::redshift::types::{RedshiftCluster, RedshiftSnapshot};

#[derive(Default)]
pub struct RedshiftQuery;

#[Object]
impl RedshiftQuery {
    /// Lists Redshift clusters, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn redshift_clusters(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RedshiftCluster>> {
        let client = ctx.data::<RedshiftClient>()?;
        let (clusters, token) = client.describe_clusters(limit, next_token).await?;
        Ok(Page {
            items: clusters.into_iter().map(RedshiftCluster::from).collect(),
            next_token: token,
        })
    }

    /// Lists Redshift cluster snapshots, optionally capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn redshift_snapshots(
        &self,
        ctx: &Context<'_>,
        cluster_identifier: Option<String>,
        snapshot_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RedshiftSnapshot>> {
        let client = ctx.data::<RedshiftClient>()?;
        let (snapshots, token) = client
            .describe_cluster_snapshots(cluster_identifier, snapshot_type, limit, next_token)
            .await?;
        Ok(Page {
            items: snapshots.into_iter().map(RedshiftSnapshot::from).collect(),
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::redshift::RedshiftClient;
    use crate::aws::test_util::{request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::RedshiftQuery;

    const ENDPOINT: &str = "https://redshift.us-east-1.amazonaws.com/";

    // --- redshift_clusters (bare passthrough) ---

    #[tokio::test]
    async fn redshift_clusters_maps_full_fields_with_tags() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeClusters&Version=2012-12-01"),
            xml_response(
                200,
                "<DescribeClustersResponse><DescribeClustersResult><Clusters>\
                 <Cluster><ClusterIdentifier>my-cluster</ClusterIdentifier>\
                 <NodeType>dc2.large</NodeType><ClusterStatus>available</ClusterStatus>\
                 <DBName>mydb</DBName><MasterUsername>admin</MasterUsername>\
                 <Endpoint><Address>my-cluster.abc.us-east-1.redshift.amazonaws.com</Address>\
                 <Port>5439</Port></Endpoint><NumberOfNodes>2</NumberOfNodes>\
                 <VpcId>vpc-123</VpcId><Encrypted>true</Encrypted>\
                 <PubliclyAccessible>false</PubliclyAccessible>\
                 <ClusterCreateTime>2024-01-01T00:00:00Z</ClusterCreateTime>\
                 <Tags><Tag><Key>env</Key><Value>prod</Value></Tag></Tags>\
                 </Cluster></Clusters></DescribeClustersResult></DescribeClustersResponse>",
            ),
        )]);
        let schema = build_query_schema(RedshiftQuery)
            .data(RedshiftClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ redshiftClusters { items { identifier nodeType clusterStatus dbName \
                 masterUsername endpointAddress endpointPort numberOfNodes vpcId encrypted \
                 publiclyAccessible createdAt tags { key value } } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["redshiftClusters"]["items"][0];
        assert_eq!(item["identifier"], "my-cluster");
        assert_eq!(item["nodeType"], "dc2.large");
        assert_eq!(item["clusterStatus"], "available");
        assert_eq!(item["dbName"], "mydb");
        assert_eq!(item["masterUsername"], "admin");
        assert_eq!(
            item["endpointAddress"],
            "my-cluster.abc.us-east-1.redshift.amazonaws.com"
        );
        assert_eq!(item["endpointPort"], 5439);
        assert_eq!(item["numberOfNodes"], 2);
        assert_eq!(item["vpcId"], "vpc-123");
        assert_eq!(item["encrypted"], true);
        assert_eq!(item["publiclyAccessible"], false);
        assert!(!item["createdAt"].is_null());
        assert_eq!(item["tags"][0]["key"], "env");
        assert_eq!(item["tags"][0]["value"], "prod");
        assert!(json["redshiftClusters"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    // --- redshift_snapshots (bare passthrough, filter args forwarded) ---

    #[tokio::test]
    async fn redshift_snapshots_filters_by_cluster_and_type_and_maps_items() {
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
                 <NumberOfNodes>2</NumberOfNodes><ClusterVersion>1.0</ClusterVersion>\
                 <AvailabilityZone>us-east-1a</AvailabilityZone>\
                 <TotalBackupSizeInMegaBytes>512.0</TotalBackupSizeInMegaBytes>\
                 <Encrypted>true</Encrypted><MasterUsername>admin</MasterUsername>\
                 <SnapshotCreateTime>2024-01-01T00:00:00Z</SnapshotCreateTime>\
                 <Tags><Tag><Key>env</Key><Value>prod</Value></Tag></Tags>\
                 </Snapshot></Snapshots></DescribeClusterSnapshotsResult></DescribeClusterSnapshotsResponse>",
            ),
        )]);
        let schema = build_query_schema(RedshiftQuery)
            .data(RedshiftClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ redshiftSnapshots(clusterIdentifier: "my-cluster", snapshotType: "manual") {
                 items { id clusterIdentifier snapshotType status nodeType numberOfNodes
                 clusterVersion availabilityZone totalBackupSizeInMegaBytes encrypted
                 masterUsername createdAt tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let item = &json["redshiftSnapshots"]["items"][0];
        assert_eq!(item["id"], "snap-1");
        assert_eq!(item["clusterIdentifier"], "my-cluster");
        assert_eq!(item["snapshotType"], "manual");
        assert_eq!(item["status"], "available");
        assert_eq!(item["nodeType"], "dc2.large");
        assert_eq!(item["numberOfNodes"], 2);
        assert_eq!(item["clusterVersion"], "1.0");
        assert_eq!(item["availabilityZone"], "us-east-1a");
        assert_eq!(item["totalBackupSizeInMegaBytes"], 512.0);
        assert_eq!(item["encrypted"], true);
        assert_eq!(item["masterUsername"], "admin");
        assert!(!item["createdAt"].is_null());
        assert_eq!(item["tags"][0]["key"], "env");
        assert_eq!(item["tags"][0]["value"], "prod");
        assert!(json["redshiftSnapshots"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
