use async_graphql::{Context, Object, Result};

use crate::aws::documentdb::DocumentDbClient;
use crate::schema::documentdb::types::{DocDbCluster, DocDbInstance};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct DocumentDbQuery;

#[Object]
impl DocumentDbQuery {
    async fn docdb_clusters(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DocDbCluster>> {
        let client = ctx.data::<DocumentDbClient>()?;
        let (clusters, next_token) = client.describe_db_clusters(limit, next_token).await?;
        Ok(Page {
            items: clusters.into_iter().map(DocDbCluster::from).collect(),
            next_token,
        })
    }

    async fn docdb_instances(
        &self,
        ctx: &Context<'_>,
        cluster_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DocDbInstance>> {
        let client = ctx.data::<DocumentDbClient>()?;
        let (instances, next_token) = client
            .describe_db_instances(cluster_id, limit, next_token)
            .await?;
        Ok(Page {
            items: instances.into_iter().map(DocDbInstance::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::documentdb::DocumentDbClient;
    use crate::aws::test_util::{request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::DocumentDbQuery;

    const ENDPOINT: &str = "https://rds.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn docdb_clusters_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31"),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 <DBCluster><DBClusterIdentifier>my-cluster</DBClusterIdentifier><Status>available</Status>\
                 <Engine>docdb</Engine><Port>27017</Port><MultiAZ>true</MultiAZ></DBCluster>\
                 </DBClusters></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let schema = build_query_schema(DocumentDbQuery)
            .data(DocumentDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ docdbClusters { items { clusterIdentifier status engine port multiAz } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["docdbClusters"]["items"];
        assert_eq!(items[0]["clusterIdentifier"], "my-cluster");
        assert_eq!(items[0]["status"], "available");
        assert_eq!(items[0]["engine"], "docdb");
        assert_eq!(items[0]["port"], 27017);
        assert_eq!(items[0]["multiAz"], true);
        assert!(json["docdbClusters"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn docdb_instances_forwards_cluster_id_filter_and_maps_items() {
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
                 <DBInstanceClass>db.r5.large</DBInstanceClass><DBInstanceStatus>available</DBInstanceStatus>\
                 <AvailabilityZone>us-east-1a</AvailabilityZone><DBClusterIdentifier>my-cluster</DBClusterIdentifier>\
                 </DBInstance></DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>",
            ),
        )]);
        let schema = build_query_schema(DocumentDbQuery)
            .data(DocumentDbClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ docdbInstances(clusterId: "my-cluster") { items { instanceIdentifier instanceClass status clusterIdentifier availabilityZone } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["docdbInstances"]["items"];
        assert_eq!(items[0]["instanceIdentifier"], "my-instance");
        assert_eq!(items[0]["instanceClass"], "db.r5.large");
        assert_eq!(items[0]["status"], "available");
        assert_eq!(items[0]["clusterIdentifier"], "my-cluster");
        assert_eq!(items[0]["availabilityZone"], "us-east-1a");
        assert!(json["docdbInstances"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
