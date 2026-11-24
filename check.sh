#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
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

prove() {
  chrono-host-bootstrap run-compiler-proof "$scratch" "$1" "${VERUS:-verus}" "$2" ||
    coverage_failure "$1: verifier execution failed"
}

admit_proof() {
  local operation=$1
  shift
  chrono-host-bootstrap collect-compiler-proof "$scratch" "$operation" "${VERUS:-verus}" \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be "$@" ||
    coverage_failure "$operation: required proof execution was not admitted"
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

prove verify.judge-workflow "$src/verified_branch.rs"
prove verify.judge-workflow-evidence "$src/verified_evidence.rs"
prove verify.judge-workflow-execution "$src/verified_execution.rs"
prove verify.judge-workflow-certificate "$src/verified_certificate.rs"
prove verify.judge-workflow-transition "$src/verified_transition.rs"
prove verify.judge-workflow-tests "$tests/verified_branch.rs"
prove verify.judge-workflow-evidence-tests "$tests/verified_evidence.rs"
prove verify.judge-workflow-certificate-tests "$tests/verified_certificate.rs"
prove verify.judge-workflow-transition-tests "$tests/verified_transition.rs"
prove verify.judge-workflow-execution-tests "$tests/verified_execution.rs"

admit_proof verify.judge-workflow "$src/verified_branch.rs"
admit_proof verify.judge-workflow-evidence "$src/verified_evidence.rs"
admit_proof verify.judge-workflow-execution "$src/verified_execution.rs" "$src/verified_response.rs"
admit_proof verify.judge-workflow-certificate "$src/verified_certificate.rs"
admit_proof verify.judge-workflow-transition "$src/verified_transition.rs"
admit_proof verify.judge-workflow-tests "$tests/verified_branch.rs" "$included/verified_branch.rs"
admit_proof verify.judge-workflow-evidence-tests "$tests/verified_evidence.rs" "$included/verified_evidence.rs"
admit_proof verify.judge-workflow-certificate-tests "$tests/verified_certificate.rs" "$included/verified_certificate.rs"
admit_proof verify.judge-workflow-transition-tests "$tests/verified_transition.rs" "$included/verified_transition.rs"
admit_proof verify.judge-workflow-execution-tests "$tests/verified_execution.rs" "$tests/proof_support/judge-workflow/full_short.rs" \
  "$included/verified_execution.rs" "$included/verified_response.rs"
