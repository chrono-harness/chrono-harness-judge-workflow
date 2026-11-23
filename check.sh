#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/judge-workflow/Cargo.toml
cargo build --locked --manifest-path crates/judge-workflow/Cargo.toml
cargo fmt --check --manifest-path crates/judge-workflow-tests/Cargo.toml
cargo test --locked --manifest-path crates/judge-workflow-tests/Cargo.toml
prove() {
  local label=$1 root=$2
  local output="$scratch/$label.rlib"
  "${VERUS:-verus}" --crate-type=lib --no-cheating --no-external-by-default --num-threads 2 \
    --output-json --compile "--emit=link=$output,dep-info=$output.d" "$root" -o "$output" \
    -Zchecksum-hash-algorithm=sha256 > "$scratch/proof.json" || { cat "$scratch/proof.json" >&2; return 1; }
  cat "$scratch/proof.json"
  jq -e '.verus.version == "0.2026.04.24.f8e1704" and
    .verus.commit == "f8e17045c4ac17a02e566b8997070f2eecc255be" and
    (."verification-results" | .success == true and ."is-verifying-entire-crate" == true and
    ."encountered-error" == false and ."encountered-vir-error" == false and
    .errors == 0 and .verified > 0)' "$scratch/proof.json" >/dev/null
  test -s "$output" && test -s "$output.d"
}

prove verify.judge-workflow crates/judge-workflow/src/verified_branch.rs
prove verify.judge-workflow-evidence crates/judge-workflow/src/verified_evidence.rs
prove verify.judge-workflow-execution crates/judge-workflow/src/verified_execution.rs
prove verify.judge-workflow-certificate crates/judge-workflow/src/verified_certificate.rs
prove verify.judge-workflow-transition crates/judge-workflow/src/verified_transition.rs
prove verify.judge-workflow-tests crates/judge-workflow-tests/tests/verified_branch.rs
prove verify.judge-workflow-evidence-tests crates/judge-workflow-tests/tests/verified_evidence.rs
prove verify.judge-workflow-certificate-tests crates/judge-workflow-tests/tests/verified_certificate.rs
prove verify.judge-workflow-transition-tests crates/judge-workflow-tests/tests/verified_transition.rs
prove verify.judge-workflow-execution-tests crates/judge-workflow-tests/tests/verified_execution.rs
