//! Independent boundary expectations for workflow evidence admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_evidence.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(value: bool) requires value { if !value { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_required_fact_admits_the_record() {
    let admitted = policy::admit_record(true, true, true, true, true, true, true);
    assert(admitted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_missing_fact_rejects_the_record() {
    let state = policy::admit_record(false, true, true, true, true, true, true);
    let judge = policy::admit_record(true, false, true, true, true, true, true);
    let request = policy::admit_record(true, true, false, true, true, true, true);
    let digest = policy::admit_record(true, true, true, false, true, true, true);
    let exit = policy::admit_record(true, true, true, true, false, true, true);
    let status = policy::admit_record(true, true, true, true, true, false, true);
    let response = policy::admit_record(true, true, true, true, true, true, false);
    assert(!state && !judge && !request && !digest && !exit && !status && !response);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_false_and_single_boundary_facts_remain_rejected() {
    let all_false = policy::admit_record(false, false, false, false, false, false, false);
    let first_only = policy::admit_record(true, false, false, false, false, false, false);
    let state_missing = policy::admit_record(false, true, true, true, true, true, true);
    assert(!all_false && !first_only && !state_missing);
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn path_order_stops_at_the_first_executable_and_checks_its_digest() {
    let available = |i: usize| -> (result: bool)
        ensures result == (i == 1 || i == 2), { i == 1 || i == 2 };
    expect(policy::first_executable(0, available).is_none());
    expect(policy::first_executable(1, available).is_none());
    let first = match policy::first_executable(3, available) {
        Some(i) => {
            proof { if i == 2 { assert(available.ensures((1usize,), false)); } }
            expect(i == 1);
            i
        },
        None => {
            proof { assert(available.ensures((1usize,), false)); }
            expect(false);
            return;
        },
    };
    proof { reveal_strlit("aa"); reveal_strlit("ab"); assert("aa"@[1] != "ab"@[1]); }
    let digest = if first == 1 { "aa" } else { "ab" };
    match policy::bind_digest(first, digest, "ab") {
        Err(i) => expect(i == 1),
        Ok(_) => expect(false),
    }
    match policy::bind_digest(first, digest, "aa") {
        Ok(i) => expect(i == 1),
        Err(_) => expect(false),
    }
}

}
