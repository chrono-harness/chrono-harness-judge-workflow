//! Independent boundary expectations for certificate binding admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_certificate.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(condition: bool) requires condition {
    if !condition { panic!(); }
}

/// The caller must learn which wire key failed first, including the last member.
#[cfg(verus_keep_ghost)]
fn delivery_requires_the_complete_producer_projection() {
    broadcast use vstd::array::group_array_axioms;
    let fields = policy::producer::producer_fields();
    expect(<&str as vstd::contrib::exec_spec::ExecSpecEq>::exec_eq(fields[0], "branch_ref"));
    expect(<&str as vstd::contrib::exec_spec::ExecSpecEq>::exec_eq(fields[1], "fork_point"));
    expect(<&str as vstd::contrib::exec_spec::ExecSpecEq>::exec_eq(fields[2], "branch_started_at"));
    expect(<&str as vstd::contrib::exec_spec::ExecSpecEq>::exec_eq(fields[3], "observed_at"));
    let mismatch = |i: usize| -> (r: bool) ensures r == (i != 1 && i != 3), { i != 1 && i != 3 };
    let first = policy::first_mismatch(4, mismatch);
    proof {
        if let Some(i) = first { if i > 1 { assert(mismatch.ensures((1usize,), true)); } }
        else { assert(mismatch.ensures((1usize,), true)); }
    }
    expect(first == Some(1usize));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_compares_eight_wire_keys_in_certificate_order() {
    let fields = policy::binding_fields();
    proof {
        reveal_strlit("base"); reveal_strlit("candidate_tree");
        reveal_strlit("registry_digest"); reveal_strlit("executables");
        reveal_strlit("tools"); reveal_strlit("environment");
        reveal_strlit("effective_inputs"); reveal_strlit("required_tests");
    }
    assert(fields[0]@ == "base"@);
    assert(fields[1]@ == "candidate_tree"@);
    assert(fields[2]@ == "registry_digest"@);
    assert(fields[3]@ == "executables"@);
    assert(fields[4]@ == "tools"@);
    assert(fields[5]@ == "environment"@);
    assert(fields[6]@ == "effective_inputs"@);
    assert(fields[7]@ == "required_tests"@);

    let same = |_i: usize| -> (r: bool) ensures r, { true };
    expect(policy::first_mismatch(8, same).is_none());
    let multiple = |i: usize| -> (r: bool) ensures r == (i != 2 && i != 5), { i != 2 && i != 5 };
    let first = policy::first_mismatch(8, multiple);
    proof {
        if let Some(i) = first { if i > 2 { assert(multiple.ensures((2usize,), true)); } }
        else { assert(multiple.ensures((2usize,), true)); }
    }
    expect(first == Some(2usize));
    let last = |i: usize| -> (r: bool) ensures r == (i != 7), { i != 7 };
    let tail = policy::first_mismatch(8, last);
    proof { if tail.is_none() { assert(last.ensures((7usize,), true)); } }
    expect(tail == Some(7usize));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn output_requires_an_admitted_first_value_and_all_later_matches() {
    let first = 17u8;
    let other = 31u8;
    let empty: [&u8; 0] = [];
    let one = [&first];
    let many = [&first, &first, &other];
    let same = |_i: usize| -> (r: bool) ensures r, { true };
    let reject_first = |i: usize| -> (r: bool) ensures r == (i != 0), { i != 0 };
    let agree = |_a: usize, _b: usize| -> (r: bool) ensures r, { true };
    let reject_last = |a: usize, b: usize| -> (r: bool)
        ensures r == (!(a == 1 && b == 2)), { !(a == 1 && b == 2) };
    expect(policy::agreed_output(&empty, same, agree).is_none());
    let denied = policy::agreed_output(&one, reject_first, agree);
    proof { if denied.is_some() { assert(reject_first.ensures((0usize,), true)); } }
    expect(denied.is_none());
    let conflict = policy::agreed_output(&many, same, reject_last);
    proof { if conflict.is_some() { assert(reject_last.ensures((1usize, 2usize), true)); } }
    expect(conflict.is_none());
    let duplicate = [&first, &first];
    match policy::agreed_output(&duplicate, same, agree) {
        Some(value) => expect(*value == 17),
        None => expect(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_has_one_row_and_claim_matches_that_producer() {
    let lone = |i: usize| -> (r: bool) ensures r == (i == 1), { i == 1 };
    let any = |_i: usize| -> (r: bool) ensures r, { true };
    let absent = |_i: usize| -> (r: bool) ensures !r, { false };
    expect(policy::unique_record(0, any) == None);
    expect(policy::unique_record(3, absent) == None);
    let middle = policy::unique_record(3, lone);
    proof {
        assert(!lone.ensures((1usize,), false));
        assert forall|a: usize, b: usize| a < b < 3 implies
            !(lone.ensures((a,), true) && lone.ensures((b,), true)) by {}
        if let Some(index) = middle { assert(lone.ensures((index,), true)); }
    }
    expect(middle == Some(1usize));
    let only = policy::unique_record(1, any);
    proof { assert(!any.ensures((0usize,), false)); }
    expect(only == Some(0usize));
    let duplicate = policy::unique_record(2, any);
    proof {
        if let Some(index) = duplicate {
            if index == 0 { assert(any.ensures((1usize,), false)); }
            else { assert(any.ensures((0usize,), false)); }
        }
        reveal_strlit("integration_run");
        reveal_strlit("delivery");
        reveal_strlit("a");
        reveal_strlit("b");
        assert("delivery"@[0] != "integration_run"@[0]);
        assert("a"@[0] != "b"@[0]);
    }
    expect(duplicate == None);
    expect(policy::producer_claim(true, Some("integration_run"), Some("a"), "a"));
    expect(!policy::producer_claim(false, Some("integration_run"), Some("a"), "a"));
    expect(!policy::producer_claim(true, Some("delivery"), Some("a"), "a"));
    expect(!policy::producer_claim(true, None, Some("a"), "a"));
    expect(!policy::producer_claim(true, Some("integration_run"), Some("b"), "a"));
    expect(!policy::producer_claim(true, Some("integration_run"), None, ""));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_binding_is_admitted() {
    let admitted = policy::admit_binding(true, true);
    assert(admitted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_or_duplicate_binding_fields_are_rejected() {
    let missing = policy::admit_binding(false, true);
    let duplicate = policy::admit_binding(true, false);
    assert(!missing && !duplicate);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_invalid_binding_facts_are_rejected() {
    let admitted = policy::admit_binding(false, false);
    assert(!admitted);
}

}

#[cfg(not(verus_keep_ghost))]
#[test]
fn equal_json_reports_preserve_the_first_original() {
    let original = serde_json::json!({"operations":["build"]});
    let duplicate = original.clone();
    let other = serde_json::json!({"operations":[]});
    let null = serde_json::Value::Null;
    let select = |values: &[&serde_json::Value], admit_null: bool| {
        policy::agreed_output(
            values,
            |i| admit_null || !values[i].is_null(),
            |a, b| values[a] == values[b],
        )
        .map(std::ptr::from_ref)
    };

    assert_eq!(
        select(&[&original, &duplicate], true),
        Some(std::ptr::from_ref(&original))
    );
    assert_eq!(select(&[&original, &duplicate, &other], true), None);
    assert_eq!(select(&[], true), None);
    assert_eq!(select(&[&null], false), None);
    assert_eq!(select(&[&null], true), Some(std::ptr::from_ref(&null)));
}
