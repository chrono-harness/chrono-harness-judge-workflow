//! Integration certificate binding admission owned by a proved predicate.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

use vstd::contrib::exec_spec::ExecSpecEq;

#[cfg(verus_keep_ghost)]
#[path = "certificate.rs"]
pub mod producer;

verus! {

/// Wire order for the fields compared before reconstructing the producer's results.
pub fn binding_fields() -> (fields: [&'static str; 8])
    ensures fields[0]@ == "base"@, fields[1]@ == "candidate_tree"@,
        fields[2]@ == "registry_digest"@, fields[3]@ == "executables"@,
        fields[4]@ == "tools"@, fields[5]@ == "environment"@,
        fields[6]@ == "effective_inputs"@, fields[7]@ == "required_tests"@,
{
    ["base", "candidate_tree", "registry_digest", "executables",
     "tools", "environment", "effective_inputs", "required_tests"]
}

pub fn first_mismatch<F: Fn(usize) -> bool>(count: usize, matches: F) -> (first: Option<usize>)
    requires forall|i: usize| i < count ==> matches.requires((i,)),
    ensures match first {
        Some(i) => i < count && matches.ensures((i,), false)
            && forall|j: usize| j < i ==> matches.ensures((j,), true),
        None => forall|i: usize| i < count ==> matches.ensures((i,), true),
    },
{
    let mut i = 0usize;
    while i < count
        invariant i <= count,
            forall|j: usize| j < count ==> matches.requires((j,)),
            forall|j: usize| j < i ==> matches.ensures((j,), true),
        decreases count - i,
    {
        if !matches(i) { return Some(i); }
        i += 1;
    }
    None
}

/// Borrow the first producer's value when every observed output is admissible.
pub fn agreed_output<'a, T, V: Fn(usize) -> bool, E: Fn(usize, usize) -> bool>(
    values: &[&'a T], valid: V, equal: E,
)
    -> (result: Option<&'a T>)
    requires
        forall|i: usize| i < values.len() ==> valid.requires((i,)),
        forall|a: usize, b: usize| a + 1 == b && b < values.len() ==> #[trigger] equal.requires((a, b)),
    ensures match result {
        Some(value) => values.len() > 0 && value == values@[0] &&
            (forall|i: usize| i < values.len() ==> valid.ensures((i,), true)) &&
            (forall|a: usize, b: usize| a + 1 == b && b < values.len() ==> #[trigger] equal.ensures((a, b), true)),
        None => values.len() == 0 ||
            (exists|i: usize| i < values.len() && valid.ensures((i,), false)) ||
            (exists|a: usize, b: usize| a + 1 == b && b < values.len() && #[trigger] equal.ensures((a, b), false)),
    },
{
    if values.len() == 0 { return None; }
    let mut i = 0;
    while i < values.len()
        invariant i <= values.len(),
            forall|k: usize| k < values.len() ==> valid.requires((k,)),
            forall|a: usize, b: usize| a + 1 == b && b < values.len() ==> #[trigger] equal.requires((a, b)),
            forall|k: usize| k < i ==> valid.ensures((k,), true),
            forall|a: usize, b: usize| a + 1 == b && b < i ==> #[trigger] equal.ensures((a, b), true),
        decreases values.len() - i,
    {
        if !valid(i) || (i > 0 && !equal(i - 1, i)) { return None; }
        i += 1;
    }
    Some(values[0])
}

/// Select the exact row observed for one judge. Stop on the second match.
pub fn unique_record<F: Fn(usize) -> bool>(count: usize, matches: F) -> (result: Option<usize>)
    requires forall|i: usize| i < count ==> matches.requires((i,)),
    ensures match result {
        Some(index) => index < count && matches.ensures((index,), true)
            && forall|i: usize| i < count && i != index ==> matches.ensures((i,), false),
        None => (forall|i: usize| i < count ==> matches.ensures((i,), false))
            || exists|a: usize, b: usize| a < b < count && matches.ensures((a,), true)
                && matches.ensures((b,), true),
    },
{
    let mut found: Option<usize> = None;
    let mut i = 0usize;
    while i < count
        invariant i <= count,
            forall|k: usize| k < count ==> matches.requires((k,)),
            match found {
                Some(index) => index < i && matches.ensures((index,), true)
                    && forall|k: usize| k < i && k != index ==> matches.ensures((k,), false),
                None => forall|k: usize| k < i ==> matches.ensures((k,), false),
            },
        decreases count - i,
    {
        if matches(i) {
            if let Some(previous) = found {
                proof { assert(previous < i && matches.ensures((previous,), true)); }
                return None;
            }
            found = Some(i);
        }
        i += 1;
    }
    found
}

/// Bind the fields from the selected row to the retained producer and certificate.
pub fn producer_claim(request_matches: bool, mode: Option<&str>, claim: Option<&str>, digest: &str)
    -> (admitted: bool)
    ensures admitted == (request_matches
        && mode.is_some() && mode.unwrap()@ == "integration_run"@
        && claim.is_some() && claim.unwrap()@ == digest@),
{
    match (mode, claim) {
        (Some(mode), Some(claim)) => request_matches
            && <&str>::exec_eq(mode, "integration_run") && <&str>::exec_eq(claim, digest),
        _ => false,
    }
}

/// A retained runner report is consumable only for a successful report status.
pub open spec fn report_status_admissible(status: Seq<char>) -> bool {
    status == "pass"@ || status == "warn"@
}

/// Consume the concrete report status through the proved delivery decision.
pub fn admit_report_status(status: Option<&str>) -> (admitted: bool)
    ensures admitted == match status {
        Some(status) => report_status_admissible(status@),
        None => false,
    },
{
    match status {
        Some(status) => {
            <&str>::exec_eq(status, "pass") || <&str>::exec_eq(status, "warn")
        }
        None => false,
    }
}

pub fn admit_certificate_header(schema: Option<u64>, status: Option<&str>) -> (admitted: bool)
    ensures admitted == (schema == Some(1u64) && status.is_some() && status.unwrap()@ == "passed"@),
{
    schema == Some(1u64) && match status {
        Some(status) => <&str>::exec_eq(status, "passed"),
        None => false,
    }
}

pub fn admit_producer_context(schema: Option<u64>, kind: Option<&str>,
    candidate: Option<&str>, producer: &str) -> (admitted: bool)
    ensures admitted == (schema == Some(2u64)
        && kind.is_some() && kind.unwrap()@ == "integration"@
        && candidate.is_some() && candidate.unwrap()@ == producer@),
{
    schema == Some(2u64) && match (kind, candidate) {
        (Some(kind), Some(candidate)) => <&str>::exec_eq(kind, "integration")
            && <&str>::exec_eq(candidate, producer),
        _ => false,
    }
}

pub fn report_fits(bytes: u64, limit: u64) -> (admitted: bool)
    ensures admitted == (bytes <= limit),
{
    bytes <= limit
}

pub fn report_outputs() -> (fields: [&'static str; 3])
    ensures fields[0]@ == "execution_plan"@, fields[1]@ == "tests"@,
        fields[2]@ == "effective_inputs"@,
{
    ["execution_plan", "tests", "effective_inputs"]
}

pub fn judge_count_matches(records: usize, observations: usize) -> (admitted: bool)
    ensures admitted == (records as int == observations as int + 1),
{
    records > 0 && records - 1 == observations
}

/// A certificate binding is admissible only when its cardinality and exact
/// field-set checks both hold in the concrete adapter.
pub open spec fn binding_admissible(
    field_count: bool,
    exact_fields: bool,
) -> bool {
    field_count && exact_fields
}

/// Consume the adapted binding facts through the proved certificate rule.
pub fn admit_binding(
    field_count: bool,
    exact_fields: bool,
) -> (admitted: bool)
    ensures admitted == binding_admissible(field_count, exact_fields),
{
    field_count && exact_fields
}

}
