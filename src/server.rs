use async_graphql::http::GraphiQLSource;
use async_graphql_axum::GraphQL;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use async_graphql::Schema;
use crate::schema::aws::registry::{QueryRoot, MutationRoot};

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

pub async fn run_server(schema: Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>, port: u16) {
    let app = Router::new()
        .route_service("/graphql", GraphQL::new(schema))
        .route("/", get(graphiql));

    let addr = format!("0.0.0.0:{port}");
    println!("GraphiQL playground: http://localhost:{port}/");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
