use aws_config::SdkConfig;
use aws_smithy_types::DateTime as SmithyDateTime;

use crate::error::VaporError;

#[derive(Debug)]
pub struct RekognitionCollectionInfo {
    pub collection_id: Option<String>,
    pub collection_arn: Option<String>,
    pub creation_timestamp: Option<SmithyDateTime>,
    pub face_model_version: Option<String>,
    pub face_count: Option<i64>,
}

#[derive(Debug)]
pub struct RekognitionStreamProcessorInfo {
    pub name: Option<String>,
    pub status: Option<String>,
    pub stream_processor_arn: Option<String>,
}

pub struct RekognitionClient {
    inner: aws_sdk_rekognition::Client,
}

impl RekognitionClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_rekognition::Client::new(config),
        }
    }

    /// Lists Rekognition collections, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`. `ListCollectionsInput` has a
    /// settable `max_results` (verified against pinned `aws-sdk-rekognition`
    /// 1.106.0's `operation/list_collections/_list_collections_input.rs`), so
    /// `limit` is capped to the remaining budget on the request itself
    /// (kinesis/mq hand-rolled-loop pattern). Each returned ID is then
    /// described (N+1) for face count, model version, and ARN, none of which
    /// `ListCollections` itself returns.
    pub async fn list_collections(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<RekognitionCollectionInfo>, Option<String>), VaporError> {
        let mut ids = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_collections();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - ids.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            ids.extend(output.collection_ids.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if ids.len() as i32 >= l => break,
                _ => continue,
            }
        }

        // N+1: describe each collection for face count, model version, and ARN
        let mut items = Vec::with_capacity(ids.len());
        for collection_id in ids {
            let desc = self
                .inner
                .describe_collection()
                .collection_id(&collection_id)
                .send()
                .await
                .map_err(crate::error::sdk_err)?;

            items.push(RekognitionCollectionInfo {
                collection_id: Some(collection_id),
                collection_arn: desc.collection_arn().map(|s| s.to_string()),
                creation_timestamp: desc.creation_timestamp().cloned(),
                face_model_version: desc.face_model_version().map(|s| s.to_string()),
                face_count: desc.face_count(),
            });
        }

        Ok((items, token))
    }

    /// Lists Rekognition Custom Labels projects, capped at `limit` results
    /// (default unlimited) and resumed from `next_token`. `DescribeProjectsInput`
    /// has a settable `max_results` (verified against pinned
    /// `aws-sdk-rekognition` 1.106.0's
    /// `operation/describe_projects/_describe_projects_input.rs`), so `limit`
    /// is capped to the remaining budget on the request itself. Returns the
    /// raw SDK `ProjectDescription` directly — it's already the fully
    /// described shape (no separate fan-out call needed, unlike collections
    /// and stream processors).
    pub async fn describe_projects(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<aws_sdk_rekognition::types::ProjectDescription>, Option<String>), VaporError>
    {
        let mut items = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.describe_projects();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - items.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            items.extend(output.project_descriptions.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if items.len() as i32 >= l => break,
                _ => continue,
            }
        }

        Ok((items, token))
    }

    /// Lists Rekognition stream processors, capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    /// `ListStreamProcessorsInput` has a settable `max_results` (verified
    /// against pinned `aws-sdk-rekognition` 1.106.0's
    /// `operation/list_stream_processors/_list_stream_processors_input.rs`),
    /// so `limit` is capped to the remaining budget on the request itself.
    /// Each summary is then described (N+1) for its ARN, which
    /// `ListStreamProcessors` doesn't return.
    pub async fn list_stream_processors(
        &self,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(Vec<RekognitionStreamProcessorInfo>, Option<String>), VaporError> {
        let mut summaries = Vec::new();
        let mut token = next_token;

        loop {
            let mut req = self.inner.list_stream_processors();
            if let Some(ref t) = token {
                req = req.next_token(t);
            }
            if let Some(l) = limit {
                req = req.max_results(l - summaries.len() as i32);
            }

            let output = req.send().await.map_err(crate::error::sdk_err)?;
            summaries.extend(output.stream_processors.unwrap_or_default());
            token = output.next_token;

            match (&token, limit) {
                (None, _) => break,
                (_, Some(l)) if summaries.len() as i32 >= l => break,
                _ => continue,
            }
        }

        let mut items = Vec::with_capacity(summaries.len());
        for sp in summaries {
            let name = sp.name;
            // N+1: describe to get the ARN (not available in list response)
            let stream_processor_arn = if let Some(ref n) = name {
                self.inner
                    .describe_stream_processor()
                    .name(n)
                    .send()
                    .await
                    .ok()
                    .and_then(|d| d.stream_processor_arn().map(|s| s.to_string()))
            } else {
                None
            };

            items.push(RekognitionStreamProcessorInfo {
                name,
                status: sp.status.map(|s| s.as_str().to_string()),
                stream_processor_arn,
            });
        }

        Ok((items, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::test_util::{
        json_error_response, json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };

    // awsJson1.1: POST JSON to a fixed `/` path, differentiated only by the
    // `x-amz-target` header (which `test_util::request` doesn't compare) —
    // same shape as `redshift_serverless.rs`. Crate name (`aws-sdk-
    // rekognition`) matches the endpoint hostname (`rekognition.*`,
    // verified against pinned `aws-sdk-rekognition` 1.106.0's `config/
    // endpoint.rs`). Request bodies use PascalCase keys (`NextToken`,
    // `MaxResults`, `CollectionId`, `Name`) per each op's `ser_*_input_input`
    // — response bodies mostly match, except `DescribeCollectionOutput`'s
    // `CollectionARN` (all-caps ARN, memory gotcha 19). No op's response
    // type has a `serde_util::*_correct_errors` fn (grepped the pinned
    // crate's `protocol_serde/*.rs`), so every optional field genuinely
    // stays `None` when omitted from a canned response. All 5 ops'
    // aws-layer pagination loops forward `limit` straight to AWS's
    // `MaxResults` with no client-side truncation (memory gotcha 13), so
    // capped-pagination tests must can exactly `limit` items.
    const BASE: &str = "https://rekognition.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn list_collections_lists_all_when_no_limit() {
        // N+1: the plain list loop discovers `col-1`, then `describe_collection`
        // fans out to fetch face count/model version/ARN not present on
        // `ListCollections`' own response.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(200, r#"{"CollectionIds":["col-1"]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"CollectionId":"col-1"}"#),
                json_response(
                    200,
                    r#"{"FaceCount":10,"FaceModelVersion":"5.0","CollectionARN":"arn:aws:rekognition:us-east-1:111111111111:collection/col-1","CreationTimestamp":1700000000,"UserCount":2}"#,
                ),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_collections(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].collection_id.as_deref(), Some("col-1"));
        assert_eq!(
            items[0].collection_arn.as_deref(),
            Some("arn:aws:rekognition:us-east-1:111111111111:collection/col-1")
        );
        assert_eq!(
            items[0].creation_timestamp,
            Some(SmithyDateTime::from_secs(1_700_000_000))
        );
        assert_eq!(items[0].face_model_version.as_deref(), Some("5.0"));
        assert_eq!(items[0].face_count, Some(10));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_collections_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"CollectionIds":[]}"#),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_collections(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_collections_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"CollectionIds":["col-1"],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"CollectionId":"col-1"}"#),
                json_response(200, r#"{"FaceCount":1}"#),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_collections(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_collections_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(200, r#"{"CollectionIds":["col-1","col-2"],"NextToken":"p2"}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"CollectionIds":["col-3"]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"CollectionId":"col-1"}"#),
                json_response(200, r#"{"FaceCount":1}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"CollectionId":"col-2"}"#),
                json_response(200, r#"{"FaceCount":2}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"CollectionId":"col-3"}"#),
                json_response(200, r#"{"FaceCount":3}"#),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_collections(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_collections_propagates_errors() {
        // `InvalidParameterException`, not a throttling-classified code
        // (memory gotcha 1).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidParameterException", "bad request"),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let err = client.list_collections(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidParameterException".to_string()));
                assert_eq!(message, "bad request");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_collections_propagates_describe_collection_fan_out_errors() {
        // Unlike `list_stream_processors`' fan-out (which swallows errors via
        // `.ok()`), `list_collections`' `describe_collection` call uses `?`
        // directly, so a fan-out failure must propagate rather than degrade
        // to a partial `RekognitionCollectionInfo`.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(200, r#"{"CollectionIds":["col-1"]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"CollectionId":"col-1"}"#),
                json_error_response("ResourceNotFoundException", "no such collection"),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let err = client.list_collections(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("ResourceNotFoundException".to_string()));
                assert_eq!(message, "no such collection");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_projects_lists_all_when_no_limit() {
        // Plain paginated list, no fan-out (unlike `list_collections`/
        // `list_stream_processors`) — `ProjectDescription` is already the
        // fully described shape.
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(
                200,
                r#"{"ProjectDescriptions":[{"ProjectArn":"arn:aws:rekognition:us-east-1:111111111111:project/proj-1/1700000000000","CreationTimestamp":1700000000,"Status":"CREATED"}]}"#,
            ),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_projects(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].project_arn(),
            Some("arn:aws:rekognition:us-east-1:111111111111:project/proj-1/1700000000000")
        );
        assert_eq!(
            items[0].creation_timestamp(),
            Some(&SmithyDateTime::from_secs(1_700_000_000))
        );
        assert_eq!(items[0].status().map(|s| s.as_str()), Some("CREATED"));
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_projects_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"ProjectDescriptions":[]}"#),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .describe_projects(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_projects_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"ProjectDescriptions":[{"ProjectArn":"proj-1"}],"NextToken":"page2-token"}"#,
            ),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_projects(Some(1), None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_projects_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"ProjectDescriptions":[{"ProjectArn":"proj-1"},{"ProjectArn":"proj-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"ProjectDescriptions":[{"ProjectArn":"proj-3"}]}"#),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.describe_projects(Some(10), None).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn describe_projects_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidPaginationTokenException", "bad token"),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let err = client.describe_projects(None, None).await.unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidPaginationTokenException".to_string()));
                assert_eq!(message, "bad token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stream_processors_lists_all_when_no_limit() {
        // N+1: `describe_stream_processor` fans out to fetch the ARN, which
        // `ListStreamProcessors` doesn't return.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(
                    200,
                    r#"{"StreamProcessors":[{"Name":"sp-1","Status":"RUNNING"}]}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"sp-1"}"#),
                json_response(
                    200,
                    r#"{"Name":"sp-1","StreamProcessorArn":"arn:aws:rekognition:us-east-1:111111111111:streamprocessor/sp-1"}"#,
                ),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_stream_processors(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_deref(), Some("sp-1"));
        assert_eq!(items[0].status.as_deref(), Some("RUNNING"));
        assert_eq!(
            items[0].stream_processor_arn.as_deref(),
            Some("arn:aws:rekognition:us-east-1:111111111111:streamprocessor/sp-1")
        );
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stream_processors_skips_describe_call_when_name_missing() {
        // The fan-out is gated on `if let Some(ref n) = name` — a summary
        // with no name must not trigger a second HTTP call (only 1
        // `ReplayEvent` is supplied; `StaticReplayClient` panics on an
        // unexpected extra request).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_response(200, r#"{"StreamProcessors":[{"Status":"STOPPED"}]}"#),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_stream_processors(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, None);
        assert_eq!(items[0].status.as_deref(), Some("STOPPED"));
        assert_eq!(items[0].stream_processor_arn, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stream_processors_resumes_from_provided_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"NextToken":"cursor-a"}"#),
            json_response(200, r#"{"StreamProcessors":[]}"#),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_stream_processors(None, Some("cursor-a".to_string()))
            .await
            .unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stream_processors_stops_at_limit_and_returns_resume_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"StreamProcessors":[{"Name":"sp-1"}],"NextToken":"page2-token"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"sp-1"}"#),
                json_response(200, r#"{"Name":"sp-1"}"#),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_stream_processors(Some(1), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(token, Some("page2-token".to_string()));
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stream_processors_pages_through_until_exhausted_when_limit_not_reached() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":10}"#),
                json_response(
                    200,
                    r#"{"StreamProcessors":[{"Name":"sp-1"},{"Name":"sp-2"}],"NextToken":"p2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"NextToken":"p2","MaxResults":8}"#),
                json_response(200, r#"{"StreamProcessors":[{"Name":"sp-3"}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"sp-1"}"#),
                json_response(200, r#"{"Name":"sp-1"}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"sp-2"}"#),
                json_response(200, r#"{"Name":"sp-2"}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"sp-3"}"#),
                json_response(200, r#"{"Name":"sp-3"}"#),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client
            .list_stream_processors(Some(10), None)
            .await
            .unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stream_processors_propagates_errors() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{}"#),
            json_error_response("InvalidPaginationTokenException", "bad token"),
        )]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let err = client
            .list_stream_processors(None, None)
            .await
            .unwrap_err();

        match err {
            VaporError::AwsSdk { code, message } => {
                assert_eq!(code, Some("InvalidPaginationTokenException".to_string()));
                assert_eq!(message, "bad token");
            }
            other => panic!("expected VaporError::AwsSdk, got {other:?}"),
        }
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn list_stream_processors_swallows_describe_fan_out_errors() {
        // Unlike `list_collections`, the `describe_stream_processor` fan-out
        // result is folded through `.ok()` (memory gotcha 10) — a failure
        // degrades `stream_processor_arn` to `None` instead of propagating,
        // and the top-level call still succeeds.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{}"#),
                json_response(200, r#"{"StreamProcessors":[{"Name":"sp-1"}]}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"Name":"sp-1"}"#),
                json_error_response("ResourceNotFoundException", "no such stream processor"),
            ),
        ]);
        let client = RekognitionClient::new(&sdk_config(http_client.clone()));

        let (items, token) = client.list_stream_processors(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_deref(), Some("sp-1"));
        assert_eq!(items[0].stream_processor_arn, None);
        assert_eq!(token, None);
        http_client.relaxed_requests_match();
    }
}
