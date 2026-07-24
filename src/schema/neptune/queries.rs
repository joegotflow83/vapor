use async_graphql::{Context, Object, Result};

use crate::aws::neptune::NeptuneClient;
use crate::schema::neptune::types::{NeptuneCluster, NeptuneInstance};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct NeptuneQuery;

#[Object]
impl NeptuneQuery {
    async fn neptune_clusters(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<NeptuneCluster>> {
        let client = ctx.data::<NeptuneClient>()?;
        let (clusters, next_token) = client.describe_db_clusters(limit, next_token).await?;
        Ok(Page {
            items: clusters.into_iter().map(NeptuneCluster::from).collect(),
            next_token,
        })
    }

    async fn neptune_instances(
        &self,
        ctx: &Context<'_>,
        cluster_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<NeptuneInstance>> {
        let client = ctx.data::<NeptuneClient>()?;
        let (instances, next_token) = client
            .describe_db_instances(cluster_id, limit, next_token)
            .await?;
        Ok(Page {
            items: instances.into_iter().map(NeptuneInstance::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::neptune::NeptuneClient;
    use crate::aws::test_util::{
        request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::NeptuneQuery;

    const ENDPOINT: &str = "https://rds.us-east-1.amazonaws.com/";

    // `neptune_clusters` is a bare passthrough to the already-tested
    // `NeptuneClient::describe_db_clusters` (pagination/limit/error-mapping
    // covered in `src/aws/neptune.rs`) — one light smoke test.
    #[tokio::test]
    async fn neptune_clusters_lists_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&MaxRecords=20"),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 <DBCluster><DBClusterIdentifier>my-cluster</DBClusterIdentifier>\
                 <DBClusterArn>arn:aws:rds:us-east-1:123456789012:cluster:my-cluster</DBClusterArn>\
                 <Status>available</Status><Engine>neptune</Engine><EngineVersion>1.3.2.0</EngineVersion>\
                 <Endpoint>my-cluster.cluster-abc.us-east-1.neptune.amazonaws.com</Endpoint>\
                 <ReaderEndpoint>my-cluster.cluster-ro-abc.us-east-1.neptune.amazonaws.com</ReaderEndpoint>\
                 <Port>8182</Port><MultiAZ>true</MultiAZ><StorageEncrypted>true</StorageEncrypted>\
                 <KmsKeyId>arn:aws:kms:us-east-1:123456789012:key/abc123</KmsKeyId>\
                 <DeletionProtection>true</DeletionProtection></DBCluster>\
                 </DBClusters><Marker>page2</Marker></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let schema = build_query_schema(NeptuneQuery)
            .data(NeptuneClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ neptuneClusters(limit: 1) { items { clusterIdentifier arn status engine port multiAz storageEncrypted deletionProtection } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["neptuneClusters"]["items"];
        assert_eq!(items[0]["clusterIdentifier"], "my-cluster");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:rds:us-east-1:123456789012:cluster:my-cluster"
        );
        assert_eq!(items[0]["status"], "available");
        assert_eq!(items[0]["engine"], "neptune");
        assert_eq!(items[0]["port"], 8182);
        assert_eq!(items[0]["multiAz"], true);
        assert_eq!(items[0]["storageEncrypted"], true);
        assert_eq!(items[0]["deletionProtection"], true);
        assert_eq!(json["neptuneClusters"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    // `neptune_instances` is a bare passthrough to the already-tested
    // `NeptuneClient::describe_db_instances` (filter/pagination/error-mapping
    // covered in `src/aws/neptune.rs`) — one light smoke test that also
    // proves the `cluster_id` GraphQL arg forwards into the `db-cluster-id`
    // filter.
    #[tokio::test]
    async fn neptune_instances_lists_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBInstances&Version=2014-10-31&Filters.Filter.1.Name=db-cluster-id&\
                 Filters.Filter.1.Values.Value.1=my-cluster&MaxRecords=20",
            ),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 <DBInstance><DBInstanceIdentifier>my-instance</DBInstanceIdentifier>\
                 <DBInstanceArn>arn:aws:rds:us-east-1:123456789012:db:my-instance</DBInstanceArn>\
                 <DBInstanceClass>db.r5.large</DBInstanceClass><DBInstanceStatus>available</DBInstanceStatus>\
                 <DBClusterIdentifier>my-cluster</DBClusterIdentifier>\
                 <AvailabilityZone>us-east-1a</AvailabilityZone></DBInstance>\
                 </DBInstances><Marker>page2</Marker></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let schema = build_query_schema(NeptuneQuery)
            .data(NeptuneClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ neptuneInstances(clusterId: "my-cluster", limit: 1) { items { instanceIdentifier arn instanceClass status clusterIdentifier availabilityZone } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["neptuneInstances"]["items"];
        assert_eq!(items[0]["instanceIdentifier"], "my-instance");
        assert_eq!(
            items[0]["arn"],
            "arn:aws:rds:us-east-1:123456789012:db:my-instance"
        );
        assert_eq!(items[0]["instanceClass"], "db.r5.large");
        assert_eq!(items[0]["status"], "available");
        assert_eq!(items[0]["clusterIdentifier"], "my-cluster");
        assert_eq!(items[0]["availabilityZone"], "us-east-1a");
        assert_eq!(json["neptuneInstances"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
