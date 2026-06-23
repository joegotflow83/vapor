use aws_config::SdkConfig;
use aws_sdk_costexplorer::types::{
    DateInterval, Granularity, GroupDefinition, GroupDefinitionType, Metric,
};

use crate::error::VaporError;

pub struct CostExplorerClient {
    inner: aws_sdk_costexplorer::Client,
}

impl CostExplorerClient {
    pub fn new(config: &SdkConfig) -> Self {
        let ce_config = aws_sdk_costexplorer::config::Builder::from(config)
            .region(aws_sdk_costexplorer::config::Region::new("us-east-1"))
            .build();
        Self {
            inner: aws_sdk_costexplorer::Client::from_conf(ce_config),
        }
    }

    pub async fn get_cost_and_usage(
        &self,
        start: &str,
        end: &str,
        granularity: &str,
        group_by: Option<Vec<String>>,
    ) -> Result<Vec<aws_sdk_costexplorer::types::ResultByTime>, VaporError> {
        let gran = match granularity {
            "HOURLY" => Granularity::Hourly,
            "MONTHLY" => Granularity::Monthly,
            _ => Granularity::Daily,
        };

        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .inner
                .get_cost_and_usage()
                .time_period(
                    DateInterval::builder()
                        .start(start)
                        .end(end)
                        .build()
                        .map_err(|e| VaporError::AwsSdk(e.to_string()))?,
                )
                .granularity(gran.clone())
                .metrics("UnblendedCost");

            if let Some(ref groups) = group_by {
                for g in groups {
                    let (gtype, key) = if let Some(tag) = g.strip_prefix("TAG:") {
                        (GroupDefinitionType::Tag, tag.to_string())
                    } else {
                        (GroupDefinitionType::Dimension, g.clone())
                    };
                    req = req.group_by(
                        GroupDefinition::builder()
                            .r#type(gtype)
                            .key(key)
                            .build(),
                    );
                }
            }

            if let Some(ref token) = next_token {
                req = req.next_page_token(token);
            }

            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            items.extend(output.results_by_time().to_vec());

            match output.next_page_token() {
                Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_cost_forecast(
        &self,
        start: &str,
        end: &str,
        granularity: &str,
    ) -> Result<Vec<aws_sdk_costexplorer::types::ForecastResult>, VaporError> {
        let gran = match granularity {
            "HOURLY" => Granularity::Hourly,
            "MONTHLY" => Granularity::Monthly,
            _ => Granularity::Daily,
        };

        let output = self
            .inner
            .get_cost_forecast()
            .time_period(
                DateInterval::builder()
                    .start(start)
                    .end(end)
                    .build()
                    .map_err(|e| VaporError::AwsSdk(e.to_string()))?,
            )
            .granularity(gran)
            .metric(Metric::UnblendedCost)
            .send()
            .await
            .map_err(|e| VaporError::AwsSdk(e.to_string()))?;

        Ok(output.forecast_results_by_time().to_vec())
    }
}
