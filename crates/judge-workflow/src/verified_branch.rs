//! Branch source-kind admission is a proved workflow policy.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Copy, Debug, PartialEq, Eq))]
pub enum SourceKind {
    Feature,
    Integration,
}

/// A source branch is admissible only when exactly one registered prefix
/// matches and the source is not the target branch.
pub open spec fn source_kind(
    feature_prefix_matches: bool,
    integration_prefix_matches: bool,
    is_target_branch: bool,
) -> Option<SourceKind> {
    if is_target_branch || feature_prefix_matches == integration_prefix_matches {
        None
    } else if feature_prefix_matches {
        Some(SourceKind::Feature)
    } else {
        Some(SourceKind::Integration)
    }
}

pub fn select_source_kind(
    feature_prefix_matches: bool,
    integration_prefix_matches: bool,
    is_target_branch: bool,
) -> (kind: Option<SourceKind>)
    ensures kind == source_kind(
        feature_prefix_matches, integration_prefix_matches, is_target_branch,
    ),
{
    if is_target_branch || feature_prefix_matches == integration_prefix_matches {
        None
    } else if feature_prefix_matches {
        Some(SourceKind::Feature)
    } else {
        Some(SourceKind::Integration)
    }
}

/// A branch is fresh with respect to the registered commit-count limit when
/// its measured distance does not exceed that limit.
pub open spec fn behind_within_limit(behind: usize, max_behind: usize) -> bool {
    behind <= max_behind
}

/// The production branch adapter supplies the measured commit count and the
/// normalized registered limit; the decision itself is proved here.
pub fn commit_count_within_limit(behind: usize, max_behind: usize) -> (allowed: bool)
    ensures allowed == behind_within_limit(behind, max_behind),
{
    behind <= max_behind
}

/// An observed non-negative age is admissible when it is at or below the
/// normalized nanosecond threshold supplied by the external-input adapter.
pub open spec fn age_limit(age_ns: u64, max_age_ns: u64) -> bool {
    age_ns <= max_age_ns
}

/// The production adapter owns time parsing and threshold normalization; the
/// freshness decision is owned by this proved predicate.
pub fn age_within_limit(age_ns: u64, max_age_ns: u64) -> (allowed: bool)
    ensures allowed == age_limit(age_ns, max_age_ns),
{
    age_ns <= max_age_ns
}

/// RFC 3339 parsing supplies the offset in seconds and absolute timestamps.
pub fn is_utc_offset(seconds: i32) -> (valid: bool)
    ensures valid == (seconds == 0),
{
    seconds == 0
}

pub fn current_observation_allowed(current_ns: i128, original_ns: i128) -> (allowed: bool)
    ensures allowed == (current_ns >= original_ns),
{
    current_ns >= original_ns
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub enum AgeError {
    Negative,
    OutOfRange,
}

/// The time library supplies the signed duration. Keep the admitted value exact.
pub fn checked_age_ns(nanos: i128) -> (age: Result<u64, AgeError>)
    ensures
        match age {
            Ok(value) => value as int == nanos as int,
            Err(AgeError::Negative) => nanos < 0,
            Err(AgeError::OutOfRange) => nanos > u64::MAX as i128,
        },
{
    if nanos < 0 {
        Err(AgeError::Negative)
    } else if nanos > u64::MAX as i128 {
        Err(AgeError::OutOfRange)
    } else {
        Ok(nanos as u64)
    }
}

/// A shallow boundary is relevant when it truncates either endpoint after the fork.
pub fn truncates_branch_history(in_dev: bool, in_candidate: bool, in_fork: bool) -> (missing: bool)
    ensures missing == ((in_dev || in_candidate) && !in_fork),
{
    (in_dev || in_candidate) && !in_fork
}

pub fn fork_is_common(in_dev: bool, in_candidate: bool) -> (common: bool)
    ensures common == (in_dev && in_candidate),
{
    in_dev && in_candidate
}

}
