use aws_config::SdkConfig;
use aws_sdk_elasticache::types::{CacheCluster, CacheSubnetGroup, ReplicationGroup, Tag};
use futures::future::join_all;

use crate::error::VaporError;

pub struct ElastiCacheClient {
    inner: aws_sdk_elasticache::Client,
}

impl ElastiCacheClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_elasticache::Client::new(config),
        }
    }

    /// Lists cache clusters, optionally filtered by `cluster_id`, capped at
    /// `limit` results (default unlimited) and resumed from `next_token`.
    /// `DescribeCacheClusters` has a `max_records` request field but AWS
    /// enforces **Constraints: Minimum 20, maximum 100** on it (verified
    /// against pinned `aws-sdk-elasticache` 1.110.0's
    /// `operation/describe_cache_clusters/builders.rs` doc comment) — a
    /// `limit` below 20 can't be requested exactly, so the per-request value
    /// is clamped and the accumulated `Vec` truncated client-side after the
    /// loop (neptune.rs `describe_db_clusters` pattern); when that happens
    /// the returned marker points past the whole fetched page, not just the
    /// truncated prefix returned, so resuming skips whatever was truncated
    /// off (same caveat class as `cost_explorer.rs::get_cost_and_usage`).
    pub async fn describe_cache_clusters(
        &self,
        cluster_id: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<(CacheCluster, Vec<Tag>)>, Option<String>), VaporError> {
        let mut clusters: Vec<CacheCluster> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self
                .inner
                .describe_cache_clusters()
                .show_cache_node_info(true);
            if let Some(id) = cluster_id {
                req = req.cache_cluster_id(id);
            }
            if let Some(l) = limit {
                req = req.max_records((l - clusters.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for c in output.cache_clusters() {
                clusters.push(c.clone());
            }
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

        let tag_futs = clusters.iter().map(|c| {
            let arn = c.arn().unwrap_or("").to_string();
            async move {
                if arn.is_empty() {
                    return vec![];
                }
                self.list_tags(&arn).await.unwrap_or_default()
            }
        });
        let all_tags = join_all(tag_futs).await;

        Ok((clusters.into_iter().zip(all_tags).collect(), marker))
    }

    /// Lists replication groups, optionally filtered by
    /// `replication_group_id`, capped at `limit` results (default unlimited)
    /// and resumed from `next_token`. Same `max_records` 20-100 floor as
    /// `describe_cache_clusters` (verified against pinned
    /// `operation/describe_replication_groups/builders.rs`).
    pub async fn describe_replication_groups(
        &self,
        replication_group_id: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<ReplicationGroup>, Option<String>), VaporError> {
        let mut groups: Vec<ReplicationGroup> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_replication_groups();
            if let Some(id) = replication_group_id {
                req = req.replication_group_id(id);
            }
            if let Some(l) = limit {
                req = req.max_records((l - groups.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for g in output.replication_groups() {
                groups.push(g.clone());
            }
            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };
            if marker.is_none() || limit.is_some_and(|l| groups.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            groups.truncate(l.max(0) as usize);
        }

        Ok((groups, marker))
    }

    /// Lists cache subnet groups, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. Same `max_records` 20-100
    /// floor as `describe_cache_clusters` (verified against pinned
    /// `operation/describe_cache_subnet_groups/builders.rs`).
    pub async fn describe_cache_subnet_groups(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<CacheSubnetGroup>, Option<String>), VaporError> {
        let mut subnet_groups: Vec<CacheSubnetGroup> = Vec::new();
        let mut marker = next_token;

        loop {
            let mut req = self.inner.describe_cache_subnet_groups();
            if let Some(l) = limit {
                req = req.max_records((l - subnet_groups.len() as i32).clamp(20, 100));
            }
            if let Some(ref m) = marker {
                req = req.marker(m);
            }
            let output = req.send().await.map_err(crate::error::sdk_err)?;
            for sg in output.cache_subnet_groups() {
                subnet_groups.push(sg.clone());
            }
            marker = match output.marker() {
                Some(m) if !m.is_empty() => Some(m.to_string()),
                _ => None,
            };
            if marker.is_none() || limit.is_some_and(|l| subnet_groups.len() as i32 >= l) {
                break;
            }
        }

        if let Some(l) = limit {
            subnet_groups.truncate(l.max(0) as usize);
        }

        Ok((subnet_groups, marker))
    }

    async fn list_tags(&self, arn: &str) -> Result<Vec<Tag>, VaporError> {
        let output = self
            .inner
            .list_tags_for_resource()
            .resource_name(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.tag_list().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};

    const ENDPOINT: &str = "https://elasticache.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn describe_cache_clusters_happy_path_fetches_tags() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheClusters&Version=2015-02-02&ShowCacheNodeInfo=true",
                ),
                xml_response(
                    200,
                    "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                     <CacheCluster><CacheClusterId>my-cluster</CacheClusterId>\
                     <CacheClusterStatus>available</CacheClusterStatus><Engine>redis</Engine>\
                     <ARN>arn:aws:elasticache:us-east-1:1234:cluster:my-cluster</ARN></CacheCluster>\
                     </CacheClusters></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
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
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_cache_clusters(None, None, None).await.unwrap();

        assert_eq!(clusters.len(), 1);
        let (cluster, tags) = &clusters[0];
        assert_eq!(cluster.cache_cluster_id(), Some("my-cluster"));
        assert_eq!(cluster.cache_cluster_status(), Some("available"));
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), Some("env"));
        assert_eq!(tags[0].value(), Some("prod"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_clusters_skips_tag_fetch_when_arn_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeCacheClusters&Version=2015-02-02&ShowCacheNodeInfo=true",
            ),
            xml_response(
                200,
                "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                 <CacheCluster><CacheClusterId>no-arn-cluster</CacheClusterId></CacheCluster>\
                 </CacheClusters></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_cache_clusters(None, None, None).await.unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].0.cache_cluster_id(), Some("no-arn-cluster"));
        assert!(clusters[0].1.is_empty());
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_clusters_list_tags_error_defaults_to_empty() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheClusters&Version=2015-02-02&ShowCacheNodeInfo=true",
                ),
                xml_response(
                    200,
                    "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                     <CacheCluster><CacheClusterId>my-cluster</CacheClusterId>\
                     <ARN>arn:aws:elasticache:us-east-1:1234:cluster:my-cluster</ARN></CacheCluster>\
                     </CacheClusters></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=ListTagsForResource&Version=2015-02-02&ResourceName=arn%3Aaws%3Aelasticache%3Aus-east-1%3A1234%3Acluster%3Amy-cluster",
                ),
                xml_error_response("CacheClusterNotFoundFault", "no such cluster"),
            ),
        ]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_cache_clusters(None, None, None).await.unwrap();

        assert_eq!(clusters.len(), 1);
        assert!(clusters[0].1.is_empty());
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_clusters_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeCacheClusters&Version=2015-02-02&Marker=cursor-a&ShowCacheNodeInfo=true",
            ),
            xml_response(
                200,
                "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                 </CacheClusters></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .describe_cache_clusters(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(clusters.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_clusters_filters_by_cluster_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeCacheClusters&Version=2015-02-02&CacheClusterId=my-cluster&ShowCacheNodeInfo=true",
            ),
            xml_response(
                200,
                "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                 </CacheClusters></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client
            .describe_cache_clusters(Some("my-cluster"), None, None)
            .await
            .unwrap();

        assert_eq!(clusters.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_clusters_stops_at_limit_below_min_max_records_clamp() {
        // `limit` of 2 is below the [20,100] MaxRecords floor, so the request
        // still asks for 20 records; the client truncates locally (before the
        // tag fan-out) and keeps AWS's marker even though it points past the
        // whole fetched page.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheClusters&Version=2015-02-02&MaxRecords=20&ShowCacheNodeInfo=true",
                ),
                xml_response(
                    200,
                    "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                     <CacheCluster><CacheClusterId>a</CacheClusterId></CacheCluster>\
                     <CacheCluster><CacheClusterId>b</CacheClusterId></CacheCluster>\
                     <CacheCluster><CacheClusterId>c</CacheClusterId></CacheCluster>\
                     </CacheClusters><Marker>page2</Marker></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
                ),
            ),
        ]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_cache_clusters(None, Some(2), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].0.cache_cluster_id(), Some("a"));
        assert_eq!(clusters[1].0.cache_cluster_id(), Some("b"));
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_clusters_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheClusters&Version=2015-02-02&MaxRecords=100&ShowCacheNodeInfo=true",
                ),
                xml_response(
                    200,
                    "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                     <CacheCluster><CacheClusterId>a</CacheClusterId></CacheCluster>\
                     </CacheClusters><Marker>p2</Marker></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheClusters&Version=2015-02-02&MaxRecords=99&Marker=p2&ShowCacheNodeInfo=true",
                ),
                xml_response(
                    200,
                    "<DescribeCacheClustersResponse><DescribeCacheClustersResult><CacheClusters>\
                     <CacheCluster><CacheClusterId>b</CacheClusterId></CacheCluster>\
                     </CacheClusters></DescribeCacheClustersResult></DescribeCacheClustersResponse>",
                ),
            ),
        ]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (clusters, marker) = client.describe_cache_clusters(None, Some(100), None).await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].0.cache_cluster_id(), Some("a"));
        assert_eq!(clusters[1].0.cache_cluster_id(), Some("b"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_clusters_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeCacheClusters&Version=2015-02-02&ShowCacheNodeInfo=true",
            ),
            xml_error_response("CacheClusterNotFoundFault", "no such cluster"),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_cache_clusters(None, None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("CacheClusterNotFoundFault".to_string()));
                assert_eq!(message, "no such cluster");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_groups_happy_path_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeReplicationGroups&Version=2015-02-02"),
            xml_response(
                200,
                "<DescribeReplicationGroupsResponse><DescribeReplicationGroupsResult><ReplicationGroups>\
                 <ReplicationGroup><ReplicationGroupId>my-rg</ReplicationGroupId>\
                 <Description>test rg</Description><Status>available</Status></ReplicationGroup>\
                 </ReplicationGroups></DescribeReplicationGroupsResult></DescribeReplicationGroupsResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.describe_replication_groups(None, None, None).await.unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].replication_group_id(), Some("my-rg"));
        assert_eq!(groups[0].description(), Some("test rg"));
        assert_eq!(groups[0].status(), Some("available"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_groups_filters_by_id() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeReplicationGroups&Version=2015-02-02&ReplicationGroupId=my-rg",
            ),
            xml_response(
                200,
                "<DescribeReplicationGroupsResponse><DescribeReplicationGroupsResult><ReplicationGroups>\
                 </ReplicationGroups></DescribeReplicationGroupsResult></DescribeReplicationGroupsResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client
            .describe_replication_groups(Some("my-rg"), None, None)
            .await
            .unwrap();

        assert_eq!(groups.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_groups_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeReplicationGroups&Version=2015-02-02&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeReplicationGroupsResponse><DescribeReplicationGroupsResult><ReplicationGroups>\
                 </ReplicationGroups></DescribeReplicationGroupsResult></DescribeReplicationGroupsResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client
            .describe_replication_groups(None, None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(groups.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_groups_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeReplicationGroups&Version=2015-02-02&MaxRecords=20",
            ),
            xml_response(
                200,
                "<DescribeReplicationGroupsResponse><DescribeReplicationGroupsResult><ReplicationGroups>\
                 <ReplicationGroup><ReplicationGroupId>a</ReplicationGroupId></ReplicationGroup>\
                 <ReplicationGroup><ReplicationGroupId>b</ReplicationGroupId></ReplicationGroup>\
                 <ReplicationGroup><ReplicationGroupId>c</ReplicationGroupId></ReplicationGroup>\
                 </ReplicationGroups><Marker>page2</Marker></DescribeReplicationGroupsResult></DescribeReplicationGroupsResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.describe_replication_groups(None, Some(2), None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_groups_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeReplicationGroups&Version=2015-02-02&MaxRecords=100",
                ),
                xml_response(
                    200,
                    "<DescribeReplicationGroupsResponse><DescribeReplicationGroupsResult><ReplicationGroups>\
                     <ReplicationGroup><ReplicationGroupId>a</ReplicationGroupId></ReplicationGroup>\
                     </ReplicationGroups><Marker>p2</Marker></DescribeReplicationGroupsResult></DescribeReplicationGroupsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeReplicationGroups&Version=2015-02-02&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeReplicationGroupsResponse><DescribeReplicationGroupsResult><ReplicationGroups>\
                     <ReplicationGroup><ReplicationGroupId>b</ReplicationGroupId></ReplicationGroup>\
                     </ReplicationGroups></DescribeReplicationGroupsResult></DescribeReplicationGroupsResponse>",
                ),
            ),
        ]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (groups, marker) = client.describe_replication_groups(None, Some(100), None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_replication_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeReplicationGroups&Version=2015-02-02"),
            xml_error_response("ReplicationGroupNotFoundFault", "no such rg"),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_replication_groups(None, None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ReplicationGroupNotFoundFault".to_string()));
                assert_eq!(message, "no such rg");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_subnet_groups_happy_path_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeCacheSubnetGroups&Version=2015-02-02"),
            xml_response(
                200,
                "<DescribeCacheSubnetGroupsResponse><DescribeCacheSubnetGroupsResult><CacheSubnetGroups>\
                 <CacheSubnetGroup><CacheSubnetGroupName>my-sg</CacheSubnetGroupName>\
                 <CacheSubnetGroupDescription>test sg</CacheSubnetGroupDescription>\
                 <VpcId>vpc-123</VpcId></CacheSubnetGroup>\
                 </CacheSubnetGroups></DescribeCacheSubnetGroupsResult></DescribeCacheSubnetGroupsResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (subnet_groups, marker) = client.describe_cache_subnet_groups(None, None).await.unwrap();

        assert_eq!(subnet_groups.len(), 1);
        assert_eq!(subnet_groups[0].cache_subnet_group_name(), Some("my-sg"));
        assert_eq!(subnet_groups[0].cache_subnet_group_description(), Some("test sg"));
        assert_eq!(subnet_groups[0].vpc_id(), Some("vpc-123"));
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_subnet_groups_resumes_from_provided_marker() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeCacheSubnetGroups&Version=2015-02-02&Marker=cursor-a",
            ),
            xml_response(
                200,
                "<DescribeCacheSubnetGroupsResponse><DescribeCacheSubnetGroupsResult><CacheSubnetGroups>\
                 </CacheSubnetGroups></DescribeCacheSubnetGroupsResult></DescribeCacheSubnetGroupsResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (subnet_groups, marker) = client
            .describe_cache_subnet_groups(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(subnet_groups.len(), 0);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_subnet_groups_stops_at_limit_below_min_max_records_clamp() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=DescribeCacheSubnetGroups&Version=2015-02-02&MaxRecords=20",
            ),
            xml_response(
                200,
                "<DescribeCacheSubnetGroupsResponse><DescribeCacheSubnetGroupsResult><CacheSubnetGroups>\
                 <CacheSubnetGroup><CacheSubnetGroupName>a</CacheSubnetGroupName></CacheSubnetGroup>\
                 <CacheSubnetGroup><CacheSubnetGroupName>b</CacheSubnetGroupName></CacheSubnetGroup>\
                 <CacheSubnetGroup><CacheSubnetGroupName>c</CacheSubnetGroupName></CacheSubnetGroup>\
                 </CacheSubnetGroups><Marker>page2</Marker></DescribeCacheSubnetGroupsResult></DescribeCacheSubnetGroupsResponse>",
            ),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (subnet_groups, marker) = client.describe_cache_subnet_groups(Some(2), None).await.unwrap();

        assert_eq!(subnet_groups.len(), 2);
        assert_eq!(marker, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_subnet_groups_pages_through_until_exhausted() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheSubnetGroups&Version=2015-02-02&MaxRecords=100",
                ),
                xml_response(
                    200,
                    "<DescribeCacheSubnetGroupsResponse><DescribeCacheSubnetGroupsResult><CacheSubnetGroups>\
                     <CacheSubnetGroup><CacheSubnetGroupName>a</CacheSubnetGroupName></CacheSubnetGroup>\
                     </CacheSubnetGroups><Marker>p2</Marker></DescribeCacheSubnetGroupsResult></DescribeCacheSubnetGroupsResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=DescribeCacheSubnetGroups&Version=2015-02-02&MaxRecords=99&Marker=p2",
                ),
                xml_response(
                    200,
                    "<DescribeCacheSubnetGroupsResponse><DescribeCacheSubnetGroupsResult><CacheSubnetGroups>\
                     <CacheSubnetGroup><CacheSubnetGroupName>b</CacheSubnetGroupName></CacheSubnetGroup>\
                     </CacheSubnetGroups></DescribeCacheSubnetGroupsResult></DescribeCacheSubnetGroupsResponse>",
                ),
            ),
        ]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let (subnet_groups, marker) = client.describe_cache_subnet_groups(Some(100), None).await.unwrap();

        assert_eq!(subnet_groups.len(), 2);
        assert_eq!(marker, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_cache_subnet_groups_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=DescribeCacheSubnetGroups&Version=2015-02-02"),
            xml_error_response("CacheSubnetGroupNotFoundFault", "no such subnet group"),
        )]);
        let client = ElastiCacheClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_cache_subnet_groups(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("CacheSubnetGroupNotFoundFault".to_string()));
                assert_eq!(message, "no such subnet group");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
