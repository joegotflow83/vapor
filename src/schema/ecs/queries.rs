use async_graphql::{Context, Object, Result};

use crate::aws::ecs::EcsClient;
use crate::schema::ecs::types::{Cluster, Service, Task, TaskDefinition};

#[derive(Default)]
pub struct EcsQuery;

#[Object]
impl EcsQuery {
    /// List clusters. If cluster_arns is None, returns all clusters.
    async fn ecs_clusters(
        &self,
        ctx: &Context<'_>,
        cluster_arns: Option<Vec<String>>,
    ) -> Result<Vec<Cluster>> {
        let client = ctx.data::<EcsClient>()?;
        let results = client.describe_clusters(cluster_arns).await?;
        Ok(results.into_iter().map(Cluster::from).collect())
    }

    /// List services in a cluster. cluster must be an ARN or name.
    async fn ecs_services(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        service_arns: Option<Vec<String>>,
    ) -> Result<Vec<Service>> {
        let client = ctx.data::<EcsClient>()?;
        let results = client.describe_services(&cluster, service_arns).await?;
        Ok(results.into_iter().map(Service::from).collect())
    }

    /// List tasks in a cluster, optionally filtered by service and desired status.
    /// desired_status: "RUNNING" | "PENDING" | "STOPPED"
    async fn ecs_tasks(
        &self,
        ctx: &Context<'_>,
        cluster: String,
        service_arn: Option<String>,
        desired_status: Option<String>,
    ) -> Result<Vec<Task>> {
        let client = ctx.data::<EcsClient>()?;
        let results = client.describe_tasks(&cluster, service_arn, desired_status).await?;
        Ok(results.into_iter().map(Task::from).collect())
    }

    /// Fetch a single task definition by ARN or family:revision.
    async fn ecs_task_definition(
        &self,
        ctx: &Context<'_>,
        task_definition: String,
    ) -> Result<Option<TaskDefinition>> {
        let client = ctx.data::<EcsClient>()?;
        let result = client.describe_task_definition(&task_definition).await?;
        Ok(result.map(TaskDefinition::from))
    }

    /// List task definition ARNs, optionally filtered by family prefix and status.
    /// status: "ACTIVE" | "INACTIVE" | "DELETE_IN_PROGRESS"
    async fn ecs_task_definitions(
        &self,
        ctx: &Context<'_>,
        family_prefix: Option<String>,
        status: Option<String>,
    ) -> Result<Vec<String>> {
        let client = ctx.data::<EcsClient>()?;
        Ok(client.list_task_definitions(family_prefix, status).await?)
    }
}
