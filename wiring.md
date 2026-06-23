# Wiring a service end-to-end

This guide explains how an AWS service is connected from the AWS SDK all the way
to a GraphQL field that hits AWS at request time. Follow it whenever you add a
new service or revive one of the existing (but unwired) resolver modules.

## The architecture in one picture

```
main.rs
  └─ load_aws_config()  ──►  SdkConfig
       └─ schema::root::build_schema(&config)
            ├─ .data(<Svc>Client::new(&config))     ← client injected into context
            └─ QueryRoot   = MergedObject(BaseQuery,    <Svc>Query, …)
               MutationRoot = MergedObject(BaseMutation, <Svc>Mutation, …)
                                   │
                                   └─ resolver: ctx.data::<<Svc>Client>()?.describe_…().await?
                                        └─ map SDK structs → GraphQL types (`From` impls)
```

There are two layers per service:

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Client** | `src/aws/<svc>.rs` | Thin wrapper over the AWS SDK: builds requests, paginates, returns SDK structs. Knows nothing about GraphQL. |
| **Resolver** | `src/schema/<svc>/` | GraphQL `#[Object]` query/mutation types + `SimpleObject` output types with `From<sdk::Type>` impls. Pulls the client from context and calls it. |

The two layers are joined in exactly three places: the **module declaration**,
the **roots** (`registry.rs`), and the **client registration** (`root.rs`). Wiring
a service means touching those three spots.

## Prerequisites

Before wiring, the service needs both layers to exist:

- A client in `src/aws/<svc>.rs`, declared (feature-gated) in `src/aws/mod.rs`.
- A resolver module in `src/schema/<svc>/` with at least `mod.rs`, `queries.rs`,
  and `types.rs` (add `mutations.rs` if it has writes).
- A Cargo feature in `Cargo.toml` that enables the underlying `aws-sdk-<svc>`
  crate. The **same feature name** gates the client, the roots, and the client
  registration — keep them identical.

`src/schema/ec2/` is the reference implementation; copy its shape.

## The three wiring steps

Using `ec2` as the worked example.

### 1. Declare the resolver module

`src/schema/mod.rs`:

```rust
pub mod aws;
pub mod ec2;   // ← add this
pub mod root;
```

Without this line the resolver code never compiles — it's invisible to the
crate. (Most `src/schema/<svc>/` dirs are currently in this undeclared state.)

### 2. Add the service to the roots

`src/schema/aws/registry.rs` — add one field to each root, feature-gated:

```rust
#[derive(MergedObject, Default)]
pub struct QueryRoot(
    BaseQuery,
    #[cfg(feature = "ec2")] crate::schema::ec2::queries::Ec2Query,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    BaseMutation,
    #[cfg(feature = "ec2")] crate::schema::ec2::mutations::Ec2Mutation,
);
```

Notes:
- `BaseQuery` / `BaseMutation` keep the roots non-empty when every service
  feature is off. Leave them first.
- The service's query/mutation structs must `#[derive(Default)]` so the
  `MergedObject` root can derive `Default`.
- Field names across all merged objects must be unique. Two services exposing a
  field with the same name will panic at schema build. Prefix generic names
  (e.g. `eksClusters` rather than `clusters`) when collisions are likely.

### 3. Register the client in context

`src/schema/root.rs` — add a feature-gated `.data(...)` so resolvers can find
the client via `ctx.data::<T>()`:

```rust
pub fn build_schema(config: &SdkConfig) -> Schema<QueryRoot, MutationRoot, EmptySubscription> {
    #[allow(unused_mut)]
    let mut builder =
        Schema::build(QueryRoot::default(), MutationRoot::default(), EmptySubscription);

    #[cfg(feature = "ec2")]
    {
        builder = builder.data(crate::aws::ec2::Ec2Client::new(config));
    }

    // add the next service's client here, same pattern …

    builder.finish()
}
```

If you skip this step the field compiles but every request fails at runtime with
a "context data not found" error — that's the symptom of a registered resolver
with no registered client.

## What a resolver looks like

The resolver fetches the typed client and maps SDK structs to GraphQL types.
Keep the mapping in `From` impls in `types.rs` so resolvers stay thin:

```rust
// src/schema/ec2/queries.rs
#[Object]
impl Ec2Query {
    async fn instances(&self, ctx: &Context<'_>, ids: Option<Vec<String>>) -> Result<Vec<Instance>> {
        let ec2 = ctx.data::<Ec2Client>()?;                 // pulled from .data(...)
        let raw = ec2.describe_instances(ids, /* … */).await?;
        Ok(raw.into_iter().map(Instance::from).collect())   // SDK → GraphQL
    }
}
```

```rust
// src/schema/ec2/types.rs
#[derive(SimpleObject, Clone)]
pub struct Instance { pub id: String, pub state: InstanceState, /* … */ }

impl From<aws_sdk_ec2::types::Instance> for Instance {
    fn from(i: aws_sdk_ec2::types::Instance) -> Self { /* field mapping */ }
}
```

Unit-test the `From` impls directly with SDK builders (no AWS calls needed); see
the `#[cfg(test)]` block at the bottom of `src/schema/ec2/types.rs`.

## Verify

```bash
cargo check                       # must compile (this is the only build step we run by default)
cargo run -- query '{ instances { id state az privateIp } }'
cargo run -- serve                # GraphiQL playground at http://localhost:4000/
```

A successful `cargo check` plus a real response from `cargo run -- query …`
(against an account with credentials in the environment) confirms the service is
wired end-to-end.

## Checklist

- [ ] Cargo feature exists and enables `aws-sdk-<svc>`
- [ ] `src/aws/<svc>.rs` client exists and is declared in `src/aws/mod.rs`
- [ ] `src/schema/<svc>/` resolver exists with `From<sdk>` mappings
- [ ] **Step 1:** module declared in `src/schema/mod.rs`
- [ ] **Step 2:** query (and mutation) field added to the roots in `registry.rs`
- [ ] **Step 3:** client registered with `.data(...)` in `root.rs`
- [ ] All three use the **same** `#[cfg(feature = "…")]` name
- [ ] No GraphQL field-name collisions across services
- [ ] `cargo check` is clean; a live query returns data
