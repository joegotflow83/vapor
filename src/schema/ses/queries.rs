use async_graphql::{Context, Object, Result};

use crate::aws::ses::SesClient;
use crate::schema::pagination::Page;
use crate::schema::ses::types::{
    SesAccountDetails, SesConfigurationSet, SesEmailTemplate, SesIdentity, SesSuppressedDestination,
};

#[derive(Default)]
pub struct SesQuery;

#[Object]
impl SesQuery {
    async fn ses_identities(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SesIdentity>> {
        let client = ctx.data::<SesClient>()?;
        let (items, next_token) = client.list_email_identities(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(SesIdentity::from).collect(),
            next_token,
        })
    }

    async fn ses_identity(
        &self,
        ctx: &Context<'_>,
        identity: String,
    ) -> Result<Option<SesIdentity>> {
        let client = ctx.data::<SesClient>()?;
        let result = client.get_email_identity(identity).await?;
        Ok(result.map(SesIdentity::from))
    }

    async fn ses_configuration_sets(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SesConfigurationSet>> {
        let client = ctx.data::<SesClient>()?;
        let (items, next_token) = client.list_configuration_sets(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(SesConfigurationSet::from).collect(),
            next_token,
        })
    }

    async fn ses_email_templates(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SesEmailTemplate>> {
        let client = ctx.data::<SesClient>()?;
        let (items, next_token) = client.list_email_templates(limit, next_token).await?;
        Ok(Page {
            items: items.into_iter().map(SesEmailTemplate::from).collect(),
            next_token,
        })
    }

    async fn ses_suppressed_destinations(
        &self,
        ctx: &Context<'_>,
        reasons: Option<Vec<String>>,
        start_date: Option<String>,
        end_date: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<SesSuppressedDestination>> {
        let client = ctx.data::<SesClient>()?;
        let (items, next_token) = client
            .list_suppressed_destinations(reasons, start_date, end_date, limit, next_token)
            .await?;
        Ok(Page {
            items: items
                .into_iter()
                .map(SesSuppressedDestination::from)
                .collect(),
            next_token,
        })
    }

    async fn ses_account_details(&self, ctx: &Context<'_>) -> Result<Option<SesAccountDetails>> {
        let client = ctx.data::<SesClient>()?;
        let account = client.get_account().await?;
        Ok(Some(SesAccountDetails::from(account)))
    }
}

// All 6 resolvers are bare passthroughs to already-tested `SesClient`
// methods (pagination/fan-out/error-mapping behavior lives in
// `src/aws/ses.rs`'s own test module; `From` impl field-mapping is already
// covered by `types.rs`'s own tests) — only light smoke tests are needed
// here per the resolver-layer sweep's stated scope.
#[cfg(test)]
mod tests {
    use crate::aws::ses::SesClient;
    use crate::aws::test_util::{
        json_response, request, sdk_config, ReplayEvent, StaticReplayClient,
    };
    use crate::schema::test_util::build_query_schema;

    use super::SesQuery;

    const BASE: &str = "https://email.us-east-1.amazonaws.com";

    #[tokio::test]
    async fn ses_identities_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/identities?PageSize=1"), ""),
            json_response(
                200,
                r#"{"EmailIdentities":[{"IdentityType":"EMAIL_ADDRESS","IdentityName":"alice@example.com","SendingEnabled":true}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(SesQuery)
            .data(SesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ sesIdentities(limit: 1) { items { identity identityType sendingEnabled } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["sesIdentities"]["items"];
        assert_eq!(items[0]["identity"], "alice@example.com");
        assert_eq!(items[0]["identityType"], "EMAIL_ADDRESS");
        assert_eq!(items[0]["sendingEnabled"], true);
        assert_eq!(json["sesIdentities"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ses_identity_maps_single_identity() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/identities/example.com"), ""),
            json_response(
                200,
                r#"{"IdentityType":"DOMAIN","VerifiedForSendingStatus":true,"DkimAttributes":{"SigningEnabled":true,"Status":"SUCCESS"}}"#,
            ),
        )]);
        let schema = build_query_schema(SesQuery)
            .data(SesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ sesIdentity(identity: "example.com") { identity identityType sendingEnabled dkimStatus } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["sesIdentity"]["identity"], "example.com");
        assert_eq!(json["sesIdentity"]["identityType"], "DOMAIN");
        assert_eq!(json["sesIdentity"]["sendingEnabled"], true);
        assert_eq!(json["sesIdentity"]["dkimStatus"], "SUCCESS");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ses_configuration_sets_maps_items_and_fans_out_details() {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(
                    &format!("{BASE}/v2/email/configuration-sets?PageSize=1"),
                    "",
                ),
                json_response(
                    200,
                    r#"{"ConfigurationSets":["set-a"],"NextToken":"page2"}"#,
                ),
            ),
            ReplayEvent::new(
                request(&format!("{BASE}/v2/email/configuration-sets/set-a"), ""),
                json_response(
                    200,
                    r#"{"SendingOptions":{"SendingEnabled":true},"Tags":[{"Key":"env","Value":"prod"}]}"#,
                ),
            ),
        ]);
        let schema = build_query_schema(SesQuery)
            .data(SesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ sesConfigurationSets(limit: 1) { items { name sendingOptions { sendingEnabled } tags { key value } } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["sesConfigurationSets"]["items"];
        assert_eq!(items[0]["name"], "set-a");
        assert_eq!(items[0]["sendingOptions"]["sendingEnabled"], true);
        assert_eq!(items[0]["tags"][0]["key"], "env");
        assert_eq!(json["sesConfigurationSets"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ses_email_templates_maps_items_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/templates?PageSize=1"), ""),
            json_response(
                200,
                r#"{"TemplatesMetadata":[{"TemplateName":"welcome","CreatedTimestamp":1700000000}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(SesQuery)
            .data(SesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ sesEmailTemplates(limit: 1) { items { templateName createdTimestamp } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["sesEmailTemplates"]["items"];
        assert_eq!(items[0]["templateName"], "welcome");
        assert!(items[0]["createdTimestamp"].is_string());
        assert_eq!(json["sesEmailTemplates"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ses_suppressed_destinations_forwards_filters_and_maps_items() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                &format!("{BASE}/v2/email/suppression/addresses?Reason=BOUNCE&PageSize=1"),
                "",
            ),
            json_response(
                200,
                r#"{"SuppressedDestinationSummaries":[{"EmailAddress":"a@example.com","Reason":"BOUNCE","LastUpdateTime":1700000000}],"NextToken":"page2"}"#,
            ),
        )]);
        let schema = build_query_schema(SesQuery)
            .data(SesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ sesSuppressedDestinations(reasons: ["BOUNCE"], limit: 1) { items { emailAddress reason } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["sesSuppressedDestinations"]["items"];
        assert_eq!(items[0]["emailAddress"], "a@example.com");
        assert_eq!(items[0]["reason"], "BOUNCE");
        assert_eq!(json["sesSuppressedDestinations"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn ses_account_details_maps_account_info() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(&format!("{BASE}/v2/email/account"), ""),
            json_response(
                200,
                r#"{"SendingEnabled":true,"SendQuota":{"Max24HourSend":50000.0,"MaxSendRate":14.0,"SentLast24Hours":120.0}}"#,
            ),
        )]);
        let schema = build_query_schema(SesQuery)
            .data(SesClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ sesAccountDetails { sendingEnabled sendingQuota maxSendRate sentLast24Hours } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["sesAccountDetails"]["sendingEnabled"], true);
        assert_eq!(json["sesAccountDetails"]["sendingQuota"], 50000.0);
        assert_eq!(json["sesAccountDetails"]["maxSendRate"], 14.0);
        assert_eq!(json["sesAccountDetails"]["sentLast24Hours"], 120.0);
        http_client.relaxed_requests_match();
    }
}
