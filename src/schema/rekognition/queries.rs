use async_graphql::{Context, Object, Result};

use crate::aws::rekognition::RekognitionClient;
use crate::schema::rekognition::types::{
    RekognitionCollection, RekognitionProject, RekognitionStreamProcessor,
};

#[derive(Default)]
pub struct RekognitionQuery;

#[Object]
impl RekognitionQuery {
    async fn rekognition_collections(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<RekognitionCollection>> {
        let client = ctx.data::<RekognitionClient>()?;
        let items = client.list_collections().await?;
        Ok(items.into_iter().map(RekognitionCollection::from).collect())
    }

    async fn rekognition_projects(&self, ctx: &Context<'_>) -> Result<Vec<RekognitionProject>> {
        let client = ctx.data::<RekognitionClient>()?;
        let items = client.describe_projects().await?;
        Ok(items.into_iter().map(RekognitionProject::from).collect())
    }

    async fn rekognition_stream_processors(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<RekognitionStreamProcessor>> {
        let client = ctx.data::<RekognitionClient>()?;
        let items = client.list_stream_processors().await?;
        Ok(items
            .into_iter()
            .map(RekognitionStreamProcessor::from)
            .collect())
    }
}
