use async_graphql::{Context, Object, Result};

use crate::aws::eks::EksClient;
use crate::schema::eks::types::{EksAddon, EksCluster, EksFargateProfile, EksNodegroup};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct EksQuery;

#[Object]
impl EksQuery {
    /// Describe a single EKS cluster by name.
    async fn eks_cluster(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<Option<EksCluster>> {
        let client = ctx.data::<EksClient>()?;
        let result = client.describe_cluster(&name).await?;
        Ok(result.map(EksCluster::from))
    }

    /// List EKS clusters. If cluster_names is provided, describes only those clusters
    /// (a non-resumable fan-out, `nextToken` always null); otherwise lists and describes
    /// one page of clusters in the region, resumable via `nextToken`. `limit` caps the
    /// number of clusters listed (default unlimited); ignored when cluster_names is provided.
    async fn eks_clusters(
        &self,
        ctx: &Context<'_>,
        cluster_names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EksCluster>> {
        let client = ctx.data::<EksClient>()?;
        let (names, token) = match cluster_names {
            Some(ns) => (ns, None),
            None => client.list_clusters(limit, next_token).await?,
        };
        let mut clusters = Vec::new();
        for name in &names {
            if let Some(c) = client.describe_cluster(name).await? {
                clusters.push(EksCluster::from(c));
            }
        }
        Ok(Page {
            items: clusters,
            next_token: token,
        })
    }

    /// List nodegroups for a cluster. If nodegroup_names is provided, describes only those
    /// (a non-resumable fan-out, `nextToken` always null); otherwise lists and describes
    /// one page of nodegroups for the cluster, resumable via `nextToken`. `limit` caps the
    /// number of nodegroups listed (default unlimited); ignored when nodegroup_names is provided.
    async fn eks_nodegroups(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        nodegroup_names: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EksNodegroup>> {
        let client = ctx.data::<EksClient>()?;
        let (names, token) = match nodegroup_names {
            Some(ns) => (ns, None),
            None => client.list_nodegroups(&cluster, limit, next_token).await?,
        };
        let mut nodegroups = Vec::new();
        for name in &names {
            if let Some(ng) = client.describe_nodegroup(&cluster, name).await? {
                nodegroups.push(EksNodegroup::from(ng));
            }
        }
        Ok(Page {
            items: nodegroups,
            next_token: token,
        })
    }

    /// List one page of Fargate profiles for a cluster. `limit` caps the number of
    /// profiles listed (default unlimited); resumable via `nextToken`.
    async fn eks_fargate_profiles(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EksFargateProfile>> {
        let client = ctx.data::<EksClient>()?;
        let (names, token) = client
            .list_fargate_profiles(&cluster, limit, next_token)
            .await?;
        let mut profiles = Vec::new();
        for name in &names {
            if let Some(fp) = client.describe_fargate_profile(&cluster, name).await? {
                profiles.push(EksFargateProfile::from(fp));
            }
        }
        Ok(Page {
            items: profiles,
            next_token: token,
        })
    }

    /// List one page of add-ons for a cluster. `limit` caps the number of add-ons
    /// listed (default unlimited); resumable via `nextToken`.
    async fn eks_addons(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EksAddon>> {
        let client = ctx.data::<EksClient>()?;
        let (names, token) = client.list_addons(&cluster, limit, next_token).await?;
        let mut addons = Vec::new();
        for name in &names {
            if let Some(addon) = client.describe_addon(&cluster, name).await? {
                addons.push(EksAddon::from(addon));
            }
        }
        Ok(Page {
            items: addons,
            next_token: token,
        })
    }
}

// `eks_cluster` and all four list resolvers are passthroughs to
// already-tested `EksClient` methods (see `src/aws/eks.rs`'s own test module
// for the pagination/error-mapping behavior) — only light smoke tests are
// needed here per the resolver-layer sweep's stated scope. `eks_clusters`/
// `eks_nodegroups` are the standout feature of this file (a dual-mode
// list-then-describe fan-out, connect/codebuild precedent): each gets two
// tests, one per mode; `eks_fargate_profiles`/`eks_addons` have no
// caller-supplied-names branch so one test each suffices.
#[cfg(test)]
mod tests {
    use crate::aws::eks::EksClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::EksQuery;

    const BASE: &str = "https://eks.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn eks_cluster_maps_detail() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/clusters/demo"), ""),
            json_response(
                200,
                r#"{"cluster":{"name":"demo","arn":"arn:aws:eks:us-east-1:111122223333:cluster/demo","status":"ACTIVE","version":"1.29"}}"#,
            ),
        )]);
        let schema = build_query_schema(EksQuery)
            .data(EksClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ eksCluster(name: "demo") { name arn status version } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["eksCluster"]["name"], "demo");
        assert_eq!(
            json["eksCluster"]["arn"],
            "arn:aws:eks:us-east-1:111122223333:cluster/demo"
        );
        assert_eq!(json["eksCluster"]["status"], "ACTIVE");
        assert_eq!(json["eksCluster"]["version"], "1.29");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn eks_clusters_without_names_discovers_then_describes_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/clusters?maxResults=1"), ""),
                json_response(200, r#"{"clusters":["c1"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/clusters/c1"), ""),
                json_response(200, r#"{"cluster":{"name":"c1","status":"ACTIVE"}}"#),
            ),
        ]);
        let schema = build_query_schema(EksQuery)
            .data(EksClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ eksClusters(limit: 1) { items { name status } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["eksClusters"]["items"];
        assert_eq!(items[0]["name"], "c1");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(json["eksClusters"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn eks_clusters_with_names_bypasses_discovery_and_never_resumes() {
        // Only one queued event: if the resolver didn't skip `list_clusters`
        // when `cluster_names` was given, `StaticReplayClient` would fail
        // with "no more test data available".
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/clusters/c1"), ""),
            json_response(200, r#"{"cluster":{"name":"c1","status":"ACTIVE"}}"#),
        )]);
        let schema = build_query_schema(EksQuery)
            .data(EksClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ eksClusters(clusterNames: ["c1"]) { items { name status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["eksClusters"]["items"];
        assert_eq!(items[0]["name"], "c1");
        assert!(json["eksClusters"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn eks_nodegroups_without_names_discovers_then_describes_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/clusters/demo/node-groups?maxResults=1"), ""),
                json_response(200, r#"{"nodegroups":["ng-1"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/clusters/demo/node-groups/ng-1"), ""),
                json_response(
                    200,
                    r#"{"nodegroup":{"nodegroupName":"ng-1","clusterName":"demo","status":"ACTIVE"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EksQuery)
            .data(EksClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ eksNodegroups(cluster: "demo", limit: 1) { items { name clusterName status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["eksNodegroups"]["items"];
        assert_eq!(items[0]["name"], "ng-1");
        assert_eq!(items[0]["clusterName"], "demo");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(json["eksNodegroups"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn eks_nodegroups_with_names_bypasses_discovery_and_never_resumes() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/clusters/demo/node-groups/ng-1"), ""),
            json_response(
                200,
                r#"{"nodegroup":{"nodegroupName":"ng-1","clusterName":"demo","status":"ACTIVE"}}"#,
            ),
        )]);
        let schema = build_query_schema(EksQuery)
            .data(EksClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ eksNodegroups(cluster: "demo", nodegroupNames: ["ng-1"]) { items { name } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["eksNodegroups"]["items"][0]["name"], "ng-1");
        assert!(json["eksNodegroups"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn eks_fargate_profiles_discovers_then_describes_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/clusters/demo/fargate-profiles?maxResults=1"), ""),
                json_response(200, r#"{"fargateProfileNames":["fp-1"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/clusters/demo/fargate-profiles/fp-1"), ""),
                json_response(
                    200,
                    r#"{"fargateProfile":{"fargateProfileName":"fp-1","clusterName":"demo","status":"ACTIVE"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EksQuery)
            .data(EksClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ eksFargateProfiles(cluster: "demo", limit: 1) { items { name clusterName status } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["eksFargateProfiles"]["items"];
        assert_eq!(items[0]["name"], "fp-1");
        assert_eq!(items[0]["clusterName"], "demo");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(json["eksFargateProfiles"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn eks_addons_discovers_then_describes_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/clusters/demo/addons?maxResults=1"), ""),
                json_response(200, r#"{"addons":["vpc-cni"],"nextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/clusters/demo/addons/vpc-cni"), ""),
                json_response(
                    200,
                    r#"{"addon":{"addonName":"vpc-cni","clusterName":"demo","status":"ACTIVE","addonVersion":"v1.18.0"}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EksQuery)
            .data(EksClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ eksAddons(cluster: "demo", limit: 1) { items { name clusterName status addonVersion } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["eksAddons"]["items"];
        assert_eq!(items[0]["name"], "vpc-cni");
        assert_eq!(items[0]["clusterName"], "demo");
        assert_eq!(items[0]["status"], "ACTIVE");
        assert_eq!(items[0]["addonVersion"], "v1.18.0");
        assert_eq!(json["eksAddons"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
