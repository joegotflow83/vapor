use async_graphql::{Context, Object, Result};

use crate::aws::comprehend::ComprehendClient;
use crate::schema::comprehend::types::{
    ComprehendDocumentClassifier, ComprehendEndpoint, ComprehendEntityRecognizer,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct ComprehendQuery;

#[Object]
impl ComprehendQuery {
    /// Lists entity recognizers, optionally capped at `limit` results and resumed via `next_token`.
    async fn comprehend_entity_recognizers(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ComprehendEntityRecognizer>> {
        let client = ctx.data::<ComprehendClient>()?;
        let (items, next_token) = client
            .list_entity_recognizers(status_filter, limit, next_token)
            .await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(ComprehendEntityRecognizer::from)
                .collect(),
            next_token,
        })
    }

    /// Lists document classifiers, optionally capped at `limit` results and resumed via `next_token`.
    async fn comprehend_document_classifiers(
        &self,
        ctx: &Context<'_>,
        status_filter: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ComprehendDocumentClassifier>> {
        let client = ctx.data::<ComprehendClient>()?;
        let (items, next_token) = client
            .list_document_classifiers(status_filter, limit, next_token)
            .await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(ComprehendDocumentClassifier::from)
                .collect(),
            next_token,
        })
    }

    /// Lists endpoints, optionally capped at `limit` results and resumed via `next_token`.
    async fn comprehend_endpoints(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<ComprehendEndpoint>> {
        let client = ctx.data::<ComprehendClient>()?;
        let (items, next_token) = client.list_endpoints(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(ComprehendEndpoint::from).collect(),
            next_token,
        })
    }
}

// All three resolvers are 1:1 passthroughs to a single already-tested
// `ComprehendClient` method each (see `src/aws/comprehend.rs`'s own test
// module for the pagination/error-mapping behavior) — only light smoke
// tests are needed here per the resolver-layer sweep's stated scope
// (codeartifact precedent: one test per resolver).
#[cfg(test)]
mod tests {
    use crate::aws::comprehend::ComprehendClient;
    use crate::aws::test_util::{json_response, request, sdk_config, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::ComprehendQuery;

    const ENDPOINT: &str = "https://comprehend.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn comprehend_entity_recognizers_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Filter":{"Status":"TRAINED"},"MaxResults":1}"#),
            json_response(
                200,
                r#"{"EntityRecognizerPropertiesList":[{"EntityRecognizerArn":"er-1","LanguageCode":"EN","Status":"TRAINED","SubmitTime":1700000000,"EndTime":1700000100,"TrainingStartTime":1700000000,"TrainingEndTime":1700000100}],"NextToken":"cursor-b"}"#,
            ),
        )]);
        let schema = build_query_schema(ComprehendQuery)
            .data(ComprehendClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ comprehendEntityRecognizers(statusFilter: "TRAINED", limit: 1) { items { entityRecognizerArn languageCode status submitTime endTime trainingStartTime trainingEndTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["comprehendEntityRecognizers"]["items"];
        assert_eq!(items[0]["entityRecognizerArn"], "er-1");
        assert_eq!(items[0]["languageCode"], "EN");
        assert_eq!(items[0]["status"], "TRAINED");
        assert_eq!(items[0]["submitTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["endTime"], "2023-11-14T22:15:00+00:00");
        assert_eq!(items[0]["trainingStartTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["trainingEndTime"], "2023-11-14T22:15:00+00:00");
        assert_eq!(json["comprehendEntityRecognizers"]["nextToken"], "cursor-b");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn comprehend_document_classifiers_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"Filter":{"Status":"TRAINED"},"MaxResults":1}"#),
            json_response(
                200,
                r#"{"DocumentClassifierPropertiesList":[{"DocumentClassifierArn":"dc-1","LanguageCode":"EN","Status":"TRAINED","Mode":"MULTI_CLASS","SubmitTime":1700000000,"EndTime":1700000100}],"NextToken":"cursor-c"}"#,
            ),
        )]);
        let schema = build_query_schema(ComprehendQuery)
            .data(ComprehendClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ comprehendDocumentClassifiers(statusFilter: "TRAINED", limit: 1) { items { documentClassifierArn languageCode status mode submitTime endTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["comprehendDocumentClassifiers"]["items"];
        assert_eq!(items[0]["documentClassifierArn"], "dc-1");
        assert_eq!(items[0]["languageCode"], "EN");
        assert_eq!(items[0]["status"], "TRAINED");
        assert_eq!(items[0]["mode"], "MULTI_CLASS");
        assert_eq!(items[0]["submitTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["endTime"], "2023-11-14T22:15:00+00:00");
        assert_eq!(json["comprehendDocumentClassifiers"]["nextToken"], "cursor-c");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn comprehend_endpoints_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"EndpointPropertiesList":[{"EndpointArn":"ep-1","ModelArn":"model-1","Status":"IN_SERVICE","CurrentInferenceUnits":2,"CreationTime":1700000000,"LastModifiedTime":1700000100}],"NextToken":"cursor-d"}"#,
            ),
        )]);
        let schema = build_query_schema(ComprehendQuery)
            .data(ComprehendClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ comprehendEndpoints(limit: 1) { items { endpointArn modelArn status currentInferenceUnits creationTime lastModifiedTime } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["comprehendEndpoints"]["items"];
        assert_eq!(items[0]["endpointArn"], "ep-1");
        assert_eq!(items[0]["modelArn"], "model-1");
        assert_eq!(items[0]["status"], "IN_SERVICE");
        assert_eq!(items[0]["currentInferenceUnits"], 2);
        assert_eq!(items[0]["creationTime"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["lastModifiedTime"], "2023-11-14T22:15:00+00:00");
        assert_eq!(json["comprehendEndpoints"]["nextToken"], "cursor-d");
        http_client.relaxed_requests_match();
    }
}
