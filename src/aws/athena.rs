use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct AthenaClient {
    inner: aws_sdk_athena::Client,
}

impl AthenaClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_athena::Client::new(config),
        }
    }

    /// Lists Athena workgroups, optionally capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListWorkGroupsInput` has a
    /// settable `max_results` (verified against pinned `aws-sdk-athena`
    /// 1.110.0's `operation/list_work_groups/_list_work_groups_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself
    /// (kinesis/mq hand-rolled loop pattern), no client-side truncation.
    pub async fn list_work_groups(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_athena::types::WorkGroupSummary>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_work_groups();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.work_groups.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn get_work_group(
        &self,
        name: &str,
    ) -> Result<aws_sdk_athena::types::WorkGroup, VaporError> {
        let output = self
            .inner
            .get_work_group()
            .work_group(name)
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        output
            .work_group()
            .cloned()
            .ok_or_else(|| VaporError::AwsSdk { code: None, message: "WorkGroup not found".to_string() })
    }

    /// Lists Athena named query IDs, optionally filtered by workgroup and
    /// capped at `limit` results (default unlimited) and resumed from
    /// `next_token`. `ListNamedQueriesInput` has a settable `max_results`
    /// (verified against pinned `aws-sdk-athena` 1.110.0's
    /// `operation/list_named_queries/_list_named_queries_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself.
    pub async fn list_named_queries(
        &self,
        workgroup: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_named_queries();
            if let Some(wg) = workgroup {
                req = req.work_group(wg);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.named_query_ids.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn batch_get_named_query(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<aws_sdk_athena::types::NamedQuery>, VaporError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let output = self
            .inner
            .batch_get_named_query()
            .set_named_query_ids(Some(ids))
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.named_queries().to_vec())
    }

    /// Lists Athena query execution IDs, optionally filtered by workgroup and
    /// capped at `limit` results (default unlimited) and resumed from
    /// `next_token`. `ListQueryExecutionsInput` has a settable `max_results`
    /// (verified against pinned `aws-sdk-athena` 1.110.0's
    /// `operation/list_query_executions/_list_query_executions_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself.
    pub async fn list_query_executions(
        &self,
        workgroup: Option<&str>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), VaporError> {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_query_executions();
            if let Some(wg) = workgroup {
                req = req.work_group(wg);
            }
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.query_execution_ids.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    pub async fn batch_get_query_execution(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<aws_sdk_athena::types::QueryExecution>, VaporError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let output = self
            .inner
            .batch_get_query_execution()
            .set_query_execution_ids(Some(ids))
            .send()
            .await
            .map_err(crate::error::sdk_err)?;
        Ok(output.query_executions().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    const ENDPOINT: &str = "https://athena.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_work_groups_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(
                200,
                r#"{"WorkGroups":[{"Name":"wg1","State":"ENABLED"},{"Name":"wg2","State":"DISABLED"}]}"#,
            ),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client.list_work_groups(None, None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name(), Some("wg1"));
        assert_eq!(groups[1].name(), Some("wg2"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_work_groups_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"WorkGroups":[{"Name":"wg3"}]}"#),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client
            .list_work_groups(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_work_groups_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":2}"#),
            json_response(
                200,
                r#"{"WorkGroups":[{"Name":"wg-a"},{"Name":"wg-b"}],"NextToken":"page2"}"#,
            ),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client.list_work_groups(Some(2), None).await.unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(token, Some("page2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_work_groups_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"WorkGroups":[{"Name":"wg-a"},{"Name":"wg-b"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"WorkGroups":[{"Name":"wg-c"}]}"#),
            ),
        ]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (groups, token) = client.list_work_groups(Some(10), None).await.unwrap();

        assert_eq!(groups.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_work_groups_propagates_errors() {
        // `InvalidRequestException` (not a throttling exception — see
        // apigateway.rs's precedent for why that would consume a second
        // replay event via the SDK's default retry strategy).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_error_response("InvalidRequestException", "invalid request"),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let err = client.list_work_groups(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_work_group_returns_work_group_when_found() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"WorkGroup":"wg1"}"#),
            json_response(200, r#"{"WorkGroup":{"Name":"wg1","State":"ENABLED"}}"#),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let wg = client.get_work_group("wg1").await.unwrap();

        assert_eq!(wg.name(), "wg1");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_work_group_errors_when_work_group_field_missing() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"WorkGroup":"wg-missing"}"#),
            json_response(200, r#"{}"#),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let err = client.get_work_group("wg-missing").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, None);
                assert_eq!(message, "WorkGroup not found");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn get_work_group_propagates_sdk_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"WorkGroup":"wg1"}"#),
            json_error_response("InvalidRequestException", "invalid request"),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let err = client.get_work_group("wg1").await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidRequestException".to_string()));
                assert_eq!(message, "invalid request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_named_queries_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"WorkGroup":"wg1"}"#),
            json_response(200, r#"{"NamedQueryIds":["q1","q2"]}"#),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_named_queries(Some("wg1"), None, None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["q1".to_string(), "q2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_named_queries_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(200, r#"{"NamedQueryIds":["q1"],"NextToken":"p2"}"#),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client.list_named_queries(None, Some(1), None).await.unwrap();

        assert_eq!(ids, vec!["q1".to_string()]);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_named_query_returns_named_queries() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"NamedQueryIds":["q1","q2"]}"#),
            json_response(
                200,
                r#"{"NamedQueries":[{"Name":"nq1","Database":"db1","QueryString":"SELECT 1"},{"Name":"nq2","Database":"db1","QueryString":"SELECT 2"}]}"#,
            ),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let queries = client
            .batch_get_named_query(vec!["q1".to_string(), "q2".to_string()])
            .await
            .unwrap();

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].name(), "nq1");
        assert_eq!(queries[1].name(), "nq2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_named_query_returns_empty_without_request_when_ids_empty() {
        let http_client = StaticReplayClient::new(vec![]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let queries = client.batch_get_named_query(vec![]).await.unwrap();

        assert!(queries.is_empty());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_query_executions_lists_all_when_no_limit() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{}"#),
            json_response(200, r#"{"QueryExecutionIds":["e1","e2"]}"#),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client.list_query_executions(None, None, None).await.unwrap();

        assert_eq!(ids, vec!["e1".to_string(), "e2".to_string()]);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_query_executions_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(200, r#"{"QueryExecutionIds":["e1"],"NextToken":"p2"}"#),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let (ids, token) = client
            .list_query_executions(None, Some(1), None)
            .await
            .unwrap();

        assert_eq!(ids, vec!["e1".to_string()]);
        assert_eq!(token, Some("p2".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_query_execution_returns_query_executions() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"QueryExecutionIds":["e1"]}"#),
            json_response(
                200,
                r#"{"QueryExecutions":[{"QueryExecutionId":"e1","Query":"SELECT 1"}]}"#,
            ),
        )]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let executions = client
            .batch_get_query_execution(vec!["e1".to_string()])
            .await
            .unwrap();

        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].query_execution_id(), Some("e1"));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn batch_get_query_execution_returns_empty_without_request_when_ids_empty() {
        let http_client = StaticReplayClient::new(vec![]);
        let client = AthenaClient::new(&sdk_config(http_client.clone()));

        let executions = client.batch_get_query_execution(vec![]).await.unwrap();

        assert!(executions.is_empty());
        http_client.relaxed_requests_match();
    }
}

