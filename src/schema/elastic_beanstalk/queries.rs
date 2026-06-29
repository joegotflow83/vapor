use async_graphql::{Context, Object, Result};

use crate::aws::elastic_beanstalk::ElasticBeanstalkClient;
use crate::schema::elastic_beanstalk::types::{
    BeanstalkApplication, BeanstalkApplicationVersion, BeanstalkEnvironment,
};

#[derive(Default)]
pub struct ElasticBeanstalkQuery;

#[Object]
impl ElasticBeanstalkQuery {
    async fn beanstalk_applications(
        &self,
        ctx: &Context<'_>,
        application_names: Option<Vec<String>>,
    ) -> Result<Vec<BeanstalkApplication>> {
        let client = ctx.data::<ElasticBeanstalkClient>()?;
        let apps = client.describe_applications(application_names).await?;
        Ok(apps.into_iter().map(BeanstalkApplication::from).collect())
    }

    async fn beanstalk_environments(
        &self,
        ctx: &Context<'_>,
        application_name: Option<String>,
        environment_names: Option<Vec<String>>,
        included_deleted_back_to: Option<String>,
    ) -> Result<Vec<BeanstalkEnvironment>> {
        let client = ctx.data::<ElasticBeanstalkClient>()?;
        let envs = client
            .describe_environments(application_name, environment_names, included_deleted_back_to)
            .await?;
        Ok(envs.into_iter().map(BeanstalkEnvironment::from).collect())
    }

    async fn beanstalk_application_versions(
        &self,
        ctx: &Context<'_>,
        application_name: Option<String>,
        version_labels: Option<Vec<String>>,
    ) -> Result<Vec<BeanstalkApplicationVersion>> {
        let client = ctx.data::<ElasticBeanstalkClient>()?;
        let versions = client
            .describe_application_versions(application_name, version_labels)
            .await?;
        Ok(versions
            .into_iter()
            .map(BeanstalkApplicationVersion::from)
            .collect())
    }
}
