# Errors

Responses follow the GraphQL envelope: `data` plus an `errors` array. vapor
preserves the **AWS error code** on `errors[].extensions.code`:

```json
{"data":null,"errors":[{"message":"not authorized",
  "extensions":{"code":"AccessDeniedException"}}]}
```

Branch on `extensions.code` — it is the AWS code (`AccessDeniedException`,
`ThrottlingException`, `ResourceNotFoundException`, …) and is far more reliable
than the message. When AWS gives no code (dispatch failures, timeouts),
`extensions` is absent and `message` carries the context.

Exit code 1 means the response carried at least one error. The JSON is still
printed on stdout; each message is repeated on stderr as `Error: <message>`.

## Common failure classes

**`Unknown field "fooBars"`** — nothing was called; this is a schema error, not
an AWS error. Either a typo, or *the service isn't compiled into this binary*
(see [feature-flags.md](feature-flags.md)). When the unknown field is on a
*result* type rather than at the query root, the message ends with that type's
complete field list — `Unknown field "name" on type "LambdaFunction". Fields on
LambdaFunction: functionName, runtime, …` — so a wrong guess corrects itself
without a separate introspection round trip.

**`ReadOnlyViolation`** — `vapor read` was handed a mutation or subscription and
refused before contacting AWS. This is not a transient failure and must not be
worked around by switching to `vapor query`; surface the command to the user
instead.

**`AccessDeniedException`** — the credentials lack the IAM permission. Don't
retry; report which principal was used (`stsCallerIdentity`) and what was denied.

**`ThrottlingException`** — vapor already retries 3× with backoff. If it still
surfaces, back off and reduce `limit`.

**`ResourceNotFoundException`** — the id or name doesn't exist in this region or
account. Check the region before concluding the resource is gone.

## Retry behaviour

3 attempts, 30s per attempt, 60s total per operation, configured in the AWS SDK.
Don't add your own retry loop on top of a non-transient code.
