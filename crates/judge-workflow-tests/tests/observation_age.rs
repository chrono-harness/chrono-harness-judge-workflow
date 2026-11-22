use chrono_judge_workflow::check_observation_age;
use serde_json::{Value, json};

fn check(observed: &str, hours: Value) -> Result<(), (u8, String)> {
    let request = json!({
        "context": {
            "branch_started_at": "2026-01-01T00:00:00Z",
            "observed_at": observed,
        },
        "workflow": {"staleness": {"max_age_hours": hours}},
    });
    check_observation_age(serde_json::to_vec(&request).unwrap().as_slice())
}

#[test]
fn accepts_exact_boundary_and_reports_the_original_stale_age() {
    assert_eq!(check("2026-01-01T00:30:00Z", json!(0.5)), Ok(()));
    assert_eq!(
        check("2026-01-01T00:30:00.000000001Z", json!(0.5)),
        Err((1, "E_BRANCH_STALE: age_ns=1800000000001".into()))
    );
}

#[test]
fn distinguishes_invalid_context_from_staleness() {
    for (time, hours, message) in [
        ("2025-12-31T23:59:59Z", json!(1), "negative age"),
        ("2026-01-01T01:00:00+01:00", json!(1), "time must be UTC"),
        ("2026-01-01T00:00:00Z", json!(-1), "invalid age threshold"),
    ] {
        assert_eq!(
            check(time, hours),
            Err((2, format!("E_BRANCH_CONTEXT: {message}")))
        );
    }
    assert_eq!(
        check_observation_age(b"{}".as_slice()),
        Err((2, "E_BRANCH_CONTEXT: missing observed_at".into()))
    );
    let (code, message) = check_observation_age(b"[] trailing".as_slice()).unwrap_err();
    assert_eq!(code, 2);
    assert!(message.starts_with("E_BRANCH_CONTEXT: invalid observation request:"));
}
