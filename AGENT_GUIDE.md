# vapor — Agent Guide

A guide for AI agents driving `vapor` to inspect and operate AWS accounts.

## What vapor is, and why you should prefer it

vapor is a single binary that exposes ~102 AWS services through **one GraphQL schema**. You
write a GraphQL query; you get back JSON containing **exactly the fields you asked for** and
nothing else.

That last property is the reason to use it instead of the AWS CLI. Compare:

```bash
aws ec2 describe-instances                 # ~2-8 KB of JSON per instance
```

versus

```bash
vapor query '{ instances(state: RUNNING) { items { id instanceType privateIp } } }'
```

```json
{ "data": { "instances": { "items": [
  { "id": "i-0abc123", "instanceType": "t3.micro", "privateIp": "10.0.1.20" }
] } } }
```

The AWS CLI has no field-selection mechanism — `--query` filters *after* the full response has
already been printed into your context if you pipe it, and you still have to know the JMESPath
shape. With vapor the field list *is* the request, so the unused 95% of the AWS response never
reaches you.

Practical consequences for an agent:

- **One tool, one syntax** for every service — no per-service CLI subcommand/flag vocabulary.
- **One round trip for multiple services** — a single query can fetch S3 buckets, running EC2
  instances, and caller identity together.
- **Bounded output** — `limit` caps rows, `nextToken` resumes; you decide how much context to spend.

## Invocation

Two modes. Prefer `vapor query` unless you're making many calls in a session.

### One-shot CLI

```bash
vapor query '<GRAPHQL>' [--region <REGION>] [--format json|compact]
```

- `--format json` (default) pretty-prints. `--format compact` emits a single line — **use
  `compact`**; it costs meaningfully fewer tokens and is just as parseable.
- `--region` overrides the region for this invocation; otherwise the standard AWS resolution
  applies (`AWS_REGION`, profile config, etc.).
- **stdout is pure JSON.** All logs go to stderr, so `vapor query ... | jq` is always safe.
- **Exit code 1 if the GraphQL response contains any errors** (the JSON is still printed on
  stdout, and each error message is repeated on stderr as `Error: <message>`). Exit 0 means
  clean. Check the exit code rather than string-matching.

### Server

```bash
vapor serve [--port 4000] [--bind 127.0.0.1] [--region <REGION>] [--auth-token <TOKEN>]
```

- GraphiQL playground at `/`, endpoint at `POST /graphql`.
- Defaults: port `4000`, bind `127.0.0.1`.
- Auth policy:

  | bind | token set? | behavior |
  |---|---|---|
  | loopback | no | open |
  | loopback | yes | token required |
  | non-loopback | no | **refuses to start, exit 1** |
  | non-loopback | yes | token required |

  Token also readable from `VAPOR_AUTH_TOKEN`. Send `Authorization: Bearer <token>` (scheme
  match is case-insensitive); failures get `401` + `WWW-Authenticate: Bearer` and never reach
  the executor.

```bash
curl -s http://localhost:4000/graphql -H 'Content-Type: application/json' \
  -d '{"query":"{ stsCallerIdentity { account userId arn } }"}'
```

Request timeout is 60s; max body 1 MiB.

## Query conventions

These hold across all services — learn them once and you can use any service without reading
its docs page.

### Naming

Rust snake_case becomes GraphQL **camelCase**: `next_token` → `nextToken`, `instance_type` →
`instanceType`. Enum values are **SCREAMING_SNAKE**: `state: RUNNING`, `state: ALARM`.

Query field names are service-prefixed only where they'd otherwise collide — EC2 owns the bare
names (`instances`, `vpcs`, `subnets`, `volumes`, `snapshots`, `securityGroups`), while other
services prefix (`s3Buckets`, `stsCallerIdentity`, `ecsClusters`).

### Pagination — the `Page` contract

**Every list query returns a `Page<T>`, not a bare list:**

```graphql
{ instances { items { id } nextToken } }
```

```rust
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_token: Option<String>,   // opaque; pass back to resume
}
```

- `items` — the rows.
- `nextToken` — `null` when the listing is complete; otherwise the **AWS SDK's own continuation
  token**, passed straight through with no wrapping or encoding.
- List queries accept `limit: Int` and `nextToken: String`. `limit` truncates the accumulated
  results and stops vapor from fetching further AWS pages.
- Token stability across vapor versions is not guaranteed — don't persist one long-term.

Resuming:

```bash
vapor query '{ instances(limit: 50) { items { id } nextToken } }'
vapor query '{ instances(limit: 50, nextToken: "AAAB...") { items { id } nextToken } }'
```

A handful of small, inherently-unpaginated queries return a plain list instead (e.g.
`keyPairs`, `elasticIps`). If you're unsure, ask for `items` — a schema error tells you it's a
plain list, and the error costs almost nothing.

### Filters

List queries take optional filter arguments that map onto the underlying AWS request, so the
filtering happens **server-side at AWS** — not locally after transfer. Typical shapes:

```graphql
{ instances(state: RUNNING, vpcId: "vpc-0abc", tags: [{ key: "Env", value: "prod" }])
    { items { id } nextToken } }
```

Common arguments: `ids: [String!]`, a `state` enum or string, a scoping id (`vpcId`,
`subnetId`, `volumeId`), and `tags: [TagFilter!]` where `TagFilter` is `{ key, value }`.

### Timestamps

Date/time fields are the `DateTime` scalar, serialized as RFC 3339 (`2026-07-09T00:00:00+00:00`),
always UTC. (Prior to v0.3 these were plain strings.)

### Mutations

Write operations are GraphQL mutations and are **real, immediate AWS calls**:

```graphql
mutation { stopInstances(ids: ["i-0abc123"]) { instanceId previousState currentState } }
mutation { terminateInstances(ids: ["i-0abc123"]) { instanceId currentState } }
```

Treat every mutation as destructive-by-default and confirm with the user before running one.

## Errors

The response follows the GraphQL envelope: `data` plus an `errors` array. vapor preserves the
**AWS error code** on `errors[].extensions.code`:

```json
{"data":null,"errors":[{"message":"not authorized",
  "extensions":{"code":"AccessDeniedException"}}]}
```

Branch on `extensions.code` — it's the AWS code (`AccessDeniedException`,
`ThrottlingException`, `ResourceNotFoundException`, …), far more reliable than the message. When
AWS gives no code (dispatch failures, timeouts), `extensions` is absent and `message` carries the
context.

Common failure classes:

- **`Unknown field "fooBars"`** — either a typo, or *the service isn't compiled into this
  binary* (see below). Not an AWS error; nothing was called.
- **`AccessDeniedException`** — the credentials lack the IAM permission. Don't retry.
- **`ThrottlingException`** — vapor already retries 3× with backoff; if it still surfaces, back
  off and reduce `limit`.

Retries: 3 attempts, 30s per attempt, 60s total per operation, configured in the SDK.

## Feature flags — why a valid query can be "unknown"

Services are **compile-time gated behind Cargo features**. A binary only exposes the schema for
services it was built with. The prebuilt GitHub release binaries ship the `release` group
(26 services), **not** all 102.

So `Unknown field "sagemakerEndpoints"` may mean the query is fine but this binary lacks the
`sagemaker` feature. Verify by introspecting (below) before concluding the query is wrong. The
fix is a rebuild: `cargo build --release --features "ec2 s3 lambda rds"`. Full procedure,
including a step-by-step agent track, in [BUILDING.md](BUILDING.md).

Naming gotchas in the feature list: `kafka` → MSK, `cognitoidentityprovider` → Cognito, `sfn` →
Step Functions, `docdb` → DocumentDB, `cloudwatch` → CloudWatch **and** Logs. `FEATURE_FLAGS.md`
has the full table.

## Discovery — find fields without guessing

Introspection is enabled. This is the cheapest way to learn the exact surface of *this* binary.

List every available top-level query:

```bash
vapor query --format compact '{ __schema { queryType { fields { name } } } }'
```

Get the arguments and return type of one query:

```bash
vapor query --format compact \
  '{ __type(name: "QueryRoot") { fields { name args { name type { name kind } } } } }'
```

Get the selectable fields of a result type before asking for them:

```bash
vapor query --format compact \
  '{ __type(name: "Instance") { fields { name type { name kind ofType { name } } } } }'
```

Liveness / smoke check without touching AWS at all:

```bash
vapor query '{ placeholder }'     # → "vapor"
```

Introspection output can be large. Scope it: ask for `fields { name }` first, then drill into a
single type — don't request a full introspection dump.

Human-readable per-service reference (generated from the live schema, so it can't drift):
<https://joegotflow83.github.io/vapor/>. Runnable, schema-validated examples live in
`docs/examples/*.graphql` in the repo.

## Credentials and region

Standard AWS credential chain — environment variables, `~/.aws/credentials`, instance/task role,
IRSA, SSO. vapor adds no credential handling of its own.

```bash
AWS_PROFILE=prod vapor query '{ stsCallerIdentity { account arn } }'
vapor query --region eu-west-1 '{ instances { items { id } } }'
```

Always confirm which account you're pointed at before mutating:

```bash
vapor query --format compact '{ stsCallerIdentity { account userId arn } }'
```

## Token-efficient patterns

Ordered roughly by payoff.

**1. Ask for the narrowest field set that answers the question.** If the user asks "which
instances are running?", `{ instances(state: RUNNING) { items { id } } }` is the whole answer.
Don't add `tags`, `state`, or `instanceType` "in case".

**2. Always pass `limit` on exploratory queries.** `limit: 20` costs one extra token and caps a
potentially unbounded response. Increase only after seeing the shape.

**3. Filter at AWS, not locally.** `instances(state: RUNNING)` transfers only running instances;
fetching everything and filtering in `jq` pays full token cost for rows you discard.

**4. Use `--format compact`.** Pretty-printing is pure whitespace overhead for a machine reader.

**5. Batch services into one query.** One round trip, one response envelope:

```bash
vapor query --format compact '{
  stsCallerIdentity { account }
  instances(state: RUNNING, limit: 20) { items { id instanceType } nextToken }
  s3Buckets(limit: 20) { items { name region } nextToken }
}'
```

**6. Alias when you need the same query twice with different filters:**

```graphql
{
  prod: instances(tags: [{key: "Env", value: "prod"}], limit: 10) { items { id } }
  dev:  instances(tags: [{key: "Env", value: "dev"}],  limit: 10) { items { id } }
}
```

**7. Don't page unless you need to.** Look at `nextToken`: if it's `null` you have everything.
Only follow it when the task genuinely requires the full set (a count, an exhaustive audit) —
otherwise the first page usually answers the question.

**8. Introspect before guessing.** Two wrong guesses cost more than one scoped `__type` query,
and a wrong guess against a mutation is worse than expensive.

## Worked examples

```bash
# Who am I?
vapor query --format compact '{ stsCallerIdentity { account userId arn } }'

# Running instances with their names
vapor query --format compact \
  '{ instances(state: RUNNING, limit: 50)
       { items { id instanceType privateIp tags { key value } } nextToken } }'

# Buckets and whether versioning is on
vapor query --format compact \
  '{ s3Buckets { items { name region versioning } nextToken } }'

# Alarms currently firing
vapor query --format compact \
  '{ alarms(state: ALARM) { items { name state metric { metricName } } nextToken } }'

# Everything needed for a quick account overview, in one call
vapor query --format compact '{
  stsCallerIdentity { account }
  instances(state: RUNNING, limit: 25) { items { id instanceType } nextToken }
  rdsInstances(limit: 25) { items { id engine status } nextToken }
  s3Buckets(limit: 25) { items { name } nextToken }
}'

# Stop an instance (destructive — confirm with the user first)
vapor query --format compact \
  'mutation { stopInstances(ids: ["i-0abc123"]) { instanceId previousState currentState } }'
```

## Checklist

Before running a vapor query:

- [ ] Only the fields that answer the question are selected.
- [ ] `limit` is set on every list query.
- [ ] Filters are pushed into arguments, not applied afterwards.
- [ ] `--format compact` is set.
- [ ] Related lookups are batched into one query.
- [ ] For a mutation: the account was confirmed via `stsCallerIdentity`, and the user approved.

After:

- [ ] Exit code checked (non-zero ⇒ inspect `errors[].extensions.code`).
- [ ] `nextToken` checked before assuming the list is complete.
