use async_graphql::{Context, Object, Result};

use crate::aws::rekognition::RekognitionClient;
use crate::schema::pagination::Page;
use crate::schema::rekognition::types::{
    RekognitionCollection, RekognitionProject, RekognitionStreamProcessor,
};

#[derive(Default)]
pub struct RekognitionQuery;

#[Object]
impl RekognitionQuery {
    /// Lists Rekognition collections, capped at `limit` results (default
    /// unlimited) and resumed from `next_token`.
    async fn rekognition_collections(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RekognitionCollection>> {
        let client = ctx.data::<RekognitionClient>()?;
        let (items, next_token) = client.list_collections(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(RekognitionCollection::from).collect(),
            next_token,
        })
    }

    /// Lists Rekognition Custom Labels projects, capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn rekognition_projects(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RekognitionProject>> {
        let client = ctx.data::<RekognitionClient>()?;
        let (items, next_token) = client.describe_projects(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(RekognitionProject::from).collect(),
            next_token,
        })
    }

    /// Lists Rekognition stream processors, capped at `limit` results
    /// (default unlimited) and resumed from `next_token`.
    async fn rekognition_stream_processors(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<RekognitionStreamProcessor>> {
        let client = ctx.data::<RekognitionClient>()?;
        let (items, next_token) = client.list_stream_processors(limit, next_token).await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(RekognitionStreamProcessor::from)
                .collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::rekognition::RekognitionClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    // awsJson1.1: POST JSON to a fixed `/` path (same shape as
    // `src/aws/rekognition.rs`'s own test module). `limit: 1` is required on
    // every test (gotcha 29): both `list_collections` and
    // `list_stream_processors` fan out N+1 to a per-item `describe_*` call,
    // and each mocked list response also carries a `NextToken`, so without a
    // matching `limit` the client's hand-rolled loop would chase an unmocked
    // second page instead of stopping after item 1.
    const BASE: &str = "https://rekognition.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn rekognition_collections_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(200, r#"{"CollectionIds":["col-1"],"NextToken":"page2"}"#),
            ),
            ReplayEvent::new(
                request(BASE, r#"{"CollectionId":"col-1"}"#),
                json_response(
                    200,
                    r#"{"FaceCount":10,"FaceModelVersion":"5.0","CollectionARN":"arn:aws:rekognition:us-east-1:111111111111:collection/col-1","CreationTimestamp":1700000000}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(RekognitionQuery)
            .data(RekognitionClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ rekognitionCollections(limit: 1) { items { collectionId collectionArn \
                 creationTimestamp faceModelVersion faceCount } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["rekognitionCollections"]["items"];
        assert_eq!(items[0]["collectionId"], "col-1");
        assert_eq!(
            items[0]["collectionArn"],
            "arn:aws:rekognition:us-east-1:111111111111:collection/col-1"
        );
        assert_eq!(items[0]["creationTimestamp"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["faceModelVersion"], "5.0");
        assert_eq!(items[0]["faceCount"], 10);
        assert_eq!(json["rekognitionCollections"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn rekognition_projects_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, r#"{"MaxResults":1}"#),
            json_response(
                200,
                r#"{"ProjectDescriptions":[{"ProjectArn":"arn:aws:rekognition:us-east-1:111111111111:project/proj-1","CreationTimestamp":1700000000,"Status":"CREATED","Feature":"CUSTOM_LABELS","Datasets":[{"DatasetType":"TRAIN","DatasetArn":"arn:aws:rekognition:us-east-1:111111111111:project/proj-1/dataset/train/1","Status":"CREATE_COMPLETE","CreationTimestamp":1700000000}]}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(RekognitionQuery)
            .data(RekognitionClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ rekognitionProjects(limit: 1) { items { projectArn creationTimestamp status \
                 projectName feature datasets { datasetType datasetArn status creationTimestamp } \
                 } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["rekognitionProjects"]["items"];
        assert_eq!(
            items[0]["projectArn"],
            "arn:aws:rekognition:us-east-1:111111111111:project/proj-1"
        );
        assert_eq!(items[0]["creationTimestamp"], "2023-11-14T22:13:20+00:00");
        assert_eq!(items[0]["status"], "CREATED");
        assert!(items[0]["projectName"].is_null());
        assert_eq!(items[0]["feature"], "CUSTOM_LABELS");
        assert_eq!(items[0]["datasets"][0]["datasetType"], "TRAIN");
        assert_eq!(
            items[0]["datasets"][0]["datasetArn"],
            "arn:aws:rekognition:us-east-1:111111111111:project/proj-1/dataset/train/1"
        );
        assert_eq!(items[0]["datasets"][0]["status"], "CREATE_COMPLETE");
        assert_eq!(
            items[0]["datasets"][0]["creationTimestamp"],
            "2023-11-14T22:13:20+00:00"
        );
        assert_eq!(json["rekognitionProjects"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn rekognition_stream_processors_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(BASE, r#"{"MaxResults":1}"#),
                json_response(
                    200,
                    r#"{"StreamProcessors":[{"Name":"sp-1","Status":"RUNNING"}],"NextToken":"page2"}"#,
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
        let schema = build_query_schema(RekognitionQuery)
            .data(RekognitionClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                "{ rekognitionStreamProcessors(limit: 1) { items { name status \
                 streamProcessorArn } nextToken } }",
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["rekognitionStreamProcessors"]["items"];
        assert_eq!(items[0]["name"], "sp-1");
        assert_eq!(items[0]["status"], "RUNNING");
        assert_eq!(
            items[0]["streamProcessorArn"],
            "arn:aws:rekognition:us-east-1:111111111111:streamprocessor/sp-1"
        );
        assert_eq!(json["rekognitionStreamProcessors"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
