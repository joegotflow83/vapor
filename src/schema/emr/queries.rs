use async_graphql::{Context, Object, Result};
use futures::future::join_all;

use aws_sdk_emr::types::ClusterState;

use crate::aws::emr::EmrClient;
use crate::schema::emr::types::{EmrCluster, EmrStep};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct EmrQuery;

#[Object]
impl EmrQuery {
    async fn emr_clusters(
        &self,
        ctx: &Context<'_>,
        states: Option<Vec<String>>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EmrCluster>> {
        let client = ctx.data::<EmrClient>()?;

        let cluster_states: Option<Vec<ClusterState>> = states.map(|ss| {
            ss.into_iter()
                .map(|s| ClusterState::from(s.as_str()))
                .collect()
        });

        let (summaries, next_token) = client
            .list_clusters(cluster_states, limit, next_token)
            .await?;

        let futures: Vec<_> = summaries
            .iter()
            .filter_map(|s| s.id())
            .map(|id| client.describe_cluster(id))
            .collect();

        let results = join_all(futures).await;

        let items = results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(EmrCluster::from)
            .collect();

        Ok(Page { items, next_token })
    }

    async fn emr_steps(
        &self,
        ctx: &Context<'_>,
        cluster_id: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<EmrStep>> {
        let client = ctx.data::<EmrClient>()?;
        let (steps, next_token) = client.list_steps(&cluster_id, limit, next_token).await?;
        let items = steps.into_iter().map(EmrStep::from).collect();
        Ok(Page { items, next_token })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::emr::EmrClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::EmrQuery;

    const ENDPOINT: &str = "https://elasticmapreduce.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn emr_clusters_discovers_then_describes_and_forwards_next_token() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "{}"),
                json_response(200, r#"{"Clusters":[{"Id":"j-1"}],"Marker":"page2"}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
                json_response(
                    200,
                    r#"{"Cluster":{"Id":"j-1","Name":"my-cluster","Status":{"State":"RUNNING"}}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EmrQuery)
            .data(EmrClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ emrClusters(limit: 1) { items { id name status } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["emrClusters"]["items"];
        assert_eq!(items[0]["id"], "j-1");
        assert_eq!(items[0]["name"], "my-cluster");
        assert_eq!(items[0]["status"], "RUNNING");
        assert_eq!(json["emrClusters"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn emr_clusters_passes_states_filter() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ClusterStates":["RUNNING"]}"#),
                json_response(200, r#"{"Clusters":[{"Id":"j-2"}]}"#),
            ),
            ReplayEvent::new(
                request(ENDPOINT, r#"{"ClusterId":"j-2"}"#),
                json_response(
                    200,
                    r#"{"Cluster":{"Id":"j-2","Name":"filtered-cluster","Status":{"State":"RUNNING"}}}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(EmrQuery)
            .data(EmrClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ emrClusters(states: ["RUNNING"]) { items { id name } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["emrClusters"]["items"];
        assert_eq!(items[0]["id"], "j-2");
        assert_eq!(items[0]["name"], "filtered-cluster");
        assert!(json["emrClusters"]["nextToken"].is_null());
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn emr_steps_forwards_cluster_id_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, r#"{"ClusterId":"j-1"}"#),
            json_response(200, r#"{"Steps":[{"Id":"a"},{"Id":"b"}],"Marker":"page2"}"#),
        )]);
        let schema = build_query_schema(EmrQuery)
            .data(EmrClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ emrSteps(clusterId: "j-1", limit: 1) { items { id } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["emrSteps"]["items"];
        assert_eq!(items[0]["id"], "a");
        assert_eq!(json["emrSteps"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }
}
