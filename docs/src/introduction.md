# vapor

vapor is a GraphQL interface over AWS APIs. Instead of memorizing AWS CLI subcommands, you write a
GraphQL query and get back exactly the fields you asked for — as a one-shot CLI command or from a
local GraphQL server with an interactive playground.

This site is the per-service query reference, one page per AWS service, generated directly from
the GraphQL schema (see the sidebar). Each page shows every query (and, for EC2, mutation) that
service exposes, with full argument and return types.

For installation, build instructions, and general usage (`vapor query` / `vapor serve`), see the
[README](https://github.com/joegotflow83/vapor#readme).

## Building with only the services you need

vapor compiles one AWS SDK client per service behind a Cargo feature, so you only pay for what you
use in binary size and build time:

```bash
# Everything
cargo build --release --all-features

# Just the services you need
cargo build --release --features "ec2 s3 lambda rds"
```

Each service page below lists the exact feature flag it needs. See
[FEATURE_FLAGS.md](https://github.com/joegotflow83/vapor/blob/master/FEATURE_FLAGS.md) for feature
groups and other build options.

## Example query

```bash
vapor query '{ s3Buckets { name creationDate } }'
```

Browse the sidebar for the full set of queries available per service.
