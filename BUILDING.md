# Building vapor

How to compile your own `vapor` binary. Two audiences: a walkthrough for humans, and a
deterministic procedure for AI agents at the end.

You need to build your own binary when the prebuilt GitHub release doesn't cover a service you
use. Releases ship the curated **`release` feature group (~26 services)**, not all ~102 — see
[FEATURE_FLAGS.md](FEATURE_FLAGS.md) for the full list.

## Prerequisites

- **Rust, stable toolchain.** The crate is edition 2021 and pins no MSRV; CI builds on
  `stable`. Install via <https://rustup.rs>.
- **A C linker/toolchain** — `build-essential` on Debian/Ubuntu, Xcode CLT on macOS, MSVC build
  tools on Windows.
- **Disk and memory.** See [Build cost](#build-cost) — a `--all-features` build is genuinely
  heavy.
- The AWS CLI, *only* if you want to run `scripts/detect-aws-services.sh`.

No AWS credentials are needed to build; they're needed only at runtime.

## The short version

```bash
git clone https://github.com/joegotflow83/vapor.git
cd vapor
cargo build --release --features "ec2 s3 lambda rds"
# → ./target/release/vapor
```

Then verify:

```bash
./target/release/vapor query '{ placeholder }'
# → {"data":{"placeholder":"vapor"}}   (no AWS call, no credentials required)
```

## Choosing features

This is the only real decision in the build. Every AWS service is behind its own Cargo feature;
the schema only exposes services compiled in. A query against an omitted service fails with
`Unknown field`, so **pick the features before you build, not after**.

| You want | Command |
|---|---|
| The default trio (`ec2`, `s3`, `lambda`) | `cargo build --release` |
| Exactly these services | `cargo build --release --no-default-features --features "ec2 rds sqs"` |
| A named group | `cargo build --release --no-default-features --features release` |
| Everything (~102 services) | `cargo build --release --all-features` |

Notes:

- **Use `--no-default-features` when you list features explicitly.** Otherwise `ec2`, `s3`, and
  `lambda` come along whether or not you asked, silently inflating the build.
- Features are space-separated in one quoted string, and are additive across groups:
  `--features "basic monitoring"` works.
- Feature groups available: `basic`, `web`, `data`, `monitoring`, `devops`, `release`. Defined
  at the bottom of `Cargo.toml`.
- A few features pull in others — `kafka` (MSK) enables `ec2` because broker AZ enrichment needs
  `describe_subnets`; `cloudwatch` brings `cloudwatchlogs`.
- Feature names aren't always the service's marketing name: `sfn` → Step Functions, `docdb` →
  DocumentDB, `cognitoidentityprovider` → Cognito, `kafka` → MSK, `inspector2` → Inspector v2.

### Let your account choose for you

```bash
./scripts/detect-aws-services.sh                 # current default region
./scripts/detect-aws-services.sh --all-regions   # every enabled region (slower)
AWS_PROFILE=my-profile ./scripts/detect-aws-services.sh
```

It scans via the Resource Groups Tagging API and prints a ready-to-run `cargo build` command
with the matching features. Best-effort: untagged resources and some global/free services
(bare IAM policies, STS) may not surface, so cross-check against FEATURE_FLAGS.md.

## Build cost

The release profile is tuned for a small, fast binary rather than a fast build —
`codegen-units = 1`, `lto = true`, `strip = true` (`Cargo.toml`). Combined with ~100 optional
AWS SDK crates, a wide build is expensive:

- **`--all-features` is memory-hungry enough to OOM rustc** on smaller machines. If you hit
  that, cap parallelism: `cargo build --release --all-features -j 1`. It is much slower but
  survives on constrained hosts.
- Narrow builds are dramatically cheaper. There is no reason to build all ~102 services unless
  you're generating docs.
- Set `CARGO_PROFILE_DEV_DEBUG=0` for debug builds if disk is tight; CI does.

## The second binary: `gen-docs`

The crate has two targets: `vapor` (the CLI) and `gen-docs` (regenerates the per-service mdBook
pages from the live schema). `gen-docs` names every service's schema module unconditionally, so
it only compiles with all of them enabled — hence `required-features = ["docs"]` in
`Cargo.toml`, which makes Cargo *skip* the target rather than fail when those features are off.

Practical implications:

- Normal partial builds just work; Cargo skips `gen-docs` silently.
- Adding `--bin vapor` to a partial build makes that explicit and is what the release workflow
  does — harmless, and it removes any ambiguity about what's being compiled.
- To regenerate docs you need the lot: `cargo run --all-features --bin gen-docs`.

## Cross-compiling / matching the official releases

The release workflow (`.github/workflows/cd.yml`) builds this exact command per target:

```bash
cargo build --release --no-default-features --features release --bin vapor --target <TARGET>
```

Targets shipped: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
`x86_64-pc-windows-msvc`. Add a target locally with `rustup target add <TARGET>` (cross-linking
to another OS needs a suitable linker; building natively per platform is simpler).

## Verifying a build

```bash
cargo fmt --all --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
```

Two things worth knowing:

- **`cargo test` and `cargo clippy` need `--all-features`** — the `gen-docs` tests don't exist
  without it.
- `./scripts/audit-feature-independence.sh` checks that every feature builds standalone under
  `--no-default-features --features <flag>`. Run it if you add a service; it catches code that
  accidentally depends on an unrelated feature's module.

To confirm which services your binary actually has:

```bash
./target/release/vapor query --format compact '{ __schema { queryType { fields { name } } } }'
```

## Installing

```bash
cargo install --path . --no-default-features --features release   # → ~/.cargo/bin/vapor
```

Or just copy `target/release/vapor` onto your `PATH`. It's a single static-ish binary with no
runtime dependencies beyond system TLS.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Query fails with `Unknown field "fooBars"` | Service not compiled in | Rebuild with that feature |
| rustc killed / OOM during build | `--all-features` parallelism | Add `-j 1`, or build fewer features |
| ~75 `E0433` errors about missing modules | Building `gen-docs` without `docs` | Add `--bin vapor`, or use `--all-features` |
| `cargo test` fails on missing `gen_docs` items | Missing `--all-features` | `cargo test --all-features` |
| Link errors on a fresh machine | No C toolchain | Install build-essential / Xcode CLT / MSVC tools |
| Binary works but every query is `AccessDenied` | Build is fine; it's credentials | Check `AWS_PROFILE`, run `vapor query '{ stsCallerIdentity { arn } }'` |

---

## For an AI agent

Follow this as a decision procedure. Do not run a build speculatively — a wide build can take
tens of minutes and exhaust memory.

**Step 1 — Establish whether a rebuild is actually needed.** A missing service is the usual
trigger. Confirm it against the binary in hand rather than assuming:

```bash
vapor query --format compact '{ __schema { queryType { fields { name } } } }'
```

If the field you need is absent, the service isn't compiled in. If it's present, the problem is
the query or the credentials — do not rebuild.

**Step 2 — Resolve services to feature names.** Grep the authoritative list rather than guessing;
several names don't match the service's marketing name:

```bash
grep -n "sagemaker\|sfn\|kafka" FEATURE_FLAGS.md
```

**Step 3 — Build the narrowest thing that works.** Always `--no-default-features` plus an
explicit list, so nothing rides along:

```bash
cargo build --release --no-default-features --features "ec2 s3 rds sagemaker" --bin vapor
```

Prefer `--features release` (the curated 26) over `--all-features` when the user wants "the
usual stuff". Reserve `--all-features` for regenerating docs.

**Step 4 — Run it in the background and don't block on it.** Release builds here exceed any
reasonable foreground timeout. Launch with `run_in_background`, then report the result when it
lands. If the build is killed, retry once with `-j 1` before concluding anything is broken.

**Step 5 — Verify without touching AWS.**

```bash
./target/release/vapor query '{ placeholder }'                                     # binary runs
./target/release/vapor query --format compact \
  '{ __schema { queryType { fields { name } } } }'                                 # services present
```

Check the second output for the specific field that motivated the rebuild. That closes the loop;
an AWS call is not needed to prove the build succeeded.

**Constraints to respect in this repo:** `CLAUDE.md` forbids running `cargo build` or `cargo test`
unless the user explicitly asks — a build request like the above counts, a hunch does not. For
type-checking only, use `cargo check -j 1 --all-features --tests` (per `AGENTS.md`). Also note
`AGENTS.md`: the shell aliases `ls` with `--color`, so use `command ls` or `find` when piping
filenames.
