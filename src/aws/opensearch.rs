use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct OpenSearchClient {
    inner: aws_sdk_opensearch::Client,
}

impl OpenSearchClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_opensearch::Client::new(config),
        }
    }

    /// List all OpenSearch domain names (single call, no pagination, max 100).
    pub async fn list_domain_names(&self) -> Result<Vec<String>, VaporError> {
        let output = self
            .inner
            .list_domain_names()
            .send()
            .await
            .map_err(crate::error::sdk_err)?;

        Ok(output
            .domain_names()
            .iter()
            .filter_map(|d| d.domain_name().map(|s| s.to_string()))
            .collect())
    }

    /// Describe OpenSearch domains in batches of 5 (hard API limit per request).
    pub async fn describe_domains(
        &self,
        domain_names: &[String],
    ) -> Result<Vec<aws_sdk_opensearch::types::DomainStatus>, VaporError> {
        let mut results = Vec::new();
        for chunk in domain_names.chunks(5) {
            let output = self
                .inner
                .describe_domains()
                .set_domain_names(Some(chunk.to_vec()))
                .send()
                .await
                .map_err(crate::error::sdk_err)?;
            results.extend(output.domain_status_list().iter().cloned());
        }
        Ok(results)
    }

    /// List tags for a domain by ARN (single call, no pagination).
    pub async fn list_tags(
        &self,
        arn: &str,
    ) -> Result<Vec<aws_sdk_opensearch::types::Tag>, VaporError> {
        let output = self
            .inner
            .list_tags()
            .arn(arn)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.tag_list().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient};

    const BASE: &str = "https://es.us-east-1.amazonaws.com";
    const ARN: &str = "arn:aws:es:us-east-1:123456789012:domain/domain-1";
    const ARN_ENCODED: &str = "arn%3Aaws%3Aes%3Aus-east-1%3A123456789012%3Adomain%2Fdomain-1";

    #[tokio::test]
    async fn list_domain_names_filters_out_entries_missing_a_name() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2021-01-01/domain"), ""),
            json_response(
                200,
                r#"{"DomainNames":[{"DomainName":"domain-1","EngineType":"OpenSearch"},{"EngineType":"Elasticsearch"}]}"#,
            ),
        )]);
        let client = OpenSearchClient::new(&sdk_config(http_client.clone()));

        let names = client.list_domain_names().await.unwrap();

        assert_eq!(names, vec!["domain-1".to_string()]);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_domain_names_propagates_errors() {
        // `ValidationException`, not a throttling-classified code (see
        // memory gotcha: those get retried and exhaust the single replay
        // event, surfacing as a DispatchFailure instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2021-01-01/domain"), ""),
            json_error_response("ValidationException", "invalid request"),
        )]);
        let client = OpenSearchClient::new(&sdk_config(http_client.clone()));

        let err = client.list_domain_names().await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_domains_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2021-01-01/opensearch/domain-info"),
                r#"{"DomainNames":["domain-1","domain-2"]}"#,
            ),
            json_response(
                200,
                r#"{"DomainStatusList":[{"DomainId":"111122223333/domain-1","DomainName":"domain-1","ARN":"arn:aws:es:us-east-1:111122223333:domain/domain-1","Created":true,"Deleted":false},{"DomainId":"111122223333/domain-2","DomainName":"domain-2","ARN":"arn:aws:es:us-east-1:111122223333:domain/domain-2"}]}"#,
            ),
        )]);
        let client = OpenSearchClient::new(&sdk_config(http_client.clone()));

        let domains = client
            .describe_domains(&["domain-1".to_string(), "domain-2".to_string()])
            .await
            .unwrap();

        assert_eq!(domains.len(), 2);
        assert_eq!(domains[0].domain_name(), "domain-1");
        assert_eq!(
            domains[0].arn(),
            "arn:aws:es:us-east-1:111122223333:domain/domain-1"
        );
        assert_eq!(domains[0].created(), Some(true));
        assert_eq!(domains[0].deleted(), Some(false));
        assert_eq!(domains[1].domain_name(), "domain-2");
        assert_eq!(domains[1].created(), None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_domains_chunks_names_over_5() {
        let first_chunk: Vec<String> = (0..5).map(|i| format!("domain-{i}")).collect();
        let second_chunk: Vec<String> = (5..8).map(|i| format!("domain-{i}")).collect();
        let mut all_names = first_chunk.clone();
        all_names.extend(second_chunk.clone());

        let first_body = format!(
            r#"{{"DomainNames":[{}]}}"#,
            first_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let second_body = format!(
            r#"{{"DomainNames":[{}]}}"#,
            second_chunk
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(",")
        );

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(&format!("{BASE}/2021-01-01/opensearch/domain-info"), first_body),
                json_response(
                    200,
                    r#"{"DomainStatusList":[{"DomainId":"1/domain-0","DomainName":"domain-0","ARN":"arn:d0"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/2021-01-01/opensearch/domain-info"), second_body),
                json_response(
                    200,
                    r#"{"DomainStatusList":[{"DomainId":"1/domain-5","DomainName":"domain-5","ARN":"arn:d5"}]}"#,
                ),
            ),
        ]);
        let client = OpenSearchClient::new(&sdk_config(http_client.clone()));

        let domains = client.describe_domains(&all_names).await.unwrap();

        assert_eq!(domains.len(), 2);
        assert_eq!(domains[0].domain_name(), "domain-0");
        assert_eq!(domains[1].domain_name(), "domain-5");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_domains_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/2021-01-01/opensearch/domain-info"),
                r#"{"DomainNames":["domain-1"]}"#,
            ),
            json_error_response("ValidationException", "invalid domain name"),
        )]);
        let client = OpenSearchClient::new(&sdk_config(http_client.clone()));

        let err = client
            .describe_domains(&["domain-1".to_string()])
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ValidationException".to_string()));
                assert_eq!(message, "invalid domain name");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_returns_tags() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2021-01-01/tags?arn={ARN_ENCODED}"), ""),
            json_response(200, r#"{"TagList":[{"Key":"env","Value":"prod"}]}"#),
        )]);
        let client = OpenSearchClient::new(&sdk_config(http_client.clone()));

        let tags = client.list_tags(ARN).await.unwrap();

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key(), "env");
        assert_eq!(tags[0].value(), "prod");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_tags_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/2021-01-01/tags?arn={ARN_ENCODED}"), ""),
            json_error_response("ResourceNotFoundException", "domain not found"),
        )]);
        let client = OpenSearchClient::new(&sdk_config(http_client.clone()));

        let err = client.list_tags(ARN).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "domain not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }
}
