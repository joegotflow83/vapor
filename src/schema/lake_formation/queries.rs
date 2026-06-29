use async_graphql::{Context, Object, Result};

use crate::aws::lake_formation::LakeFormationClient;
use crate::schema::lake_formation::types::{
    LakeFormationPermission, LakeFormationResource, LakeFormationSettings,
};

#[derive(Default)]
pub struct LakeFormationQuery;

#[Object]
impl LakeFormationQuery {
    async fn lake_formation_resources(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<LakeFormationResource>> {
        let client = ctx.data::<LakeFormationClient>()?;
        let resources = client.list_resources().await?;
        Ok(resources.into_iter().map(LakeFormationResource::from).collect())
    }

    async fn lake_formation_permissions(
        &self,
        ctx: &Context<'_>,
        principal: Option<String>,
        resource_type: Option<String>,
    ) -> Result<Vec<LakeFormationPermission>> {
        let client = ctx.data::<LakeFormationClient>()?;
        let permissions = client.list_permissions(principal, resource_type).await?;
        Ok(permissions
            .into_iter()
            .map(LakeFormationPermission::from)
            .collect())
    }

    async fn lake_formation_settings(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<LakeFormationSettings>> {
        let client = ctx.data::<LakeFormationClient>()?;
        let settings = client.get_data_lake_settings().await?;
        Ok(settings.map(LakeFormationSettings::from))
    }
}
