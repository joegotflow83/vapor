# Token-efficient patterns and worked examples

## Patterns, roughly by payoff

**1. Ask for the narrowest field set that answers the question.** If the user
asks "which instances are running?", `{ instances(state: RUNNING) { items { id } } }`
is the whole answer. Don't add `tags`, `state`, or `instanceType` "in case".

**2. Always pass `limit` on exploratory queries.** `limit: 20` costs one extra
token and caps a potentially unbounded response. Increase only after seeing the shape.

**3. Filter at AWS, not locally.** `instances(state: RUNNING)` transfers only
running instances; fetching everything and filtering in `jq` pays full token cost
for rows you discard.

**4. Use `--format compact`.** Pretty-printing is pure whitespace overhead for a
machine reader.

**5. Batch services into one query.** One round trip, one response envelope.

**6. Alias when you need the same query twice with different filters:**

```graphql
{
  prod: instances(tags: [{key: "Env", value: "prod"}], limit: 10) { items { id } }
  dev:  instances(tags: [{key: "Env", value: "dev"}],  limit: 10) { items { id } }
}
```

**7. Don't page unless you need to.** If `nextToken` is `null` you have
everything. Only follow it when the task genuinely requires the full set.

**8. Introspect before guessing.** Two wrong guesses cost more than one scoped
`__type` query.

## Worked examples

```bash
# Who am I?
vapor read --format compact '{ stsCallerIdentity { account userId arn } }'

# Running instances with their names
vapor read --format compact \
  '{ instances(state: RUNNING, limit: 50)
       { items { id instanceType privateIp tags { key value } } nextToken } }'

# Buckets and whether versioning is on
vapor read --format compact \
  '{ s3Buckets(limit: 50) { items { name region versioning } nextToken } }'

# Alarms currently firing
vapor read --format compact \
  '{ alarms(state: ALARM) { items { name state metric { metricName } } nextToken } }'

# Lambda functions — note functionName, not name
vapor read --format compact \
  '{ lambdaFunctions(limit: 25) { items { functionName runtime lastModified } nextToken } }'

# CloudFormation stacks in a failed state
vapor read --format compact \
  '{ cfnStacks(limit: 25) { items { stackName stackStatus } nextToken } }'

# Recent log events from one stream
vapor read --format compact \
  '{ logEvents(logGroupName: "/aws/lambda/api", limit: 20) { items { timestamp message } nextToken } }'

# Quick account overview in one call
vapor read --format compact '{
  stsCallerIdentity { account }
  instances(state: RUNNING, limit: 25) { items { id instanceType } nextToken }
  dbInstances(limit: 25) { items { id engine status } nextToken }
  s3Buckets(limit: 25) { items { name } nextToken }
}'

# Cross-region check
vapor read --region eu-west-1 --format compact \
  '{ instances(state: RUNNING, limit: 25) { items { id } nextToken } }'
```

## Long sessions

If a session will make many calls, `vapor serve` exposes the same schema over
HTTP (GraphiQL at `/`, endpoint at `POST /graphql`, default `127.0.0.1:4000`).
Note that the server exposes mutations as well as queries — it has no read-only
mode — so prefer `vapor read` for agent-driven work.

```bash
curl -s http://localhost:4000/graphql -H 'Content-Type: application/json' \
  -d '{"query":"{ stsCallerIdentity { account userId arn } }"}'
```

Binding to a non-loopback address requires `--auth-token` (or `VAPOR_AUTH_TOKEN`);
vapor refuses to start otherwise. Request timeout 60s, max body 1 MiB.
