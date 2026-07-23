# Migration: 0.2.x → 0.3.0 (schema v2 — resumable pagination + typed timestamps)

This is a breaking GraphQL schema release. Every list-returning query and
every date/time field changed shape. There is no server-side compatibility
mode — update client queries before upgrading.

## 1. List queries now return a page, not a bare list

Every query that used to return `[T!]!` now returns a generic `XPage` type
with `items` and `nextToken`:

```graphql
type XPage {
  items: [X!]!
  nextToken: String
}
```

`limit` behavior is unchanged (still an optional cap on how many items come
back); what's new is `nextToken`, both as an input argument (to resume) and
an output field (opaque — pass it back verbatim, don't parse it). Token
strings are the underlying AWS SDK's own continuation tokens passed through
as-is; they are not guaranteed stable across vapor versions.

**Before:**

```graphql
query {
  s3Buckets(limit: 50) {
    name
  }
}
```

**After:**

```graphql
query {
  s3Buckets(limit: 50) {
    items {
      name
    }
    nextToken
  }
}
```

To page through everything, keep re-issuing the query with `nextToken`
from the previous response until it comes back `null`:

```graphql
query {
  s3Buckets(limit: 50, nextToken: "<token from previous response>") {
    items { name }
    nextToken
  }
}
```

Omitting `limit` entirely still drains every page server-side and returns
`nextToken: null`, same as before this release.

A small number of resolvers that only ever describe caller-supplied IDs
(e.g. EC2 `keyPairs`/`elasticIps`, ECS `ecsClusters`/`ecsServices` when
given explicit ARNs) have no underlying AWS pagination and are unaffected —
they still return a bare list.

## 2. Timestamp fields are now typed, not strings

Every date/time field that used to be a plain `String` is now the
`DateTime` scalar (RFC 3339 / ISO 8601 on the wire), e.g.:

```graphql
# Before
type S3Bucket {
  creationDate: String
}

# After
type S3Bucket {
  creationDate: DateTime
}
```

Query syntax for selecting the field is unchanged; only the returned value's
format changes, from whatever string AWS happened to format it as (usually
already RFC 3339, but not guaranteed) to a strict RFC 3339 string, e.g.
`"2026-07-14T12:00:00Z"`. Clients that parsed these fields as opaque strings
and passed them back unmodified are unaffected; clients that did their own
date parsing against an assumed format should switch to a standard RFC 3339
parser if they haven't already.

A few fields that AWS itself represents as opaque strings on the wire (not a
real timestamp type in the SDK — e.g. Cost Explorer's `timePeriodStart`/
`timePeriodEnd`, EC2's `Image.creationDate`) were intentionally left as
`String`; those are unchanged by this release.

## 3. Nothing else changed

Field names, argument names, and non-list/non-timestamp field types are
unchanged. This is purely the pagination-wrapper + timestamp-typing sweep;
see `specs/plan-2-schema-v2-pagination-timestamps.md` for the full design
rationale.
