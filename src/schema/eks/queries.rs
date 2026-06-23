use async_graphql::{Context, Object, Result};

use crate::aws::eks::EksClient;
use crate::schema::eks::types::{EksAddon, EksCluster, EksFargateProfile, EksNodegroup};

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

    /// List EKS clusters. If cluster_names is provided, describes only those clusters;
    /// otherwise lists and describes all clusters in the region.
    async fn eks_clusters(
        &self,
        ctx: &Context<'_>,
        cluster_names: Option<Vec<String>>,
    ) -> Result<Vec<EksCluster>> {
        let client = ctx.data::<EksClient>()?;
        let names = match cluster_names {
            Some(ns) => ns,
            None => client.list_clusters().await?,
        };
        let mut clusters = Vec::new();
        for name in &names {
            if let Some(c) = client.describe_cluster(name).await? {
                clusters.push(EksCluster::from(c));
            }
        }
        Ok(clusters)
    }

    /// List nodegroups for a cluster. If nodegroup_names is provided, describes only those;
    /// otherwise lists and describes all nodegroups for the cluster.
    async fn eks_nodegroups(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        nodegroup_names: Option<Vec<String>>,
    ) -> Result<Vec<EksNodegroup>> {
        let client = ctx.data::<EksClient>()?;
        let names = match nodegroup_names {
            Some(ns) => ns,
            None => client.list_nodegroups(&cluster).await?,
        };
        let mut nodegroups = Vec::new();
        for name in &names {
            if let Some(ng) = client.describe_nodegroup(&cluster, name).await? {
                nodegroups.push(EksNodegroup::from(ng));
            }
        }
        Ok(nodegroups)
    }

    /// List all Fargate profiles for a cluster.
    async fn eks_fargate_profiles(
        &self,
        ctx: &Context<'_>,
        cluster: String,
    ) -> Result<Vec<EksFargateProfile>> {
        let client = ctx.data::<EksClient>()?;
        let names = client.list_fargate_profiles(&cluster).await?;
        let mut profiles = Vec::new();
        for name in &names {
            if let Some(fp) = client.describe_fargate_profile(&cluster, name).await? {
                profiles.push(EksFargateProfile::from(fp));
            }
        }
        Ok(profiles)
    }

    /// List all add-ons for a cluster.
    async fn eks_addons(
        &self,
        ctx: &Context<'_>,
        cluster: String,
    ) -> Result<Vec<EksAddon>> {
        let client = ctx.data::<EksClient>()?;
        let names = client.list_addons(&cluster).await?;
        let mut addons = Vec::new();
        for name in &names {
            if let Some(addon) = client.describe_addon(&cluster, name).await? {
                addons.push(EksAddon::from(addon));
            }
        }
        Ok(addons)
    }
}
