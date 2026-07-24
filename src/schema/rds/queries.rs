use async_graphql::{Context, Object, Result};

use crate::aws::rds::RdsClient;
use crate::schema::pagination::Page;
use crate::schema::rds::types::{
    DbCluster, DbInstance, DbParameterGroup, DbSnapshot, DbSubnetGroup,
};

#[derive(Default)]
pub struct RdsQuery;

#[Object]
impl RdsQuery {
    /// List RDS DB instances. If `ids` is provided, describes only those instances (a
    /// non-resumable fan-out, `nextToken` always null, still capped by `limit`); otherwise
    /// lists one resumable page, resumable via `nextToken`. `limit` caps the number of
    /// instances returned (default unlimited).
    async fn db_instances(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DbInstance>> {
        let client = ctx.data::<RdsClient>()?;
        let (results, next_token) = client.describe_db_instances(ids, limit, next_token).await?;
        Ok(Page {
            items: results.into_iter().map(DbInstance::from).collect(),
            next_token,
        })
    }

    /// List RDS DB clusters. If `ids` is provided, describes only those clusters (a
    /// non-resumable fan-out, `nextToken` always null, still capped by `limit`); otherwise
    /// lists one resumable page, resumable via `nextToken`. `limit` caps the number of
    /// clusters returned (default unlimited).
    async fn db_clusters(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DbCluster>> {
        let client = ctx.data::<RdsClient>()?;
        let (results, next_token) = client.describe_db_clusters(ids, limit, next_token).await?;
        Ok(Page {
            items: results.into_iter().map(DbCluster::from).collect(),
            next_token,
        })
    }

    /// List RDS DB snapshots. `limit` caps the number of snapshots returned (default
    /// unlimited) and resumed from `next_token`.
    async fn db_snapshots(
        &self,
        ctx: &Context<'_>,
        db_instance_id: Option<String>,
        snapshot_type: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DbSnapshot>> {
        let client = ctx.data::<RdsClient>()?;
        let (results, next_token) = client
            .describe_db_snapshots(db_instance_id, snapshot_type, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(DbSnapshot::from).collect(),
            next_token,
        })
    }

    /// List RDS DB parameter groups. `limit` caps the number returned (default unlimited)
    /// and resumed from `next_token`.
    async fn rds_parameter_groups(
        &self,
        ctx: &Context<'_>,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DbParameterGroup>> {
        let client = ctx.data::<RdsClient>()?;
        let (results, next_token) = client
            .describe_db_parameter_groups(name, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(DbParameterGroup::from).collect(),
            next_token,
        })
    }

    /// List RDS DB subnet groups. `limit` caps the number returned (default unlimited) and
    /// resumed from `next_token`.
    async fn rds_subnet_groups(
        &self,
        ctx: &Context<'_>,
        name: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<DbSubnetGroup>> {
        let client = ctx.data::<RdsClient>()?;
        let (results, next_token) = client
            .describe_db_subnet_groups(name, limit, next_token)
            .await?;
        Ok(Page {
            items: results.into_iter().map(DbSubnetGroup::from).collect(),
            next_token,
        })
    }
}

// All 5 resolvers are 1:1 passthroughs to a single already-tested
// `RdsClient` method each (see `src/aws/rds.rs`'s own test module for the
// pagination/ids-fan-out/error-mapping behavior) — only light smoke tests
// are needed here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::rds::RdsClient;
    use crate::aws::test_util::{
        request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::RdsQuery;

    const ENDPOINT: &str = "https://rds.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn db_instances_maps_items_and_next_token() {
        // limit=1 clamps MaxRecords to 20 (rds.rs's [20,100] clamp), but the
        // loop still stops after 1 item since 1 >= limit.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeDBInstances&Version=2014-10-31&MaxRecords=20",
            ),
            xml_response(
                200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances>\
                 <DBInstance><DBInstanceIdentifier>my-instance</DBInstanceIdentifier>\
                 <Engine>postgres</Engine><DBInstanceStatus>available</DBInstanceStatus>\
                 </DBInstance></DBInstances><Marker>page2</Marker></DescribeDBInstancesResult>\
                 </DescribeDBInstancesResponse>",
            ),
        )]);
        let schema = build_query_schema(RdsQuery)
            .data(RdsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ dbInstances(limit: 1) { items { id engine status } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["dbInstances"]["items"];
        assert_eq!(items[0]["id"], "my-instance");
        assert_eq!(items[0]["engine"], "postgres");
        assert_eq!(items[0]["status"], "available");
        assert_eq!(json["dbInstances"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn db_clusters_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeDBClusters&Version=2014-10-31&MaxRecords=20"),
            xml_response(
                200,
                "<DescribeDBClustersResponse><DescribeDBClustersResult><DBClusters>\
                 <DBCluster><DBClusterIdentifier>my-cluster</DBClusterIdentifier><Status>available</Status>\
                 <Engine>aurora-postgresql</Engine></DBCluster>\
                 </DBClusters><Marker>page2</Marker></DescribeDBClustersResult></DescribeDBClustersResponse>",
            ),
        )]);
        let schema = build_query_schema(RdsQuery)
            .data(RdsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute("{ dbClusters(limit: 1) { items { id engine status } nextToken } }")
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["dbClusters"]["items"];
        assert_eq!(items[0]["id"], "my-cluster");
        assert_eq!(items[0]["engine"], "aurora-postgresql");
        assert_eq!(items[0]["status"], "available");
        assert_eq!(json["dbClusters"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn db_snapshots_forwards_filters_and_maps_items() {
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
        let schema = build_query_schema(RdsQuery)
            .data(RdsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ dbSnapshots(dbInstanceId: \"my-instance\", snapshotType: \"manual\") \
                 { items { id dbInstanceId snapshotType } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["dbSnapshots"]["items"];
        assert_eq!(items[0]["id"], "snap-2");
        assert_eq!(items[0]["dbInstanceId"], "my-instance");
        assert_eq!(items[0]["snapshotType"], "manual");
        assert!(json["dbSnapshots"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn rds_parameter_groups_forwards_name_and_maps_items() {
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
        let schema = build_query_schema(RdsQuery)
            .data(RdsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ rdsParameterGroups(name: \"my-family\") \
                 { items { name family description } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["rdsParameterGroups"]["items"];
        assert_eq!(items[0]["name"], "my-family");
        assert_eq!(items[0]["family"], "postgres15");
        assert_eq!(items[0]["description"], "custom params");
        assert!(json["rdsParameterGroups"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn rds_subnet_groups_forwards_name_and_maps_items() {
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
        let schema = build_query_schema(RdsQuery)
            .data(RdsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ rdsSubnetGroups(name: \"my-subnet-group\") \
                 { items { name description vpcId status } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["rdsSubnetGroups"]["items"];
        assert_eq!(items[0]["name"], "my-subnet-group");
        assert_eq!(items[0]["description"], "default vpc subnets");
        assert_eq!(items[0]["vpcId"], "vpc-123");
        assert_eq!(items[0]["status"], "Complete");
        assert!(json["rdsSubnetGroups"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }
}
