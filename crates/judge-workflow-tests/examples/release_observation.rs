//! Admission of the error observed from the release binary's observation-age probe.
#![allow(unexpected_cfgs)]
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn accepts(exit: &str, diagnostic: &str) -> (accepted: bool)
    ensures accepted == (exit@ == "2"@ && diagnostic@ == "E_BRANCH_CONTEXT: missing observed_at"@),
{
    <&str>::exec_eq(exit, "2") && <&str>::exec_eq(diagnostic, "E_BRANCH_CONTEXT: missing observed_at")
}
}

#[cfg(not(verus_keep_ghost))]
fn main() -> std::process::ExitCode {
    let values: Vec<_> = std::env::args().skip(1).collect();
    if let [exit, diagnostic] = values.as_slice() {
        if accepts(exit, diagnostic) {
            return std::process::ExitCode::SUCCESS;
        }
        eprintln!("Unexpected release observation: exit {exit}, diagnostic {diagnostic:?}");
    } else {
        eprintln!("usage: release_observation EXIT DIAGNOSTIC");
    }
    std::process::ExitCode::FAILURE
}
