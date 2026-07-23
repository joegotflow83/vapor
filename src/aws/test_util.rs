//! Shared `StaticReplayClient`-backed test scaffolding for `src/aws/*.rs`
//! client tests (plan-3 test coverage). One canonical copy of the
//! `SdkConfig` builder and canned-response helpers — per-file tests should
//! use these rather than pasting their own copies.
//!
//! `StaticReplayClient` hands out responses in the order given regardless
//! of the actual request sent; call `.relaxed_requests_match()` at the end
//! of a test to assert on what was actually sent (e.g. pagination token /
//! `limit` passthrough), ignoring the non-deterministic `x-amz-user-agent`/
//! `authorization` headers.
//!
//! Unused-import/dead-code allows below: this module is consumed
//! incrementally by the per-`src/aws/*.rs` test sweep (plan-3), so some
//! exports have no caller yet.
#![allow(unused_imports, dead_code)]

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_credential_types::Credentials;
use aws_smithy_types::body::SdkBody;
use http::{Request, Response};

pub(crate) use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};

/// Builds a `SdkConfig` wired to `http_client` so AWS SDK client wrappers
/// (`AcmClient::new(&cfg)` etc.) exercise real request-building /
/// response-parsing with no network I/O. Region/credentials are fixed test
/// values — no wrapper method under test depends on their content.
pub(crate) fn sdk_config(http_client: StaticReplayClient) -> SdkConfig {
    SdkConfig::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .credentials_provider(SharedCredentialsProvider::new(Credentials::for_tests()))
        .http_client(http_client)
        .build()
}

/// A `ReplayEvent`'s expected-request half. `uri` should be the full
/// request URL the client is expected to send; `body` is compared
/// field-by-field (not byte-for-byte) when the content type is JSON.
pub(crate) fn request(uri: &str, body: impl Into<String>) -> Request<SdkBody> {
    Request::builder()
        .uri(uri)
        .body(SdkBody::from(body.into()))
        .unwrap()
}

/// A canned restJson1/awsJson1_x success response.
pub(crate) fn json_response(status: u16, body: impl Into<String>) -> Response<SdkBody> {
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(SdkBody::from(body.into()))
        .unwrap()
}

/// A canned restJson1/awsJson1_x error response, matching the `__type` +
/// `message` shape `error.rs::sdk_err` reads error code/message out of.
pub(crate) fn json_error_response(error_type: &str, message: &str) -> Response<SdkBody> {
    json_response(
        400,
        format!(r#"{{"__type":"{error_type}","message":"{message}"}}"#),
    )
}

/// A canned awsQuery/ec2Query success response (XML body). Unlike restJson1/
/// awsJson1_x, `StaticReplayClient` compares the *request* body byte-for-byte
/// for this protocol (its `Content-Type: application/x-www-form-urlencoded`
/// doesn't contain "json", so `MediaType::Other` applies, not the
/// semantic JSON compare) — expected request bodies must match the SDK's
/// exact `QueryWriter` field order/encoding.
pub(crate) fn xml_response(status: u16, body: impl Into<String>) -> Response<SdkBody> {
    Response::builder()
        .status(status)
        .header("content-type", "text/xml")
        .body(SdkBody::from(body.into()))
        .unwrap()
}

/// A canned awsQuery/ec2Query error response, matching the `<ErrorResponse>
/// <Error><Code>.../<Message>...` shape `rest_xml_wrapped_errors::
/// parse_error_metadata` reads error code/message out of.
pub(crate) fn xml_error_response(error_code: &str, message: &str) -> Response<SdkBody> {
    xml_response(
        400,
        format!(
            "<ErrorResponse><Error><Code>{error_code}</Code><Message>{message}</Message></Error></ErrorResponse>"
        ),
    )
}

/// A canned ec2Query error response. Unlike the generic awsQuery family's
/// `<ErrorResponse><Error>...</Error></ErrorResponse>` (see
/// `xml_error_response`), `aws-sdk-ec2` has its own `ec2_query_errors::
/// parse_error_metadata`, which reads a distinct
/// `<Response><Errors><Error><Code>.../<Message>...</Error></Errors></Response>`
/// shape — verified against the pinned `aws-sdk-ec2` crate's
/// `ec2_query_errors.rs` rather than assumed from the generic awsQuery
/// helper above.
pub(crate) fn ec2_error_response(error_code: &str, message: &str) -> Response<SdkBody> {
    xml_response(
        400,
        format!(
            "<Response><Errors><Error><Code>{error_code}</Code><Message>{message}</Message></Error></Errors><RequestId>test-request-id</RequestId></Response>"
        ),
    )
}

/// A canned S3-style unwrapped-XML error response, matching the `<Error>
/// <Code>...</Code><Message>...</Message></Error>` shape (no outer
/// `<ErrorResponse>` wrapper) that `rest_xml_unwrapped_errors::
/// parse_error_metadata` reads error code/message out of — distinct from the
/// generic awsQuery/ec2Query wrapped shape `xml_error_response` produces.
pub(crate) fn s3_error_response(status: u16, error_code: &str, message: &str) -> Response<SdkBody> {
    xml_response(
        status,
        format!("<Error><Code>{error_code}</Code><Message>{message}</Message></Error>"),
    )
}

/// A `ReplayEvent`'s expected-request half for Smithy RPC v2 CBOR
/// (`rpc-v2-cbor`) bodies — the wire format used by newer-generation SDKs
/// (e.g. `aws-sdk-cloudwatch`) that moved off awsJson1.1 while keeping
/// awsQuery-compatible error codes. `StaticReplayClient` falls back to
/// exact byte-for-byte body comparison for any non-JSON content type (CBOR
/// is binary, not valid UTF-8 in general), so `body` must match the SDK's
/// own serialization exactly — build it with `aws_smithy_cbor::Encoder`,
/// mirroring the pinned SDK's `ser_*_input` codegen field order exactly,
/// not reordered for readability.
pub(crate) fn cbor_request(uri: &str, body: Vec<u8>) -> Request<SdkBody> {
    Request::builder()
        .uri(uri)
        .body(SdkBody::from(body))
        .unwrap()
}

/// A canned `rpc-v2-cbor` success response. Build `body` with
/// `aws_smithy_cbor::Encoder` — response map field order doesn't matter
/// (the decoder matches by key name, not position), unlike `cbor_request`'s
/// byte-exact requirement.
pub(crate) fn cbor_response(status: u16, body: Vec<u8>) -> Response<SdkBody> {
    Response::builder()
        .status(status)
        .header("content-type", "application/cbor")
        .header("smithy-protocol", "rpc-v2-cbor")
        .body(SdkBody::from(body))
        .unwrap()
}

/// A canned `rpc-v2-cbor` error response, matching the `__type`/`message`
/// CBOR-map shape `cbor_errors::parse_error_metadata` reads error code/
/// message out of (same field names as awsJson1.1's `__type`/`message`
/// shape, just CBOR-encoded rather than JSON text).
pub(crate) fn cbor_error_response(error_type: &str, message: &str) -> Response<SdkBody> {
    let mut encoder = aws_smithy_cbor::Encoder::new(Vec::new());
    encoder
        .begin_map()
        .str("__type")
        .str(error_type)
        .str("message")
        .str(message)
        .end();
    cbor_response(400, encoder.into_writer())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_config_wires_up_the_replay_client() {
        let cfg = sdk_config(StaticReplayClient::new(vec![]));
        assert_eq!(cfg.region().map(Region::as_ref), Some("us-east-1"));
        assert!(cfg.http_client().is_some());
        assert!(cfg.credentials_provider().is_some());
    }

    #[test]
    fn json_error_response_embeds_type_and_message() {
        let resp = json_error_response("AccessDeniedException", "not authorized");
        assert_eq!(resp.status(), 400);
        let body = std::str::from_utf8(resp.body().bytes().unwrap()).unwrap();
        assert!(body.contains("AccessDeniedException"));
        assert!(body.contains("not authorized"));
    }
}
