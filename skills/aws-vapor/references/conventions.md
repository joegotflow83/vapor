# Schema conventions

These hold across all ~102 services, so learning them once removes the need to
read any per-service page.

## Naming

Rust snake_case becomes GraphQL **camelCase**: `next_token` → `nextToken`,
`instance_type` → `instanceType`. Enum values are **SCREAMING_SNAKE**:
`state: RUNNING`, `state: ALARM`.

**Leaf field names are the AWS SDK's own names, camelCased — never normalized.**
If the AWS API calls it `FunctionName`, vapor calls it `functionName`, not
`name`. A type exposes a plain `name` only where AWS itself does (EKS clusters,
CodeArtifact domains); Lambda has `functionName`, CloudFormation `stackName`,
IAM `roleName`, DynamoDB `tableName`. There are no renames anywhere in the
schema, across all ~400 types: **the AWS API reference is the field reference.**

Query-root names are covered separately in [prefixes.md](prefixes.md).

## Pagination — the `Page` contract

Every list query returns a `Page<T>`, not a bare list:

```graphql
{ instances(limit: 50) { items { id } nextToken } }
```

- `items` — the rows.
- `nextToken` — `null` when the listing is complete; otherwise the AWS SDK's own
  continuation token, passed straight through with no wrapping or encoding.
- List queries accept `limit: Int` and `nextToken: String`. `limit` truncates the
  accumulated results *and* stops vapor from fetching further AWS pages.
- Token stability across vapor versions is not guaranteed — don't persist one.

Resuming:

```bash
vapor read --format compact '{ instances(limit: 50) { items { id } nextToken } }'
vapor read --format compact '{ instances(limit: 50, nextToken: "AAAB...") { items { id } nextToken } }'
```

Don't page unless the task genuinely needs the full set (a count, an exhaustive
audit). If `nextToken` is `null` you already have everything.

A handful of small, inherently-unpaginated queries return a plain list instead
(e.g. `keyPairs`, `elasticIps`). If unsure, ask for `items` — the schema error
tells you it's a plain list and costs almost nothing.

## Filters

Filter arguments map onto the underlying AWS request, so filtering happens
**server-side at AWS**, not locally after transfer:

```graphql
{ instances(state: RUNNING, vpcId: "vpc-0abc", tags: [{ key: "Env", value: "prod" }])
    { items { id } nextToken } }
```

Common arguments: `ids: [String!]`, a `state` enum or string, a scoping id
(`vpcId`, `subnetId`, `volumeId`), and `tags: [TagFilter!]` where `TagFilter` is
`{ key, value }`.

## Timestamps

Date/time fields use the `DateTime` scalar, serialized as RFC 3339
(`2026-07-09T00:00:00+00:00`), always UTC.

## Credentials and region

The standard AWS credential chain — environment variables,
`~/.aws/credentials`, instance/task role, IRSA, SSO. vapor adds no credential
handling of its own.

```bash
AWS_PROFILE=prod vapor read --format compact '{ stsCallerIdentity { account arn } }'
vapor read --region eu-west-1 --format compact '{ instances(limit: 20) { items { id } } }'
```

## Mutations are not part of this skill

Writes exist as GraphQL mutations and are real, immediate AWS calls. `vapor read`
refuses them. They are available through `vapor query`, which should be run by
the user, not by an agent satisfying a read request.
