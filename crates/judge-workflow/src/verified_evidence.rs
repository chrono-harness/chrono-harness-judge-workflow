//! Workflow evidence admission is a proved conjunction over adapted facts.
#![allow(unexpected_cfgs)]

use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

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


/// Stop at the first entry the operating system reports as executable.
pub fn first_executable<F: Fn(usize) -> bool>(
    count: usize, executable: F,
) -> (first: Option<usize>)
    requires forall|i: usize| i < count ==> executable.requires((i,)),
    ensures match first {
        None => forall|i: usize| i < count ==> executable.ensures((i,), false),
        Some(index) => index < count && executable.ensures((index,), true)
            && forall|i: usize| i < index ==> executable.ensures((i,), false),
    },
{
    let mut i = 0;
    while i < count
        invariant i <= count,
            forall|j: usize| j < count ==> executable.requires((j,)),
            forall|j: usize| j < i ==> executable.ensures((j,), false),
        decreases count - i,
    {
        if executable(i) { return Some(i); }
        i += 1;
    }
    None
}

/// Preserve the selected identity on either outcome. The caller reads and
/// hashes only this entry, so a rejected entry cannot fall through to another.
pub fn bind_digest<T>(selected: T, actual: &str, expected: &str) -> (result: Result<T, T>)
    ensures result == if actual@ == expected@ { Ok(selected) } else { Err(selected) },
{
    if <&str>::exec_eq(actual, expected) { Ok(selected) } else { Err(selected) }
}

}
