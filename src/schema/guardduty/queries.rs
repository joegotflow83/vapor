use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use aws_sdk_guardduty::types::{Condition, FindingCriteria};

use crate::aws::guardduty::GuardDutyClient;
use crate::schema::guardduty::types::{Detector, Finding};

#[derive(Default)]
pub struct GuardDutyQuery;

#[Object]
impl GuardDutyQuery {
    async fn guardduty_detectors(&self, ctx: &Context<'_>) -> Result<Vec<Detector>> {
        let client = ctx.data::<GuardDutyClient>()?;
        let ids = client.list_detectors().await?;

        let futures: Vec<_> = ids
            .iter()
            .map(|id| async move {
                let output = client.get_detector(id).await;
                (id.clone(), output)
            })
            .collect();

        let results = join_all(futures).await;
        let mut detectors = Vec::new();
        for (id, result) in results {
            match result {
                Ok(output) => detectors.push(Detector::from_output(id, output)),
                Err(e) => {
                    return Err(async_graphql::Error::new(format!(
                        "Failed to get detector {id}: {e}"
                    )));
                }
            }
        }
        Ok(detectors)
    }

    async fn guardduty_findings(
        &self,
        ctx: &Context<'_>,
        detector_id: String,
        min_severity: Option<f64>,
        finding_type: Option<String>,
        archived: Option<bool>,
    ) -> Result<Vec<Finding>> {
        let client = ctx.data::<GuardDutyClient>()?;

        let mut criterion = std::collections::HashMap::new();

        if let Some(sev) = min_severity {
            criterion.insert(
                "severity".to_string(),
                Condition::builder().greater_than_or_equal(sev as i64).build(),
            );
        }

        if let Some(ref ft) = finding_type {
            criterion.insert(
                "type".to_string(),
                Condition::builder().equals(ft.clone()).build(),
            );
        }

        if let Some(arch) = archived {
            criterion.insert(
                "service.archived".to_string(),
                Condition::builder()
                    .equals(if arch { "true" } else { "false" })
                    .build(),
            );
        }

        let criteria = if criterion.is_empty() {
            None
        } else {
            Some(FindingCriteria::builder().set_criterion(Some(criterion)).build())
        };

        let finding_ids = client.list_findings(&detector_id, criteria).await?;

        if finding_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Batch in chunks of 50
        let mut all_findings = Vec::new();
        for chunk in finding_ids.chunks(50) {
            let findings = client
                .get_findings(&detector_id, chunk.to_vec())
                .await?;
            all_findings.extend(findings.into_iter().map(Finding::from));
        }

        Ok(all_findings)
    }
}
