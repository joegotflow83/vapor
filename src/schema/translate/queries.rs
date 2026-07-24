use async_graphql::{Context, InputObject, Object, Result};

use crate::aws::translate::{TranslateClient, TranslateJobFilter};
use crate::schema::pagination::Page;
use crate::schema::translate::types::{
    TranslateParallelData, TranslateTerminology, TranslateTextTranslationJob,
};

#[derive(InputObject)]
pub struct TranslateJobFilterInput {
    pub job_name: Option<String>,
    pub job_status: Option<String>,
    pub submitted_before_time: Option<String>,
    pub submitted_after_time: Option<String>,
}

#[derive(Default)]
pub struct TranslateQuery;

#[Object]
impl TranslateQuery {
    /// Lists custom terminologies, optionally capped at `limit` results (default unlimited).
    async fn translate_terminologies(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TranslateTerminology>> {
        let client = ctx.data::<TranslateClient>()?;
        let (items, next_token) = client.list_terminologies(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(TranslateTerminology::from).collect(),
            next_token,
        })
    }

    /// Lists parallel data resources, optionally capped at `limit` results (default unlimited).
    async fn translate_parallel_data(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TranslateParallelData>> {
        let client = ctx.data::<TranslateClient>()?;
        let (items, next_token) = client.list_parallel_data(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(TranslateParallelData::from).collect(),
            next_token,
        })
    }

    /// Lists text translation jobs, optionally capped at `limit` results (default unlimited).
    async fn translate_text_translation_jobs(
        &self,
        ctx: &Context<'_>,
        filter: Option<TranslateJobFilterInput>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<TranslateTextTranslationJob>> {
        let client = ctx.data::<TranslateClient>()?;
        let filter = filter.map(|f| TranslateJobFilter {
            job_name: f.job_name,
            job_status: f.job_status,
            submitted_before_time: f.submitted_before_time,
            submitted_after_time: f.submitted_after_time,
        });
        let (items, next_token) = client
            .list_text_translation_jobs(filter, limit, next_token)
            .await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(TranslateTextTranslationJob::from)
                .collect(),
            next_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::aws::translate::TranslateClient;
    use crate::schema::test_util::build_query_schema;

    use super::TranslateQuery;

    const BASE: &str = "https://translate.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn translate_terminologies_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"TerminologyPropertiesList":[{"Name":"t1","Description":"desc1","Arn":"arn:t1","SourceLanguageCode":"en","TargetLanguageCodes":["es","fr"],"TermCount":5,"CreatedAt":1705314600,"LastUpdatedAt":1705315600,"Directionality":"UNI","Format":"CSV"},{"Name":"t2","Arn":"arn:t2"}]}"#,
            ),
        )]);
        let schema = build_query_schema(TranslateQuery)
            .data(TranslateClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ translateTerminologies { items { name description arn sourceLanguageCode targetLanguageCodes termCount directionality format } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["translateTerminologies"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let t1 = &items[0];
        assert_eq!(t1["name"], "t1");
        assert_eq!(t1["description"], "desc1");
        assert_eq!(t1["arn"], "arn:t1");
        assert_eq!(t1["sourceLanguageCode"], "en");
        assert_eq!(t1["targetLanguageCodes"], serde_json::json!(["es", "fr"]));
        assert_eq!(t1["termCount"], 5);
        assert_eq!(t1["directionality"], "UNI");
        assert_eq!(t1["format"], "CSV");

        let t2 = &items[1];
        assert_eq!(t2["name"], "t2");
        assert_eq!(t2["description"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn translate_parallel_data_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(BASE, "{}"),
            json_response(
                200,
                r#"{"ParallelDataPropertiesList":[{"Name":"pd1","Arn":"arn:pd1","Description":"desc1","Status":"ACTIVE","SourceLanguageCode":"en","TargetLanguageCodes":["de"],"CreatedAt":1705314600,"LastUpdatedAt":1705315600},{"Name":"pd2","Arn":"arn:pd2"}]}"#,
            ),
        )]);
        let schema = build_query_schema(TranslateQuery)
            .data(TranslateClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ translateParallelData { items { name arn description status sourceLanguageCode targetLanguageCodes } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["translateParallelData"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let pd1 = &items[0];
        assert_eq!(pd1["name"], "pd1");
        assert_eq!(pd1["arn"], "arn:pd1");
        assert_eq!(pd1["status"], "ACTIVE");
        assert_eq!(pd1["targetLanguageCodes"], serde_json::json!(["de"]));

        let pd2 = &items[1];
        assert_eq!(pd2["name"], "pd2");
        assert_eq!(pd2["status"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn translate_text_translation_jobs_forwards_filter_and_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                BASE,
                r#"{"Filter":{"JobName":"job-one","JobStatus":"COMPLETED"}}"#,
            ),
            json_response(
                200,
                r#"{"TextTranslationJobPropertiesList":[{"JobId":"j1","JobName":"job-one","JobStatus":"COMPLETED","SourceLanguageCode":"en","TargetLanguageCodes":["es"],"SubmittedTime":1705314600,"EndTime":1705315600}]}"#,
            ),
        )]);
        let schema = build_query_schema(TranslateQuery)
            .data(TranslateClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ translateTextTranslationJobs(filter: { jobName: "job-one", jobStatus: "COMPLETED" }) { items { jobId jobName jobStatus sourceLanguageCode targetLanguageCodes } } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = json["translateTextTranslationJobs"]["items"]
            .as_array()
            .unwrap();
        assert_eq!(items.len(), 1);
        let j1 = &items[0];
        assert_eq!(j1["jobId"], "j1");
        assert_eq!(j1["jobName"], "job-one");
        assert_eq!(j1["jobStatus"], "COMPLETED");
        assert_eq!(j1["sourceLanguageCode"], "en");
        assert_eq!(j1["targetLanguageCodes"], serde_json::json!(["es"]));
        http_client.relaxed_requests_match();
    }
}
