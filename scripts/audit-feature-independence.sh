#!/usr/bin/env bash
set -uo pipefail

# Verify that every individual Cargo feature flag (and feature group) builds
# standalone with `--no-default-features --features <flag>`. Vapor's design
# intent is that each AWS service feature is independently buildable; this
# catches cases where one feature's code silently depends on another
# feature's module being compiled in too (e.g. a shared type accidentally
# living behind an unrelated feature gate).
#
# Usage:
#   ./scripts/audit-feature-independence.sh > report.txt
#
# Exits non-zero if any feature failed to build standalone.

cd "$(dirname "$0")/.."

features=$(awk '
  /^\[features\]/ { infeat=1; next }
  /^\[/ { infeat=0 }
  infeat && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
    split($0, parts, "=")
    gsub(/[[:space:]]/, "", parts[1])
    if (parts[1] != "default") print parts[1]
  }
' Cargo.toml)

total=0
failed=0
fail_list=()

for feature in $features; do
  total=$((total + 1))
  echo "== $feature ==" >&2
  if output=$(cargo check --no-default-features --features "$feature" --message-format=short 2>&1); then
    echo "PASS  $feature"
  else
    failed=$((failed + 1))
    fail_list+=("$feature")
    echo "FAIL  $feature"
    echo "$output" | grep '^error' | sed 's/^/      /'
  fi
done

echo
echo "== Summary: $((total - failed))/$total standalone builds passed =="
if [[ $failed -gt 0 ]]; then
  echo "Failing features: ${fail_list[*]}"
  exit 1
fi
