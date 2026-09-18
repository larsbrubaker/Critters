#!/usr/bin/env bash
# Pre-commit checks for the Critter Stack port. Run from the repo root.
# Mirrors CI: file-length guardrail, tests, fmt, clippy, wasm check.
set -u
failed=0

echo "==> file length guardrail"
cargo test -p critters-core --test file_line_count --quiet || failed=1

echo "==> cargo test --workspace"
cargo test --workspace --quiet || failed=1

echo "==> cargo fmt --check"
cargo fmt --package critters-core --package critters-native --package critters-wasm -- --check || failed=1

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings || failed=1

echo "==> cargo check (wasm32)"
cargo check -p critters-wasm --target wasm32-unknown-unknown || failed=1

if [ "$failed" -ne 0 ]; then
    echo "PRE-COMMIT CHECKS FAILED"
    exit 1
fi
echo "All pre-commit checks passed"
