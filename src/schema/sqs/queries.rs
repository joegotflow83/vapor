use async_graphql::{Context, Object, Result};

use crate::aws::sqs::SqsClient;
use crate::schema::sqs::types::SqsQueue;

#[derive(Default)]
pub struct SqsQuery;

#[Object]
impl SqsQuery {
    /// List SQS queue URLs. Optionally filter by name prefix.
    async fn sqs_queues(
        &self,
        ctx: &Context<'_>,
        prefix: Option<String>,
    ) -> Result<Vec<String>> {
        let client = ctx.data::<SqsClient>()?;
        Ok(client.list_queues(prefix.as_deref()).await?)
    }

    /// Fetch full metadata for a single SQS queue by URL.
    async fn sqs_queue(
        &self,
        ctx: &Context<'_>,
        queue_url: String,
    ) -> Result<Option<SqsQueue>> {
        let client = ctx.data::<SqsClient>()?;
        let attrs = client.get_queue_attributes(&queue_url).await?;
        let tags = client.list_queue_tags(&queue_url).await?;
        Ok(Some(SqsQueue::from_parts(queue_url, attrs, tags)))
    }
}
