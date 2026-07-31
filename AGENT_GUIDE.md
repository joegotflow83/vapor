# vapor — Agent Guide

This guide has moved. The canonical agent-facing documentation now lives in the
skill, so there is a single source that both humans and agent harnesses read:

- **[skills/aws-vapor/SKILL.md](skills/aws-vapor/SKILL.md)** — start here: what
  vapor is, how to invoke `vapor read`, and how to write a query.
- **[skills/aws-vapor/references/prefixes.md](skills/aws-vapor/references/prefixes.md)**
  — query-root naming, including the prefixes you cannot derive.
- **[skills/aws-vapor/references/conventions.md](skills/aws-vapor/references/conventions.md)**
  — naming, the `Page<T>` contract, filters, dates, credentials.
- **[skills/aws-vapor/references/errors.md](skills/aws-vapor/references/errors.md)**
  — the error envelope, exit codes, and what each failure class means.
- **[skills/aws-vapor/references/feature-flags.md](skills/aws-vapor/references/feature-flags.md)**
  — why a valid query can report an unknown field.
- **[skills/aws-vapor/references/examples.md](skills/aws-vapor/references/examples.md)**
  — token-efficient patterns and worked queries.

## Using it in an agent harness

**Claude Code** — install the plugin from this repo; the `aws-vapor` skill loads
on demand. Allowlisting `Bash(vapor read:*)` is safe: `vapor read` rejects any
operation that is not a `query` before AWS credentials are touched.

**Kiro** — run `./scripts/build-kiro-steering.sh` and copy the generated
`kiro/steering/vapor.md` into your workspace's `.kiro/steering/`. It is a
flattened, self-contained build of the same skill; edit the skill and
regenerate rather than editing it directly.

Anything else — point the agent at `skills/aws-vapor/SKILL.md`.
