//! GraphQL request tracing and mutation audit logging.
//!
//! This server executes AWS-mutating operations (e.g. `terminateInstances`,
//! `runInstances`) with its own AWS credentials and previously left no trace
//! of what was invoked. `AuditLog` wraps every GraphQL request in a single
//! tracing span and, for any request containing a mutation operation, emits
//! an INFO-level log line with the operation name and its arguments.

use std::sync::Arc;

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextParseQuery, NextRequest,
};
use async_graphql::parser::types::{ExecutableDocument, OperationType};
use async_graphql::{Response, ServerResult, Variables};
use tracing::Instrument;

pub struct AuditLog;

impl ExtensionFactory for AuditLog {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(AuditLogExtension)
    }
}

struct AuditLogExtension;

#[async_trait::async_trait]
impl Extension for AuditLogExtension {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        next.run(ctx)
            .instrument(tracing::info_span!("graphql_request"))
            .await
    }

    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let document = next.run(ctx, query, variables).await?;
        let has_mutation = document
            .operations
            .iter()
            .any(|(_, op)| op.node.ty == OperationType::Mutation);
        if has_mutation {
            tracing::info!(
                target: "vapor::audit",
                query = %ctx.stringify_execute_doc(&document, variables),
                "mutation executed"
            );
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{EmptySubscription, Object, Schema};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    /// Minimal `Subscriber` that only counts `target: "vapor::audit"` events
    /// — enough to assert presence/absence without pulling in a test-only
    /// tracing crate for a single call site.
    #[derive(Default)]
    struct AuditCapture {
        count: AtomicUsize,
    }

    impl Subscriber for AuditCapture {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _span: &Id, _values: &Record<'_>) {}
        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
        fn event(&self, event: &Event<'_>) {
            if event.metadata().target() == "vapor::audit" {
                self.count.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _span: &Id) {}
        fn exit(&self, _span: &Id) {}
    }

    struct Query;

    #[Object]
    impl Query {
        async fn ping(&self) -> &str {
            "pong"
        }
    }

    struct Mutation;

    #[Object]
    impl Mutation {
        async fn set_value(&self, value: String) -> String {
            value
        }
    }

    fn schema() -> Schema<Query, Mutation, EmptySubscription> {
        Schema::build(Query, Mutation, EmptySubscription)
            .extension(AuditLog)
            .finish()
    }

    /// Runs `body` with a capturing subscriber installed for the entire
    /// async execution (a scoped `with_default(|| ...)` closure only covers
    /// the synchronous part of building the future, not subsequent `.await`
    /// polls) and returns how many `vapor::audit` events fired.
    async fn audit_event_count(
        body: impl std::future::Future<Output = async_graphql::Response>,
    ) -> usize {
        let capture = std::sync::Arc::new(AuditCapture::default());
        let guard = tracing::subscriber::set_default(capture.clone());
        let response = body.await;
        drop(guard);
        assert!(
            response.errors.is_empty(),
            "unexpected errors: {:?}",
            response.errors
        );
        capture.count.load(Ordering::SeqCst)
    }

    #[tokio::test]
    async fn query_does_not_emit_audit_log() {
        let schema = schema();
        let count = audit_event_count(schema.execute("{ ping }")).await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn mutation_emits_audit_log() {
        let schema = schema();
        let count = audit_event_count(schema.execute(r#"mutation { setValue(value: "x") }"#)).await;
        assert_eq!(count, 1);
    }
}
