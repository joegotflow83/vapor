use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct QldbLedgerInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub state: Option<String>,
    pub creation_date_time: Option<String>,
    pub permissions_mode: Option<String>,
    pub deletion_protection: Option<bool>,
    pub kms_key_arn: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct QldbJournalExportInfo {
    pub ledger_name: String,
    pub export_id: String,
    pub export_creation_time: Option<String>,
    pub status: Option<String>,
    pub inclusive_start_time: Option<String>,
    pub exclusive_end_time: Option<String>,
    pub output_format: Option<String>,
}

pub struct QldbClient {
    inner: aws_sdk_qldb::Client,
}

impl QldbClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_qldb::Client::new(config),
        }
    }

    async fn get_ledger_details(&self, name: &str) -> Result<QldbLedgerInfo, VaporError> {
        let desc = self
            .inner
            .describe_ledger()
            .name(name)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        let arn = desc.arn().map(|s| s.to_string());

        let tags = if let Some(ref arn_str) = arn {
            let tags_output = self
                .inner
                .list_tags_for_resource()
                .resource_arn(arn_str)
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            tags_output
                .tags()
                .into_iter()
                .flat_map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_ref().map(|val| (k.clone(), val.clone())))
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            vec![]
        };

        Ok(QldbLedgerInfo {
            name: desc.name().map(|s| s.to_string()),
            arn,
            state: desc.state().map(|s| s.as_str().to_string()),
            creation_date_time: desc.creation_date_time().map(|dt| dt.to_string()),
            permissions_mode: desc.permissions_mode().map(|s| s.as_str().to_string()),
            deletion_protection: desc.deletion_protection(),
            kms_key_arn: desc
                .encryption_description()
                .map(|e| e.kms_key_arn().to_string()),
            tags,
        })
    }

    pub async fn list_ledgers(&self) -> Result<Vec<QldbLedgerInfo>, VaporError> {
        let mut names = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.inner.list_ledgers();
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for ledger in output.ledgers() {
                if let Some(name) = ledger.name() {
                    names.push(name.to_string());
                }
            }
            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        let mut results = Vec::new();
        for name in names {
            results.push(self.get_ledger_details(&name).await?);
        }
        Ok(results)
    }

    pub async fn describe_ledger(&self, name: &str) -> Result<Option<QldbLedgerInfo>, VaporError> {
        match self.get_ledger_details(name).await {
            Ok(ledger) => Ok(Some(ledger)),
            Err(_) => Ok(None),
        }
    }

    pub async fn list_journal_s3_exports(
        &self,
        ledger_name: &str,
    ) -> Result<Vec<QldbJournalExportInfo>, VaporError> {
        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .list_journal_s3_exports_for_ledger()
                .name(ledger_name);
            if let Some(ref tok) = next_token {
                req = req.next_token(tok);
            }
            let output = req
                .send()
                .await
                .map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for export in output.journal_s3_exports() {
                items.push(QldbJournalExportInfo {
                    ledger_name: export.ledger_name().to_string(),
                    export_id: export.export_id().to_string(),
                    export_creation_time: Some(export.export_creation_time().to_string()),
                    status: Some(export.status().as_str().to_string()),
                    inclusive_start_time: Some(export.inclusive_start_time().to_string()),
                    exclusive_end_time: Some(export.exclusive_end_time().to_string()),
                    output_format: export.output_format().map(|f| f.as_str().to_string()),
                });
            }
            match output.next_token() {
                Some(tok) if !tok.is_empty() => next_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
