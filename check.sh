#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -eq 0 ]]; then
  /bin/sh .chrono-harness/ci/install.sh
  export PATH="$PWD/.chrono-harness/bin:$PATH"
  export CHRONO_CHECK_SOURCE=${CHRONO_CHECK_SOURCE:-local}
  exec .chrono-harness/bin/chrono-harness check
fi
if [[ $# -ne 1 || $1 != --native ]]; then
  echo 'usage: ./check.sh [--native]' >&2
  exit 2
fi
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
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
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" \
  src/lib.rs src/verified_branch.rs src/verified_certificate.rs src/verified_evidence.rs \
  src/verified_execution.rs src/verified_response.rs src/verified_transition.rs src/branch.rs \
  src/certificate.rs src/evidence.rs src/transition.rs src/registration.rs \
  src/routes.rs src/projects.rs ||
  coverage_failure "native library differs from required sources"

src=crates/judge-workflow/src
tests=crates/judge-workflow-tests/tests
included="$tests/../../judge-workflow/src"

chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow.rlib "$src/verified_branch.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-evidence.rlib "$src/verified_evidence.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-execution.rlib "$src/verified_execution.rs" "$src/verified_response.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-certificate.rlib "$src/verified_certificate.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-transition.rlib "$src/verified_transition.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-tests.rlib "$tests/verified_branch.rs" "$included/verified_branch.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-evidence-tests.rlib "$tests/verified_evidence.rs" "$included/verified_evidence.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-certificate-tests.rlib "$tests/verified_certificate.rs" "$included/verified_certificate.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-transition-tests.rlib "$tests/verified_transition.rs" "$included/verified_transition.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/verify.judge-workflow-execution-tests.rlib "$tests/verified_execution.rs" "$tests/proof_support/judge-workflow/full_short.rs" \
  "$included/verified_execution.rs" "$included/verified_response.rs"
