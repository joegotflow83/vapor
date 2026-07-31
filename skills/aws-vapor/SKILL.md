---
name: aws-vapor
description: Query live AWS accounts with vapor, a GraphQL CLI that returns exactly the fields you ask for. Use this instead of the AWS CLI for every read of AWS state — checking EC2 instances, S3 buckets, RDS databases, Lambda functions, CloudWatch alarms, IAM, ECS/EKS, costs, stack status, "is prod healthy", "what's running", "who am I in this account". Covers ~102 services through one query language.
---

# Querying AWS with vapor

`vapor read` runs a GraphQL query against a live AWS account and returns JSON
containing **only the fields you selected**. Prefer it over the AWS CLI for all
reads: `aws ec2 describe-instances` spends 2–8 KB of context per instance,
while the equivalent vapor query spends a few dozen tokens.

```bash
vapor read --format compact '{ instances(state: RUNNING, limit: 20) { items { id instanceType privateIp } } }'
```

```json
{"data":{"instances":{"items":[{"id":"i-0abc123","instanceType":"t3.micro","privateIp":"10.0.1.20"}]}}}
```

The field list *is* the request, so the unused 95% of the AWS response never
reaches your context. One syntax covers every service, and one query can span
several services in a single round trip.

## Before the first query in a session

```bash
vapor services
```

Prints the version and the services compiled into this binary, e.g.
`vapor 0.4.0` / `services (26): acm autoscaling cloudformation …`.

- **If the command is not found or the version is below 0.4.0, stop.** Tell the
  user vapor 0.4.0+ is required and that they can install or upgrade from
  <https://github.com/joegotflow83/vapor>. Do not fall back to older
  subcommands and do not silently switch to the AWS CLI without saying so.
- A service missing from that list has no schema in this binary. See
  [references/feature-flags.md](references/feature-flags.md).

Confirm which account you are pointed at before reporting anything account-specific:

```bash
vapor read --format compact '{ stsCallerIdentity { account userId arn } }'
```

## Invocation

```bash
vapor read '<GRAPHQL>' [--region <REGION>] [--format json|compact]
```

- **Always pass `--format compact`.** Pretty-printing is whitespace you pay for.
- **stdout is pure JSON; logs go to stderr.** Piping to `jq` is always safe.
- **Exit 1 means the response carried errors** (the JSON is still printed).
  Check the exit code rather than string-matching the output.
- `--region` overrides the region for one invocation; otherwise the standard AWS
  chain applies (`AWS_REGION`, `AWS_PROFILE`, instance role, SSO).

`vapor read` refuses mutations and subscriptions before any AWS call is made,
which is what makes it safe to run unattended:

```json
{"data":null,"errors":[{"message":"vapor read accepts queries only; got a mutation. …","extensions":{"code":"ReadOnlyViolation"}}]}
```

**Writes are out of scope for this skill.** vapor can perform them via
`vapor query 'mutation { … }'`, but every mutation is a real, immediate,
destructive-by-default AWS call. Never run one to satisfy a read request. If the
user explicitly asks for a write, surface the exact command and let them run it.

## Writing the query

Four rules cover most of it; the details are in
[references/conventions.md](references/conventions.md).

1. **Every list query returns a `Page`, not a bare list** — select
   `items { … }`, and `nextToken` only when you actually intend to page.
   `nextToken: null` means the listing is complete.
2. **Always pass `limit`** on exploratory queries. `limit: 20` costs one token
   and caps an unbounded response.
3. **Filter in the arguments**, not afterwards — filtering happens at AWS, so
   filtered-out rows are never transferred. `instances(state: RUNNING, tags: [{key: "Env", value: "prod"}])`.
4. **Leaf field names are the AWS SDK's own names, camelCased — never
   normalized.** Lambda has `functionName`, not `name`; CloudFormation has
   `stackName`; IAM has `roleName`. The AWS API reference is the field
   reference. Enums are `SCREAMING_SNAKE`.

Query-root names are mostly the service name, but a dozen are not derivable
(`cfn`, `apigwRest`, `apiV2`, `r53`, `dx`, `org`, `dynamo`, `docdb`, `db`,
`beanstalk`, `waf`, `secret`), and several services own bare names the way EC2
does. **Check [references/prefixes.md](references/prefixes.md) before guessing a
query name** — the "did you mean" suggester does not bridge these.

Batch related lookups into one call rather than issuing several:

```bash
vapor read --format compact '{
  stsCallerIdentity { account }
  instances(state: RUNNING, limit: 25) { items { id instanceType } nextToken }
  dbInstances(limit: 25) { items { id engine status } nextToken }
  s3Buckets(limit: 25) { items { name } nextToken }
}'
```

More patterns and worked examples: [references/examples.md](references/examples.md).

## When a query fails

Errors arrive as `errors[].extensions.code`, carrying the **AWS error code** —
branch on that, not on the message. Full taxonomy in
[references/errors.md](references/errors.md).

- `Unknown field "…"` — nothing was called. Either a typo, or the service is not
  compiled into this binary. When the unknown field is on a result type, the
  message lists that type's real fields, so a wrong guess corrects itself.
- `AccessDeniedException` — missing IAM permission. Don't retry.
- `ThrottlingException` — vapor already retried 3×; back off and lower `limit`.

## Discovery

Introspection describes *this* binary exactly, and is cheaper than two wrong guesses.

```bash
# every available top-level query
vapor read --format compact '{ __schema { queryType { fields { name } } } }'

# the selectable fields of one result type
vapor read --format compact '{ __type(name: "Instance") { fields { name type { name kind } } } }'

# liveness, without touching AWS
vapor read '{ placeholder }'     # → "vapor"
```

Scope introspection — ask for `fields { name }` first, then drill into a single
type. Never request a full introspection dump.

Human-readable per-service reference, generated from the live schema:
<https://joegotflow83.github.io/vapor/>.

## Checklist

Before: only answering fields selected · `limit` set · filters in arguments ·
`--format compact` · related lookups batched.

After: exit code checked · `nextToken` checked before assuming the list is complete.
