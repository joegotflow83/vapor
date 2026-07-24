use async_graphql::{Context, Object, Result};

use crate::aws::sts::StsClient;
use crate::schema::sts::types::CallerIdentity;

#[derive(Default)]
pub struct StsQuery;

#[Object]
impl StsQuery {
    async fn sts_caller_identity(&self, ctx: &Context<'_>) -> Result<CallerIdentity> {
        let client = ctx.data::<StsClient>()?;
        let output = client.get_caller_identity().await?;
        Ok(CallerIdentity::from(output))
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::sts::StsClient;
    use crate::aws::test_util::{
        request, sdk_config, xml_response, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::StsQuery;

    const ENDPOINT: &str = "https://sts.us-east-1.amazonaws.com/";

    #[tokio::test]
    async fn sts_caller_identity_maps_fields() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=GetCallerIdentity&Version=2011-06-15"),
            xml_response(
                200,
                "<GetCallerIdentityResponse><GetCallerIdentityResult>\
                 <UserId>AIDACKCEVSQ6C2EXAMPLE</UserId>\
                 <Account>123456789012</Account>\
                 <Arn>arn:aws:iam::123456789012:user/Alice</Arn>\
                 </GetCallerIdentityResult></GetCallerIdentityResponse>",
            ),
        )]);
        let schema = build_query_schema(StsQuery)
            .data(StsClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ stsCallerIdentity { account arn userId } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let identity = &json["stsCallerIdentity"];
        assert_eq!(identity["account"], "123456789012");
        assert_eq!(identity["arn"], "arn:aws:iam::123456789012:user/Alice");
        assert_eq!(identity["userId"], "AIDACKCEVSQ6C2EXAMPLE");
    }
}
