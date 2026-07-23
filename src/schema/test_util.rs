//! Shared scaffolding for `src/schema/*/queries.rs` resolver-layer tests
//! (Priority 4's "resolver layer" tranche, plan-3 test coverage). Builds a
//! single-service `Schema` around one query root object, wired to a
//! `StaticReplayClient`-backed AWS client via `crate::aws::test_util::sdk_config`
//! — `schema.execute("{ ... }").await` then exercises the real resolver plus
//! the real client's request-building/response-parsing with no network I/O,
//! the same mocking mechanism `src/aws/test_util.rs` uses for per-client
//! tests, one layer up the call stack.
//!
//! Only resolvers with actual logic beyond a bare passthrough to a single AWS
//! client call need dedicated tests here (fan-outs, filters, id-vs-discovery
//! branches, partial-failure handling) — a resolver that just forwards its
//! args to one client method and maps the result 1:1 is already covered by
//! that client method's own `src/aws/*.rs` test module.
#![allow(unused_imports, dead_code)]

use async_graphql::{EmptyMutation, EmptySubscription, ObjectType, Schema, SchemaBuilder};

use crate::schema::aws::registry::BaseQuery;

/// Wraps `query` in a minimal single-service schema (no mutations/
/// subscriptions) so a GraphQL query string can exercise just that one
/// resolver set. Call `.data(some_client)` on the returned builder — once per
/// AWS client the resolvers under test pull via `ctx.data::<T>()` — before
/// `.finish()`.
pub(crate) fn build_query_schema<Q>(query: Q) -> SchemaBuilder<Q, EmptyMutation, EmptySubscription>
where
    Q: ObjectType + 'static,
{
    Schema::build(query, EmptyMutation, EmptySubscription)
}

/// Same as `build_query_schema` but for a mutation root. Async-graphql
/// requires a query root even for a mutation-only test schema — there's no
/// `EmptyQuery` type, so this reuses the crate's own `BaseQuery` (a bare
/// liveness field) rather than inventing a throwaway query object per test.
pub(crate) fn build_mutation_schema<M>(
    mutation: M,
) -> SchemaBuilder<BaseQuery, M, EmptySubscription>
where
    M: ObjectType + 'static,
{
    Schema::build(BaseQuery, mutation, EmptySubscription)
}
