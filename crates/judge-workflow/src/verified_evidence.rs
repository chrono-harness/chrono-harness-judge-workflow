//! Workflow evidence admission is a proved conjunction over adapted facts.
#![allow(unexpected_cfgs)]

use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "installed_judge.rs"]
pub mod installed;
#[cfg(verus_keep_ghost)]
#[path = "wire.rs"]
pub mod wire;

#[cfg(verus_keep_ghost)]
#[path = "evidence.rs"]
pub mod evidence;

#[cfg(verus_keep_ghost)]
#[path = "facts.rs"]
pub mod facts;
#[cfg(verus_keep_ghost)]
#[path = "projects.rs"]
pub mod projects;
#[cfg(verus_keep_ghost)]
#[path = "query.rs"]
pub mod query;
#[cfg(verus_keep_ghost)]
#[path = "registration.rs"]
pub mod registration;
#[cfg(verus_keep_ghost)]
#[path = "routes.rs"]
pub mod routes;
#[cfg(verus_keep_ghost)]
#[path = "session.rs"]
pub mod session;

verus! {

/// The JSON/process adapters may observe these facts, but this conjunction is
/// the sole owner of the evidence admission decision.
pub open spec fn record_admissible(
    state_executed: bool,
    judge_identity: bool,
    request_identity: bool,
    request_digest: bool,
    successful_exit: bool,
    protocol_status: bool,
    response_bytes: bool,
) -> bool {
    state_executed
        && judge_identity
        && request_identity
        && request_digest
        && successful_exit
        && protocol_status
        && response_bytes
}

/// Combine adapted response/process observations into the proved admission
/// result consumed by the workflow producer.
pub fn admit_record(
    state_executed: bool,
    judge_identity: bool,
    request_identity: bool,
    request_digest: bool,
    successful_exit: bool,
    protocol_status: bool,
    response_bytes: bool,
) -> (admitted: bool)
    ensures admitted == record_admissible(
        state_executed,
        judge_identity,
        request_identity,
        request_digest,
        successful_exit,
        protocol_status,
        response_bytes,
    ),
{
    state_executed
        && judge_identity
        && request_identity
        && request_digest
        && successful_exit
        && protocol_status
        && response_bytes
}


/// Preserve the selected identity on either outcome. The caller reads and
/// hashes only this entry, so a rejected entry cannot fall through to another.
pub fn bind_digest<T>(selected: T, actual: &str, expected: &str) -> (result: Result<T, T>)
    ensures result == if actual@ == expected@ { Ok(selected) } else { Err(selected) },
{
    if <&str>::exec_eq(actual, expected) { Ok(selected) } else { Err(selected) }
}

}
