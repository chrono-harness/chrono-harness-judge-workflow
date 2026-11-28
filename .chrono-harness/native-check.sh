#!/usr/bin/env bash
set -euo pipefail
scratch=${CHRONO_NATIVE_SCRATCH:?expected the directory allocated by chrono-host-bootstrap}
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/judge-workflow/Cargo.toml
product=crates/judge-workflow/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/build.jsonl"
cargo fmt --check --manifest-path crates/judge-workflow-tests/Cargo.toml
cargo test --locked --manifest-path crates/judge-workflow-tests/Cargo.toml
coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

# Read the exact library selected by Cargo in the build of the tested executable.
stem=$(chrono-host-bootstrap select-native-cargo-library "$(pwd -P)/$product" "$scratch/build.jsonl") ||
  coverage_failure "native library was not identified"
chrono-host-bootstrap check-compiler-sources native "$stem" \
  src/lib.rs src/verified_branch.rs src/verified_certificate.rs src/verified_evidence.rs \
  src/verified_execution.rs src/verified_response.rs src/verified_transition.rs src/branch.rs \
  src/certificate.rs src/evidence.rs src/transition.rs src/registration.rs \
  src/routes.rs src/projects.rs ||
  coverage_failure "native library differs from required sources"

# CI binds the ten Verus receipts to their registered sources in FILEMAP.
