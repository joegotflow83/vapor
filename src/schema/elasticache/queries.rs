use async_graphql::{Context, Object, Result};

use crate::aws::elasticache::ElastiCacheClient;
use crate::schema::elasticache::types::{
    ElastiCacheCluster, ElastiCacheReplicationGroup, ElastiCacheSubnetGroup,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct ElastiCacheQuery;

#[Object]
impl ElastiCacheQuery {
    /// List ElastiCache clusters. Optionally filter by clusterId.
    async fn elasticache_clusters(
        &self,
        ctx: &Context<'_>,
        cluster_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ElastiCacheCluster>> {
        let client = ctx.data::<ElastiCacheClient>()?;
        let (results, next_token) = client
            .describe_cache_clusters(cluster_id.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: results
                .iter()
                .map(|(c, tags)| ElastiCacheCluster::from_sdk(c, tags))
                .collect(),
            next_token,
        })
    }

    /// List ElastiCache replication groups. Optionally filter by replicationGroupId.
    async fn elasticache_replication_groups(
        &self,
        ctx: &Context<'_>,
        replication_group_id: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ElastiCacheReplicationGroup>> {
        let client = ctx.data::<ElastiCacheClient>()?;
        let (results, next_token) = client
            .describe_replication_groups(replication_group_id.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: results.iter().map(ElastiCacheReplicationGroup::from).collect(),
            next_token,
        })
    }

    /// List all ElastiCache subnet groups.
    async fn elasticache_subnet_groups(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ElastiCacheSubnetGroup>> {
        let client = ctx.data::<ElastiCacheClient>()?;
        let (results, next_token) = client.describe_cache_subnet_groups(limit, next_token).await?;
        Ok(Page {
            items: results.iter().map(ElastiCacheSubnetGroup::from).collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::elasticache::ElastiCacheClient;
    use crate::aws::test_util::{request, sdk_config, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::ElastiCacheQuery;

    const ENDPOINT: &str = "https://elasticache.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn elasticache_clusters_fans_out_to_tags_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheClusters&Version=2015-02-02&MaxRecords=20&ShowCacheNodeInfo=true",
                ),
                xml_response(
                    200,
                    "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                     <CacheCluster><CacheClusterId>my-cluster</CacheClusterId>\
                     <CacheClusterStatus>available</CacheClusterStatus><Engine>redis</Engine>\
                     <ARN>arn:aws:elasticache:us-east-1:1234:cluster:my-cluster</ARN></CacheCluster>\
                     </CacheClusters><Marker>page2</Marker></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=ListTagsForResource&Version=2015-02-02&ResourceName=arn%3Aaws%3Aelasticache%3Aus-east-1%3A1234%3Acluster%3Amy-cluster",
                ),
                xml_response(
                    200,
                    "<ListTagsForResourceResponse><ListTagsForResourceResult><TagList>\
                     <Tag><Key>env</Key><Value>prod</Value></Tag>\
                     </TagList></ListTagsForResourceResult></ListTagsForResourceResponse>",
                ),
            ),
        ]);
        let schema = build_query_schema(ElastiCacheQuery)
            .data(ElastiCacheClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ elasticacheClusters(limit: 1) { items { cacheClusterId cacheClusterStatus engine tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["elasticacheClusters"]["items"];
        assert_eq!(items[0]["cacheClusterId"], "my-cluster");
        assert_eq!(items[0]["cacheClusterStatus"], "available");
        assert_eq!(items[0]["engine"], "redis");
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(items[0]["tags"][0]["value"], "prod");
        assert_eq!(json["elasticacheClusters"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn elasticache_replication_groups_forwards_id_filter_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeReplicationGroups&Version=2015-02-02&ReplicationGroupId=my-rg&MaxRecords=20",
            ),
            xml_response(
                200,
                "<DescribeReplicationGroupsResponse><DescribeReplicationGroupsResult><ReplicationGroups>\
                 <ReplicationGroup><ReplicationGroupId>my-rg</ReplicationGroupId>\
                 <Description>test rg</Description><Status>available</Status></ReplicationGroup>\
                 </ReplicationGroups><Marker>page2</Marker></DescribeReplicationGroupsResult></DescribeReplicationGroupsResponse>",
            ),
        )]);
        let schema = build_query_schema(ElastiCacheQuery)
            .data(ElastiCacheClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ elasticacheReplicationGroups(replicationGroupId: "my-rg", limit: 1) { items { replicationGroupId description status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["elasticacheReplicationGroups"]["items"];
        assert_eq!(items[0]["replicationGroupId"], "my-rg");
        assert_eq!(items[0]["description"], "test rg");
        assert_eq!(items[0]["status"], "available");
        assert_eq!(json["elasticacheReplicationGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn elasticache_subnet_groups_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeCacheSubnetGroups&Version=2015-02-02&MaxRecords=20",
            ),
            xml_response(
                200,
                "<DescribeCacheSubnetGroupsResponse><DescribeCacheSubnetGroupsResult><CacheSubnetGroups>\
                 <CacheSubnetGroup><CacheSubnetGroupName>my-sg</CacheSubnetGroupName>\
                 <CacheSubnetGroupDescription>test sg</CacheSubnetGroupDescription>\
                 <VpcId>vpc-123</VpcId></CacheSubnetGroup>\
                 </CacheSubnetGroups><Marker>page2</Marker></DescribeCacheSubnetGroupsResult></DescribeCacheSubnetGroupsResponse>",
            ),
        )]);
        let schema = build_query_schema(ElastiCacheQuery)
            .data(ElastiCacheClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ elasticacheSubnetGroups(limit: 1) { items { cacheSubnetGroupName cacheSubnetGroupDescription vpcId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["elasticacheSubnetGroups"]["items"];
        assert_eq!(items[0]["cacheSubnetGroupName"], "my-sg");
        assert_eq!(items[0]["cacheSubnetGroupDescription"], "test sg");
        assert_eq!(items[0]["vpcId"], "vpc-123");
        assert_eq!(json["elasticacheSubnetGroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
