use async_graphql::{Context, Object, Result};

use crate::aws::iam::IamClient;
use crate::schema::iam::types::{
    IamAccessKey, IamAttachedPolicy, IamGroup, IamInlinePolicy, IamMfaDevice, IamPasswordPolicy,
    IamPolicy, IamPolicyDocument, IamRole, IamUser,
};
use crate::schema::pagination::Page;

#[derive(Default)]
pub struct IamQuery;

#[Object]
impl IamQuery {
    /// List IAM roles. Optionally filter by path prefix (e.g. "/service-role/").
    /// `limit`/`next_token` paginate the returned list.
    async fn iam_roles(
        &self,
        ctx: &Context<'_>,
        path_prefix: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamRole>> {
        let iam = ctx.data::<IamClient>()?;
        let (roles, token) = iam
            .list_roles(path_prefix.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: roles.into_iter().map(IamRole::from).collect(),
            next_token: token,
        })
    }

    /// List IAM managed policies. scope accepts "Local" (default, customer-managed),
    /// "AWS" (AWS-managed), or "All". Optionally filter by path prefix.
    /// `limit`/`next_token` paginate the returned list.
    async fn iam_policies(
        &self,
        ctx: &Context<'_>,
        scope: Option<String>,
        path_prefix: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamPolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let scope_str = scope.as_deref().unwrap_or("Local");
        let (policies, token) = iam
            .list_policies(scope_str, path_prefix.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: policies.into_iter().map(IamPolicy::from).collect(),
            next_token: token,
        })
    }

    /// List IAM users. Optionally filter by path prefix.
    /// `limit`/`next_token` paginate the returned list.
    async fn iam_users(
        &self,
        ctx: &Context<'_>,
        path_prefix: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamUser>> {
        let iam = ctx.data::<IamClient>()?;
        let (users, token) = iam
            .list_users(path_prefix.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: users.into_iter().map(IamUser::from).collect(),
            next_token: token,
        })
    }

    /// List IAM groups. Optionally filter by path prefix.
    /// `limit`/`next_token` paginate the returned list.
    async fn iam_groups(
        &self,
        ctx: &Context<'_>,
        path_prefix: Option<String>,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamGroup>> {
        let iam = ctx.data::<IamClient>()?;
        let (groups, token) = iam
            .list_groups(path_prefix.as_deref(), limit, next_token)
            .await?;
        Ok(Page {
            items: groups.into_iter().map(IamGroup::from).collect(),
            next_token: token,
        })
    }

    /// List policies attached to an IAM role.
    /// `limit`/`next_token` paginate the returned list.
    async fn iam_attached_role_policies(
        &self,
        ctx: &Context<'_>,
        role_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamAttachedPolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let (policies, token) = iam
            .list_attached_role_policies(&role_name, limit, next_token)
            .await?;
        Ok(Page {
            items: policies.into_iter().map(IamAttachedPolicy::from).collect(),
            next_token: token,
        })
    }

    /// Fetch the JSON document for a managed IAM policy.
    /// Optionally specify a version_id (e.g. "v3"); defaults to the policy's current default version.
    async fn iam_policy_document(
        &self,
        ctx: &Context<'_>,
        policy_arn: String,
        version_id: Option<String>,
    ) -> Result<IamPolicyDocument> {
        let iam = ctx.data::<IamClient>()?;
        let version = iam
            .get_managed_policy_document(&policy_arn, version_id.as_deref())
            .await?;
        Ok(IamPolicyDocument::from((policy_arn, version)))
    }

    /// List all inline policies embedded directly in an IAM role, including their decoded JSON documents.
    /// `limit`/`next_token` paginate the policy-name discovery list before the document fan-out.
    async fn iam_role_inline_policies(
        &self,
        ctx: &Context<'_>,
        role_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamInlinePolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let (policies, token) = iam
            .get_role_inline_policies(&role_name, limit, next_token)
            .await?;
        Ok(Page {
            items: policies.into_iter().map(IamInlinePolicy::from).collect(),
            next_token: token,
        })
    }

    /// Fetch the account-wide IAM password policy.
    /// Returns null if no custom password policy has been configured —
    /// AWS then applies minimal defaults (8-char minimum, no complexity requirements).
    /// Use this to audit CIS AWS Benchmark 1.x controls: minimum length ≥14,
    /// all complexity flags enabled, max_password_age ≤90, reuse prevention ≥24,
    /// hard_expiry enabled.
    async fn iam_password_policy(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<IamPasswordPolicy>> {
        let iam = ctx.data::<IamClient>()?;
        let policy = iam.get_account_password_policy().await?;
        Ok(policy.map(IamPasswordPolicy::from))
    }

    /// List MFA devices enrolled for an IAM user.
    /// Returns an empty list when the user has no MFA configured.
    /// Cross-reference with iamUsers to detect users lacking MFA (CIS AWS Benchmark 1.10).
    /// `limit`/`next_token` paginate the returned list.
    async fn iam_mfa_devices(
        &self,
        ctx: &Context<'_>,
        user_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamMfaDevice>> {
        let iam = ctx.data::<IamClient>()?;
        let (devices, token) = iam.list_mfa_devices(&user_name, limit, next_token).await?;
        Ok(Page {
            items: devices.into_iter().map(IamMfaDevice::from).collect(),
            next_token: token,
        })
    }

    /// List access keys for an IAM user, enriched with last-used metadata.
    /// Use this to detect stale or inactive credentials (keys unused for 90+ days,
    /// keys that are Inactive, etc.).
    /// `limit`/`next_token` paginate the access-key discovery list before the
    /// per-key last-used-metadata fan-out.
    async fn iam_access_keys(
        &self,
        ctx: &Context<'_>,
        user_name: String,
        limit: Option<i32>,
        next_token: Option<String>,
    ) -> Result<Page<IamAccessKey>> {
        let iam = ctx.data::<IamClient>()?;
        let (keys, token) = iam.list_access_keys(&user_name, limit, next_token).await?;
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            let key_id = key.access_key_id().unwrap_or("").to_string();
            let last_used = iam.get_access_key_last_used(&key_id).await?;
            result.push(IamAccessKey::from((key, last_used)));
        }
        Ok(Page {
            items: result,
            next_token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::aws::iam::IamClient;
    use crate::aws::test_util::{request, sdk_config, xml_error_response, xml_response, ReplayEvent, StaticReplayClient};
    use crate::schema::test_util::build_query_schema;

    use super::IamQuery;

    const ENDPOINT: &str = "https://iam.amazonaws.com/";
    const POLICY_ARN: &str = "arn:aws:iam::123456789012:policy/test-policy";
    const POLICY_ARN_ENC: &str = "arn%3Aaws%3Aiam%3A%3A123456789012%3Apolicy%2Ftest-policy";

    #[tokio::test]
    async fn iam_roles_forwards_path_prefix_and_next_token() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListRoles&Version=2010-05-08&PathPrefix=%2Fapp%2F&MaxItems=1"),
            xml_response(
                200,
                "<ListRolesResponse><ListRolesResult><Roles><member>\
                 <RoleName>my-role</RoleName><RoleId>AROAEXAMPLE</RoleId>\
                 <Arn>arn:aws:iam::123456789012:role/my-role</Arn><Path>/app/</Path>\
                 <CreateDate>2024-01-01T00:00:00Z</CreateDate></member></Roles>\
                 <IsTruncated>true</IsTruncated><Marker>page2</Marker>\
                 </ListRolesResult></ListRolesResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iamRoles(pathPrefix: "/app/", limit: 1) { items { roleName arn path } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iamRoles"]["items"];
        assert_eq!(items[0]["roleName"], "my-role");
        assert_eq!(items[0]["arn"], "arn:aws:iam::123456789012:role/my-role");
        assert_eq!(items[0]["path"], "/app/");
        assert_eq!(json["iamRoles"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_policies_forwards_scope_and_path_prefix() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListPolicies&Version=2010-05-08&Scope=AWS&PathPrefix=%2F&MaxItems=1"),
            xml_response(
                200,
                "<ListPoliciesResponse><ListPoliciesResult><Policies><member>\
                 <PolicyName>AdministratorAccess</PolicyName>\
                 <Arn>arn:aws:iam::aws:policy/AdministratorAccess</Arn>\
                 <DefaultVersionId>v1</DefaultVersionId></member></Policies>\
                 <IsTruncated>true</IsTruncated><Marker>page2</Marker>\
                 </ListPoliciesResult></ListPoliciesResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iamPolicies(scope: "AWS", pathPrefix: "/", limit: 1) { items { policyName arn defaultVersionId } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iamPolicies"]["items"];
        assert_eq!(items[0]["policyName"], "AdministratorAccess");
        assert_eq!(items[0]["defaultVersionId"], "v1");
        assert_eq!(json["iamPolicies"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_policies_defaults_scope_to_local_when_omitted() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListPolicies&Version=2010-05-08&Scope=Local"),
            xml_response(
                200,
                "<ListPoliciesResponse><ListPoliciesResult><Policies></Policies>\
                 <IsTruncated>false</IsTruncated></ListPoliciesResult></ListPoliciesResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute(r#"{ iamPolicies { items { policyName } nextToken } }"#).await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamPolicies"]["items"].as_array().unwrap().len(), 0);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_users_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListUsers&Version=2010-05-08"),
            xml_response(
                200,
                "<ListUsersResponse><ListUsersResult><Users><member>\
                 <UserName>alice</UserName><UserId>AIDAEXAMPLE</UserId>\
                 <Arn>arn:aws:iam::123456789012:user/alice</Arn><Path>/</Path>\
                 <CreateDate>2024-01-01T00:00:00Z</CreateDate></member></Users>\
                 <IsTruncated>false</IsTruncated></ListUsersResult></ListUsersResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ iamUsers { items { userName arn path } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamUsers"]["items"][0]["userName"], "alice");
        assert_eq!(json["iamUsers"]["nextToken"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_groups_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListGroups&Version=2010-05-08"),
            xml_response(
                200,
                "<ListGroupsResponse><ListGroupsResult><Groups><member>\
                 <GroupName>admins</GroupName><GroupId>AGPAEXAMPLE</GroupId>\
                 <Arn>arn:aws:iam::123456789012:group/admins</Arn><Path>/</Path>\
                 <CreateDate>2024-01-01T00:00:00Z</CreateDate></member></Groups>\
                 <IsTruncated>false</IsTruncated></ListGroupsResult></ListGroupsResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ iamGroups { items { groupName arn } nextToken } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamGroups"]["items"][0]["groupName"], "admins");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_attached_role_policies_forwards_role_name() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                "Action=ListAttachedRolePolicies&Version=2010-05-08&RoleName=my-role&MaxItems=1",
            ),
            xml_response(
                200,
                "<ListAttachedRolePoliciesResponse><ListAttachedRolePoliciesResult>\
                 <AttachedPolicies><member><PolicyName>AdministratorAccess</PolicyName>\
                 <PolicyArn>arn:aws:iam::aws:policy/AdministratorAccess</PolicyArn></member>\
                 </AttachedPolicies><IsTruncated>true</IsTruncated><Marker>page2</Marker>\
                 </ListAttachedRolePoliciesResult></ListAttachedRolePoliciesResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iamAttachedRolePolicies(roleName: "my-role", limit: 1) { items { policyName policyArn } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamAttachedRolePolicies"]["items"][0]["policyName"], "AdministratorAccess");
        assert_eq!(json["iamAttachedRolePolicies"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_policy_document_with_explicit_version_skips_get_policy() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(
                ENDPOINT,
                format!("Action=GetPolicyVersion&Version=2010-05-08&PolicyArn={POLICY_ARN_ENC}&VersionId=v2"),
            ),
            xml_response(
                200,
                "<GetPolicyVersionResponse><GetPolicyVersionResult><PolicyVersion>\
                 <Document>%7B%22Version%22%3A%222012-10-17%22%7D</Document><VersionId>v2</VersionId>\
                 <IsDefaultVersion>false</IsDefaultVersion></PolicyVersion>\
                 </GetPolicyVersionResult></GetPolicyVersionResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let query = format!(
            r#"{{ iamPolicyDocument(policyArn: "{POLICY_ARN}", versionId: "v2") {{ policyArn versionId isDefaultVersion document }} }}"#
        );
        let res = schema.execute(query.as_str()).await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamPolicyDocument"]["policyArn"], POLICY_ARN);
        assert_eq!(json["iamPolicyDocument"]["versionId"], "v2");
        assert_eq!(json["iamPolicyDocument"]["isDefaultVersion"], false);
        assert_eq!(json["iamPolicyDocument"]["document"], r#"{"Version":"2012-10-17"}"#);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_role_inline_policies_happy_path_fanout() {
        // `get_role_inline_policies`'s own discovery-then-`GetRolePolicy`-fanout
        // loop is already covered by `src/aws/iam.rs`'s own test module; this
        // exercises it end-to-end through the resolver (connect/control_tower
        // precedent) rather than forcing a zero-result response.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListRolePolicies&Version=2010-05-08&RoleName=my-role"),
                xml_response(
                    200,
                    "<ListRolePoliciesResponse><ListRolePoliciesResult><PolicyNames>\
                     <member>inline-policy-1</member></PolicyNames><IsTruncated>false</IsTruncated>\
                     </ListRolePoliciesResult></ListRolePoliciesResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetRolePolicy&Version=2010-05-08&RoleName=my-role&PolicyName=inline-policy-1",
                ),
                xml_response(
                    200,
                    "<GetRolePolicyResponse><GetRolePolicyResult><RoleName>my-role</RoleName>\
                     <PolicyName>inline-policy-1</PolicyName>\
                     <PolicyDocument>%7B%22Version%22%3A%222012-10-17%22%7D</PolicyDocument>\
                     </GetRolePolicyResult></GetRolePolicyResponse>",
                ),
            ),
        ]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iamRoleInlinePolicies(roleName: "my-role") { items { policyName document } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamRoleInlinePolicies"]["items"][0]["policyName"], "inline-policy-1");
        assert_eq!(
            json["iamRoleInlinePolicies"]["items"][0]["document"],
            r#"{"Version":"2012-10-17"}"#
        );
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_password_policy_configured() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=GetAccountPasswordPolicy&Version=2010-05-08"),
            xml_response(
                200,
                "<GetAccountPasswordPolicyResponse><GetAccountPasswordPolicyResult>\
                 <PasswordPolicy><MinimumPasswordLength>14</MinimumPasswordLength>\
                 <RequireSymbols>true</RequireSymbols><MaxPasswordAge>90</MaxPasswordAge>\
                 </PasswordPolicy></GetAccountPasswordPolicyResult></GetAccountPasswordPolicyResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ iamPasswordPolicy { minimumPasswordLength requireSymbols maxPasswordAge } }"#)
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamPasswordPolicy"]["minimumPasswordLength"], 14);
        assert_eq!(json["iamPasswordPolicy"]["requireSymbols"], true);
        assert_eq!(json["iamPasswordPolicy"]["maxPasswordAge"], 90);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_password_policy_null_when_not_configured() {
        // Only the explicit `NoSuchEntityException` catch in
        // `get_account_password_policy` reaches `Ok(None)` here (gotcha 14 —
        // an empty-body response gets filled in by the SDK's own
        // `*_correct_errors` default synthesis instead).
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=GetAccountPasswordPolicy&Version=2010-05-08"),
            xml_error_response("NoSuchEntity", "no custom password policy"),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema.execute(r#"{ iamPasswordPolicy { requireSymbols } }"#).await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamPasswordPolicy"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_mfa_devices_happy_path() {
        let http_client = StaticReplayClient::new(vec![ReplayEvent::new(
            request(ENDPOINT, "Action=ListMFADevices&Version=2010-05-08&UserName=alice&MaxItems=1"),
            xml_response(
                200,
                "<ListMFADevicesResponse><ListMFADevicesResult><MFADevices><member>\
                 <UserName>alice</UserName><SerialNumber>arn:aws:iam::123456789012:mfa/alice</SerialNumber>\
                 <EnableDate>2024-01-01T00:00:00Z</EnableDate></member></MFADevices>\
                 <IsTruncated>true</IsTruncated><Marker>page2</Marker>\
                 </ListMFADevicesResult></ListMFADevicesResponse>",
            ),
        )]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iamMfaDevices(userName: "alice", limit: 1) { items { userName serialNumber } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        assert_eq!(json["iamMfaDevices"]["items"][0]["serialNumber"], "arn:aws:iam::123456789012:mfa/alice");
        assert_eq!(json["iamMfaDevices"]["nextToken"], "page2");
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_access_keys_fans_out_to_last_used_per_key() {
        // Unlike every other resolver in this file, `iam_access_keys` owns
        // its own per-key fan-out loop directly in the resolver body (not
        // inside `src/aws/iam.rs`), so it earns bespoke coverage proving the
        // enrichment actually happens for more than one discovered key, in
        // declaration order (gotcha 27: no real executor suspension, so a
        // plain `for` loop's calls land in source order in the mock queue).
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListAccessKeys&Version=2010-05-08&UserName=alice"),
                xml_response(
                    200,
                    "<ListAccessKeysResponse><ListAccessKeysResult><AccessKeyMetadata>\
                     <member><UserName>alice</UserName><AccessKeyId>AKIAFIRST</AccessKeyId>\
                     <Status>Active</Status></member>\
                     <member><UserName>alice</UserName><AccessKeyId>AKIASECOND</AccessKeyId>\
                     <Status>Inactive</Status></member>\
                     </AccessKeyMetadata><IsTruncated>false</IsTruncated>\
                     </ListAccessKeysResult></ListAccessKeysResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetAccessKeyLastUsed&Version=2010-05-08&AccessKeyId=AKIAFIRST",
                ),
                xml_response(
                    200,
                    "<GetAccessKeyLastUsedResponse><GetAccessKeyLastUsedResult>\
                     <UserName>alice</UserName><AccessKeyLastUsed>\
                     <LastUsedDate>2024-01-01T00:00:00Z</LastUsedDate><ServiceName>s3</ServiceName>\
                     <Region>us-east-1</Region></AccessKeyLastUsed>\
                     </GetAccessKeyLastUsedResult></GetAccessKeyLastUsedResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetAccessKeyLastUsed&Version=2010-05-08&AccessKeyId=AKIASECOND",
                ),
                xml_response(
                    200,
                    "<GetAccessKeyLastUsedResponse><GetAccessKeyLastUsedResult>\
                     <UserName>alice</UserName>\
                     </GetAccessKeyLastUsedResult></GetAccessKeyLastUsedResponse>",
                ),
            ),
        ]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(
                r#"{ iamAccessKeys(userName: "alice") { items { accessKeyId status lastUsedService lastUsedRegion } nextToken } }"#,
            )
            .await;

        assert!(res.errors.is_empty(), "unexpected errors: {:?}", res.errors);
        let json = res.data.into_json().unwrap();
        let items = &json["iamAccessKeys"]["items"];
        assert_eq!(items[0]["accessKeyId"], "AKIAFIRST");
        assert_eq!(items[0]["status"], "Active");
        assert_eq!(items[0]["lastUsedService"], "s3");
        assert_eq!(items[0]["lastUsedRegion"], "us-east-1");
        assert_eq!(items[1]["accessKeyId"], "AKIASECOND");
        assert_eq!(items[1]["status"], "Inactive");
        assert_eq!(items[1]["lastUsedService"], serde_json::Value::Null);
        http_client.relaxed_requests_match();
    }

    #[tokio::test]
    async fn iam_access_keys_propagates_error_from_last_used_lookup() {
        // The resolver's fan-out loop uses plain `?` (no `.ok()`/
        // `unwrap_or_default()` swallowing), so a `GetAccessKeyLastUsed`
        // failure on one key aborts the whole list, unlike e.g.
        // codepipeline's `.ok().flatten()` per-item error-swallowing.
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                request(ENDPOINT, "Action=ListAccessKeys&Version=2010-05-08&UserName=alice"),
                xml_response(
                    200,
                    "<ListAccessKeysResponse><ListAccessKeysResult><AccessKeyMetadata><member>\
                     <UserName>alice</UserName><AccessKeyId>AKIAFIRST</AccessKeyId>\
                     <Status>Active</Status></member></AccessKeyMetadata>\
                     <IsTruncated>false</IsTruncated>\
                     </ListAccessKeysResult></ListAccessKeysResponse>",
                ),
            ),
            ReplayEvent::new(
                request(
                    ENDPOINT,
                    "Action=GetAccessKeyLastUsed&Version=2010-05-08&AccessKeyId=AKIAFIRST",
                ),
                xml_error_response("NoSuchEntity", "no such access key"),
            ),
        ]);
        let schema = build_query_schema(IamQuery)
            .data(IamClient::new(&sdk_config(http_client.clone())))
            .finish();

        let res = schema
            .execute(r#"{ iamAccessKeys(userName: "alice") { items { accessKeyId } } }"#)
            .await;

        assert!(!res.errors.is_empty(), "expected a GraphQL error");
        http_client.relaxed_requests_match();
    }
}
