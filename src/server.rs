use async_graphql::http::GraphiQLSource;
use async_graphql_axum::GraphQL;
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Request},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tower_http::timeout::TimeoutLayer;

use crate::schema::aws::registry::{MutationRoot, QueryRoot};
use async_graphql::Schema;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_BODY_BYTES: usize = 1024 * 1024;

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

/// Legacy BSD `inet_aton`-style shorthand (e.g. `"127.1"` == `127.0.0.1`)
/// that Rust's strict `Ipv4Addr::from_str` rejects but some tools/operators
/// still pass. Only the resulting first octet matters here, since the
/// entire `127.0.0.0/8` range is loopback.
fn inet_aton_first_octet(bind: &str) -> Option<u8> {
    let parts: Vec<&str> = bind.split('.').collect();
    if parts.is_empty() || parts.len() > 4 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let nums: Vec<u32> = parts
        .iter()
        .map(|p| p.parse::<u32>().ok())
        .collect::<Option<_>>()?;
    // `(1u64 << bits) as u32` would overflow-wrap to 0 for `bits == 32`
    // (the single-decimal `[a]` arm below), making that arm's guard always
    // false — compare in `u64` instead so the `bits == 32` case (every
    // `u32` fits) is correct too.
    let fits = |n: u32, bits: u32| (n as u64) < (1u64 << bits);
    match nums[..] {
        [a] if fits(a, 32) => Some((a >> 24) as u8),
        [a, b] if a <= 255 && fits(b, 24) => Some(a as u8),
        [a, b, c] if a <= 255 && b <= 255 && fits(c, 16) => Some(a as u8),
        [a, b, c, d] if [a, b, c, d].iter().all(|&n| n <= 255) => Some(a as u8),
        _ => None,
    }
}

/// `127.0.0.1`/`::1`/`localhost` and less-common loopback forms
/// (`127.0.0.2`, `127.1`, …) all count — parsed via `IpAddr`/`is_loopback`
/// rather than string comparison so those aren't missed.
pub fn is_loopback(bind: &str) -> bool {
    if bind.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if let Ok(ip) = IpAddr::from_str(bind) {
        return ip.is_loopback();
    }
    inet_aton_first_octet(bind) == Some(127)
}

/// Refuses non-loopback binds with no auth token configured — the point at
/// which an unauthenticated, AWS-mutating GraphQL endpoint would otherwise
/// become reachable off-box.
pub fn validate_bind_policy(bind: &str, auth_token: &Option<String>) -> Result<(), String> {
    if !is_loopback(bind) && auth_token.is_none() {
        return Err(format!(
            "refusing to bind to {bind} without an auth token: this would expose \
             AWS-mutating GraphQL operations (e.g. terminateInstances, runInstances) \
             to the network with no authentication. Set --auth-token or \
             VAPOR_AUTH_TOKEN, and put a TLS-terminating reverse proxy in front \
             for off-loopback use (bearer tokens over plaintext HTTP are not safe)."
        ));
    }
    Ok(())
}

#[derive(Clone)]
struct AuthState {
    token: Arc<str>,
}

fn unauthorized() -> Response {
    let body = Json(serde_json::json!({"errors": [{"message": "unauthorized"}]}));
    let mut response = (StatusCode::UNAUTHORIZED, body).into_response();
    response.headers_mut().insert(
        axum::http::header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Bearer"),
    );
    response
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then_some(token)
}

async fn auth(
    axum::extract::State(state): axum::extract::State<AuthState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let authorized = bearer_token(&headers)
        .map(|presented| bool::from(presented.as_bytes().ct_eq(state.token.as_bytes())))
        .unwrap_or(false);

    if authorized {
        next.run(request).await
    } else {
        tracing::warn!(target: "vapor::audit", peer = %peer, "rejected unauthenticated request");
        unauthorized()
    }
}

/// Wraps `app` in the bearer-auth layer when a token is configured; a no-op
/// otherwise. Kept separate from `run_server` so it's testable with
/// `tower::ServiceExt::oneshot` against a plain router, no listener needed.
fn with_auth(app: Router, auth_token: Option<String>) -> Router {
    match auth_token {
        Some(token) => app.layer(middleware::from_fn_with_state(
            AuthState {
                token: Arc::from(token.as_str()),
            },
            auth,
        )),
        None => app,
    }
}

pub async fn run_server(
    schema: Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>,
    port: u16,
    bind: &str,
    auth_token: Option<String>,
) {
    if let Err(e) = validate_bind_policy(bind, &auth_token) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    let app = Router::new()
        .route_service("/graphql", GraphQL::new(schema))
        .route("/", get(graphiql))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ));
    // Auth layer goes outermost (added last) so it runs before the body-limit
    // and timeout layers — unauthenticated requests are rejected before any
    // body is read.
    let app = with_auth(app, auth_token.clone());

    if !is_loopback(bind) {
        eprintln!(
            "WARNING: binding to {bind} exposes this server to other hosts on \
             the network. This server's GraphQL schema includes AWS-mutating \
             operations (e.g. terminateInstances, runInstances) executed with \
             the server's own AWS credentials. A bearer token is required and \
             enforced for this bind, but requests travel in plaintext — put a \
             TLS-terminating reverse proxy in front for off-loopback use."
        );
    }

    let addr = format!("{bind}:{port}");
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Error: failed to bind to {addr}: {e}");
            std::process::exit(1);
        }
    };
    println!("GraphiQL playground: http://localhost:{port}/");
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    {
        eprintln!("Error: server failed: {e}");
        std::process::exit(1);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("shutdown signal received, draining in-flight requests");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    fn peer() -> ConnectInfo<SocketAddr> {
        ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345)))
    }

    fn test_router(auth_token: Option<&str>) -> Router {
        let app = Router::new().route("/ping", get(|| async { "pong" }));
        with_auth(app, auth_token.map(str::to_string))
    }

    fn request(auth_header: Option<&str>) -> HttpRequest<Body> {
        let mut builder = HttpRequest::builder().uri("/ping");
        if let Some(value) = auth_header {
            builder = builder.header(axum::http::header::AUTHORIZATION, value);
        }
        let mut req = builder.body(Body::empty()).unwrap();
        req.extensions_mut().insert(peer());
        req
    }

    #[tokio::test]
    async fn no_token_configured_passes_through_without_header() {
        let res = test_router(None).oneshot(request(None)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn token_configured_missing_header_rejected() {
        let res = test_router(Some("secret"))
            .oneshot(request(None))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            res.headers()
                .get(axum::http::header::WWW_AUTHENTICATE)
                .unwrap(),
            "Bearer"
        );
    }

    #[tokio::test]
    async fn token_configured_wrong_token_rejected() {
        let res = test_router(Some("secret"))
            .oneshot(request(Some("Bearer wrong")))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn token_configured_right_token_accepted() {
        let res = test_router(Some("secret"))
            .oneshot(request(Some("Bearer secret")))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn scheme_is_case_insensitive() {
        let res = test_router(Some("secret"))
            .oneshot(request(Some("bearer secret")))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn graphiql_serves_html_with_graphql_endpoint() {
        use axum::response::IntoResponse;

        let res = graphiql().await.into_response();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("/graphql"));
    }

    #[test]
    fn inet_aton_first_octet_table() {
        // 1-part (whole address as a single u32, top octet).
        assert_eq!(inet_aton_first_octet("2130706433"), Some(127));
        // 3-part.
        assert_eq!(inet_aton_first_octet("127.0.1"), Some(127));
        // 4-part, already covered indirectly via is_loopback's "127.0.0.2"
        // case below, but assert the helper directly too.
        assert_eq!(inet_aton_first_octet("127.0.0.2"), Some(127));
        // Malformed: empty segment, too many segments, non-numeric,
        // out-of-range octet in the 2/3/4-part forms, out-of-range single
        // u32 value.
        assert_eq!(inet_aton_first_octet(""), None);
        assert_eq!(inet_aton_first_octet("1.2.3.4.5"), None);
        assert_eq!(inet_aton_first_octet("1..3"), None);
        assert_eq!(inet_aton_first_octet("abc"), None);
        assert_eq!(inet_aton_first_octet("256.0.0.1"), None);
        assert_eq!(inet_aton_first_octet("300.1"), None);
        assert_eq!(inet_aton_first_octet("4294967296"), None);
    }

    #[test]
    fn is_loopback_table() {
        for bind in [
            "127.0.0.1",
            "127.0.0.2",
            "127.1",
            "::1",
            "localhost",
            "LOCALHOST",
        ] {
            assert!(is_loopback(bind), "{bind} should be loopback");
        }
        for bind in ["0.0.0.0", "192.168.1.10", "::"] {
            assert!(!is_loopback(bind), "{bind} should not be loopback");
        }
    }

    #[test]
    fn validate_bind_policy_rejects_non_loopback_without_token() {
        assert!(validate_bind_policy("0.0.0.0", &None).is_err());
        assert!(validate_bind_policy("0.0.0.0", &Some("t".to_string())).is_ok());
        assert!(validate_bind_policy("127.0.0.1", &None).is_ok());
    }
}
