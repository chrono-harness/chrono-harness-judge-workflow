//! Independent boundary expectations for branch source-kind admission.
#![allow(unexpected_cfgs)]
#[path = "../../judge-workflow/src/verified_branch.rs"]
mod policy;
#[path = "../examples/release_observation.rs"]
mod release_observation;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn publishing_requires_the_original_missing_timestamp_diagnostic_and_error_exit() {
    proof {
        reveal_strlit("0"); reveal_strlit("2");
        reveal_strlit("E_BRANCH_CONTEXT: missing observed_at");
        reveal_strlit("E_BRANCH_CONTEXT: missing branch_started_at");
        reveal_strlit("E_BRANCH_CONTEXT:");
        assert("0"@[0] != "2"@[0]);
        assert("E_BRANCH_CONTEXT: missing branch_started_at"@.len() != "E_BRANCH_CONTEXT: missing observed_at"@.len());
        assert("E_BRANCH_CONTEXT:"@.len() != "E_BRANCH_CONTEXT: missing observed_at"@.len());
    }
    let accepted = release_observation::accepts("2", "E_BRANCH_CONTEXT: missing observed_at");
    let wrong_exit = release_observation::accepts("0", "E_BRANCH_CONTEXT: missing observed_at");
    let wrong_error = release_observation::accepts("2", "E_BRANCH_CONTEXT: missing branch_started_at");
    let truncated = release_observation::accepts("2", "E_BRANCH_CONTEXT:");
    assert(accepted);
    assert(!wrong_exit);
    assert(!wrong_error);
    assert(!truncated);
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn exactly_one_prefix_selects_feature_or_integration() {
    let feature = policy::select_source_kind(true, false, false);
    let integration = policy::select_source_kind(false, true, false);
    assert(matches!(feature, Some(policy::SourceKind::Feature)));
    assert(matches!(integration, Some(policy::SourceKind::Integration)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn zero_or_two_prefixes_are_rejected() {
    let none = policy::select_source_kind(false, false, false);
    let both = policy::select_source_kind(true, true, false);
    assert(none.is_none() && both.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn target_branch_is_rejected_even_with_one_prefix() {
    let feature = policy::select_source_kind(true, false, true);
    let integration = policy::select_source_kind(false, true, true);
    assert(feature.is_none() && integration.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn commit_count_limit_is_inclusive() {
    let zero = policy::commit_count_within_limit(0, 0);
    let exact = policy::commit_count_within_limit(7, 7);
    let over = policy::commit_count_within_limit(8, 7);
    assert(zero && exact && !over);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn commit_count_limit_preserves_large_unsigned_boundaries() {
    let largest = policy::commit_count_within_limit(usize::MAX, usize::MAX);
    let below = policy::commit_count_within_limit(usize::MAX - 1, usize::MAX);
    let over_zero = policy::commit_count_within_limit(1, 0);
    assert(largest && below && !over_zero);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn age_limit_is_inclusive_and_preserves_unsigned_boundaries() {
    let zero = policy::age_within_limit(0, 0);
    let exact = policy::age_within_limit(3_600_000_000_000, 3_600_000_000_000);
    let one_over = policy::age_within_limit(3_600_000_000_001, 3_600_000_000_000);
    let largest = policy::age_within_limit(u64::MAX, u64::MAX);
    let below_largest = policy::age_within_limit(u64::MAX - 1, u64::MAX);
    assert(zero && exact && !one_over && largest && below_largest);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn truncation_requires_a_relevant_endpoint_beyond_the_fork() {
    let dev = policy::truncates_branch_history(true, false, false);
    let candidate = policy::truncates_branch_history(false, true, false);
    let both = policy::truncates_branch_history(true, true, false);
    let unrelated = policy::truncates_branch_history(false, false, false);
    assert(dev && candidate && both && !unrelated);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_boundary_in_fork_history_is_admissible_for_each_endpoint() {
    let dev = policy::truncates_branch_history(true, false, true);
    let candidate = policy::truncates_branch_history(false, true, true);
    let both = policy::truncates_branch_history(true, true, true);
    let neither = policy::truncates_branch_history(false, false, true);
    assert(!dev && !candidate && !both && !neither);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_fork_must_be_reachable_from_both_endpoints() {
    let both = policy::fork_is_common(true, true);
    let only_dev = policy::fork_is_common(true, false);
    let only_candidate = policy::fork_is_common(false, true);
    let neither = policy::fork_is_common(false, false);
    assert(both && !only_dev && !only_candidate && !neither);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observation_times_require_utc_and_allow_equality() {
    let west = policy::is_utc_offset(-1);
    let utc = policy::is_utc_offset(0);
    let east = policy::is_utc_offset(1);
    assert(!west && utc && !east);
    let earlier = policy::current_observation_allowed(-2, -1);
    let equal = policy::current_observation_allowed(-1, -1);
    let later = policy::current_observation_allowed(0, -1);
    assert(!earlier && equal && later);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn signed_age_yields_an_exact_duration_or_a_distinct_context_error() {
    let negative = policy::checked_age_ns(-1);
    let zero = policy::checked_age_ns(0);
    let positive = policy::checked_age_ns(1);
    let largest = policy::checked_age_ns(u64::MAX as i128);
    let too_large = policy::checked_age_ns(u64::MAX as i128 + 1);
    assert(matches!(negative, Err(policy::AgeError::Negative)));
    assert(matches!(zero, Ok(0)));
    assert(matches!(positive, Ok(1)));
    assert(matches!(largest, Ok(value) if value == u64::MAX));
    assert(matches!(too_large, Err(policy::AgeError::OutOfRange)));
}

}
