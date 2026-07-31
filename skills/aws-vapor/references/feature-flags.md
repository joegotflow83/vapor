# Feature flags — why a valid query can be "unknown"

Services are **compile-time gated behind Cargo features**. A binary only exposes
the schema for services it was built with. The prebuilt GitHub release binaries
ship the `release` group (~26 services), **not** all ~102.

So `Unknown field "sagemakerEndpoints"` may mean the query is fine but this
binary lacks the `sagemaker` feature.

## Check before concluding the query is wrong

```bash
vapor services
```

Prints the version and every service compiled into the running binary. The list
is generated at build time from the enabled features, so it cannot drift.
Cross-check with introspection if you want the exact query names:

```bash
vapor read --format compact '{ __schema { queryType { fields { name } } } }'
```

## Fixing it

The fix is a rebuild, which is the user's call — surface the command, don't run
a long build unprompted:

```bash
cargo build --release --features "ec2 s3 lambda rds"
```

Full procedure, including a step-by-step agent track, is in the repo's
`BUILDING.md`.

## Naming gotchas in the feature list

| Feature | Service |
|---|---|
| `kafka` | MSK |
| `cognitoidentityprovider` | Cognito |
| `sfn` | Step Functions |
| `docdb` | DocumentDB |
| `cloudwatch` | CloudWatch **and** CloudWatch Logs |
| `elasticloadbalancingv2` | ELBv2 / ALB / NLB |
| `sts` | caller identity |

The repo's `FEATURE_FLAGS.md` has the full table.
