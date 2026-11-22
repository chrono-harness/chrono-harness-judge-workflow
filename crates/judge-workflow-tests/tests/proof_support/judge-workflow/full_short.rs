#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn initial_check_needs_failure_copy(exit_code: i32) -> (retain: bool)
    ensures retain == (exit_code != 0),
{
    exit_code != 0
}
}
