use crate::facts;
use crate::verified_branch::{self, SourceKind};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("E_BRANCH_CONTEXT: missing {k}"))
}
fn moment(v: &Value, k: &str) -> Result<OffsetDateTime, String> {
    let d = OffsetDateTime::parse(text(v, k)?, &Rfc3339)
        .map_err(|e| format!("E_BRANCH_CONTEXT: {k}: {e}"))?;
    if !verified_branch::is_utc_offset(d.offset().whole_seconds()) {
        return Err("E_BRANCH_CONTEXT: time must be UTC".into());
    }
    Ok(d)
}
/// Registered observation freshness, shared with the local round producer.
/// The caller supplies the observation; this predicate never reads a clock.
pub fn observation_age(ctx: &Value, w: &Value) -> Result<time::Duration, String> {
    age_at(ctx, w, moment(ctx, "observed_at")?)
}
/// Check the local context producer's original timestamps with the same policy as the judge.
pub fn check_observation_age(input: impl std::io::Read) -> Result<(), (u8, String)> {
    let request: Value = serde_json::from_reader(input).map_err(|error| {
        (
            2,
            format!("E_BRANCH_CONTEXT: invalid observation request: {error}"),
        )
    })?;
    observation_age(&request["context"], &request["workflow"])
        .map(|_| ())
        .map_err(|message| {
            let code = if message.starts_with("E_BRANCH_STALE:") {
                1
            } else {
                2
            };
            (code, message)
        })
}
/// Admission can observe now without changing the original common context.
pub fn current_observation_age(
    ctx: &Value,
    w: &Value,
    observation: &Value,
) -> Result<time::Duration, String> {
    let nanos = observation["unix_timestamp_nanos"]
        .as_str()
        .ok_or("E_BRANCH_CONTEXT: current observation missing")?
        .parse()
        .map_err(|_| "E_BRANCH_CONTEXT: invalid current observation")?;
    let current = OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|e| e.to_string())?;
    let original = moment(ctx, "observed_at")?;
    if !verified_branch::current_observation_allowed(nanos, original.unix_timestamp_nanos()) {
        return Err("E_BRANCH_CONTEXT: current observation precedes original production".into());
    }
    age_at(ctx, w, current)
}
fn age_at(ctx: &Value, w: &Value, observed: OffsetDateTime) -> Result<time::Duration, String> {
    let age = observed - moment(ctx, "branch_started_at")?;
    let age_ns = verified_branch::checked_age_ns(age.whole_nanoseconds()).map_err(|reason| {
        match reason {
            verified_branch::AgeError::Negative => "E_BRANCH_CONTEXT: negative age",
            verified_branch::AgeError::OutOfRange => {
                "E_BRANCH_CONTEXT: age exceeds supported range"
            }
        }
        .to_string()
    })?;
    let hours = w["staleness"]["max_age_hours"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("E_BRANCH_CONTEXT: invalid age threshold")?;
    let threshold_ns = hours * 3_600_000_000_000.0;
    // The JSON number is an external input. Normalize it once at the adapter
    // boundary; the admission decision itself is the proved Verus predicate.
    let max_age_ns = if threshold_ns >= u64::MAX as f64 {
        u64::MAX
    } else {
        threshold_ns as u64
    };
    if !verified_branch::age_within_limit(age_ns, max_age_ns) {
        return Err(format!("E_BRANCH_STALE: age_ns={}", age_ns));
    }
    Ok(age)
}
/// Fixed Git/context inputs only. No branch discovery, wall clock or commit-date inference.
pub fn branch(root: &Path, candidate: &str, ctx: &Value, w: &Value) -> Result<Value, String> {
    let reader = facts::Reader::legacy(root)?;
    let result = with_reader(&reader, candidate, ctx, w);
    reader.finish()?;
    result
}
pub fn with_reader(
    reader: &facts::Reader,
    candidate: &str,
    ctx: &Value,
    w: &Value,
) -> Result<Value, String> {
    let dev = text(ctx, "dev_tip")?;
    let fork = text(ctx, "fork_point")?;
    for oid in [candidate, dev, fork] {
        reader
            .verify_oid(oid)
            .map_err(|e| format!("E_HISTORY_MISSING: {e}"))?;
    }
    if ctx["base"] != dev {
        return Err("E_BRANCH_CONTEXT: base/dev mismatch".into());
    }
    let source = text(ctx, "branch_ref")?;
    reader
        .call::<()>(json!({"op":"check-branch-name","branch":source}))
        .map_err(|e| format!("E_BRANCH_CONTEXT: invalid Git branch name: {e}"))?;
    let feature_prefix = text(w, "feature_prefix")?;
    let integration_prefix = text(w, "integration_prefix")?;
    let feature_matches = source
        .strip_prefix(feature_prefix)
        .is_some_and(|tail| !tail.is_empty());
    let integration_matches = source
        .strip_prefix(integration_prefix)
        .is_some_and(|tail| !tail.is_empty());
    let kind = verified_branch::select_source_kind(
        feature_matches,
        integration_matches,
        source == text(w, "target_branch")?,
    )
    .ok_or("E_BRANCH_CONTEXT: missing or ambiguous registered source branch")?;
    let kind_name = match kind {
        SourceKind::Feature => "feature",
        SourceKind::Integration => "integration",
    };
    let age = observation_age(ctx, w)?;
    let dev_ancestors = reader.call::<BTreeSet<String>>(json!({"op":"ancestors","commit":dev}))?;
    let branch_ancestors =
        reader.call::<BTreeSet<String>>(json!({"op":"ancestors","commit":candidate}))?;
    // Shallow boundaries are acceptable only at/before the proven fork. Any other
    // truncated relevant history prevents a complete count or ancestry decision.
    let fork_ancestors =
        reader.call::<BTreeSet<String>>(json!({"op":"ancestors","commit":fork}))?;
    let boundaries: Vec<String> = reader.call(json!({"op":"shallow-boundaries"}))?;
    for s in &boundaries {
        if verified_branch::truncates_branch_history(
            dev_ancestors.contains(s),
            branch_ancestors.contains(s),
            fork_ancestors.contains(s),
        ) {
            return Err("E_HISTORY_MISSING: truncated ancestry beyond fork".into());
        }
    }
    if !verified_branch::fork_is_common(
        dev_ancestors.contains(fork),
        branch_ancestors.contains(fork),
    ) {
        return Err("E_BRANCH_CONTEXT: fork is not a common ancestor".into());
    }
    let behind = dev_ancestors.difference(&fork_ancestors).count();
    let count = w["staleness"]["max_behind_commits"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("E_BRANCH_CONTEXT: invalid count threshold")?;
    // The JSON/floating-point threshold is an external input adapter. Once
    // validated and normalized, the admission decision is owned by Verus.
    let max_behind = count as usize;
    if !verified_branch::commit_count_within_limit(behind, max_behind) {
        return Err(format!(
            "E_BRANCH_STALE: behind={behind}; age_ns={}",
            age.whole_nanoseconds()
        ));
    }
    Ok(
        json!({"kind":kind_name,"branch_ref":source,"fork_point":fork,"behind_commits":behind,"age_seconds":age.as_seconds_f64(),"branch_started_at":ctx["branch_started_at"],"observed_at":ctx["observed_at"]}),
    )
}
