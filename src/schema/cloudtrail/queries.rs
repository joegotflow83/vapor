use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use aws_sdk_cloudtrail::primitives::DateTime;
use aws_sdk_cloudtrail::types::{LookupAttribute, LookupAttributeKey};

use crate::aws::cloudtrail::CloudTrailClient;
use crate::schema::cloudtrail::types::{CloudTrailEvent, Trail};

#[derive(Default)]
pub struct CloudTrailQuery;

#[Object]
impl CloudTrailQuery {
    async fn cloudtrail_trails(&self, ctx: &Context<'_>) -> Result<Vec<Trail>> {
        let client = ctx.data::<CloudTrailClient>()?;
        let trails = client.describe_trails().await?;

        let futures: Vec<_> = trails
            .iter()
            .map(|t| async move {
                let name = t.trail_arn().or(t.name()).unwrap_or_default();
                let status = client.get_trail_status(name).await;
                (t, status)
            })
            .collect();

        let results = join_all(futures).await;
        let mut out = Vec::new();
        for (trail, status_result) in results {
            let is_logging = match status_result {
                Ok(status) => status.is_logging().unwrap_or(false),
                Err(_) => false,
            };
            out.push(Trail::from_sdk(trail, is_logging));
        }
        Ok(out)
    }

    async fn cloudtrail_events(
        &self,
        ctx: &Context<'_>,
        start_time: String,
        end_time: String,
        event_name: Option<String>,
        username: Option<String>,
    ) -> Result<Vec<CloudTrailEvent>> {
        let client = ctx.data::<CloudTrailClient>()?;

        let start = DateTime::from_secs_f64(
            chrono::DateTime::parse_from_rfc3339(&start_time)
                .map_err(|e| async_graphql::Error::new(format!("Invalid startTime: {e}")))?
                .timestamp() as f64,
        );
        let end = DateTime::from_secs_f64(
            chrono::DateTime::parse_from_rfc3339(&end_time)
                .map_err(|e| async_graphql::Error::new(format!("Invalid endTime: {e}")))?
                .timestamp() as f64,
        );

        let mut attributes = Vec::new();
        if let Some(ref name) = event_name {
            attributes.push(
                LookupAttribute::builder()
                    .attribute_key(LookupAttributeKey::EventName)
                    .attribute_value(name)
                    .build()
                    .map_err(|e| async_graphql::Error::new(format!("Failed to build attribute: {e}")))?,
            );
        }
        if let Some(ref user) = username {
            attributes.push(
                LookupAttribute::builder()
                    .attribute_key(LookupAttributeKey::Username)
                    .attribute_value(user)
                    .build()
                    .map_err(|e| async_graphql::Error::new(format!("Failed to build attribute: {e}")))?,
            );
        }

        let events = client.lookup_events(start, end, attributes).await?;
        Ok(events.into_iter().map(CloudTrailEvent::from).collect())
    }
}
