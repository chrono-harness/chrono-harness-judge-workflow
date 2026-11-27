//! Independent boundary expectations for workflow evidence admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_evidence.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "installed_judge.rs"]
mod installed;

#[cfg(verus_keep_ghost)]
use policy::evidence::{InputIdentity, input_identity};

verus! {
#[cfg(verus_keep_ghost)]
fn effective_input_markers_select_only_their_own_digest() {
    let mode = input_identity(false,false);
    assert(matches!(mode,Some(InputIdentity::Endpoints)));
    let mode = input_identity(false,true);
    assert(matches!(mode,Some(InputIdentity::Endpoints)));
    let mode = input_identity(true,true);
    assert(matches!(mode,Some(InputIdentity::SchemaAndEndpoints)));
    let mode = input_identity(true,false);
    assert(matches!(mode,None));
}




#[cfg(verus_keep_ghost)]
fn query_keeps_both_raw_streams_and_requires_the_complete_declared_payload() {
    let data = [65u8, 0, 10, 255];
    let result = policy::query::split_payload(&data, 2, 2, 2);
    assert(result.is_some());
    let (out, err) = result.unwrap();
    assert(out@ == seq![65u8, 0u8] && err@ == seq![10u8, 255u8]);
    let success = policy::query::selected_stream(0, out, err);
    let failure = policy::query::selected_stream(7, out, err);
    let signal = policy::query::selected_stream(-1, out, err);
    assert(success == policy::query::CommandReply::Success(out));
    assert(failure == policy::query::CommandReply::Rejected(err));
    assert(signal == policy::query::CommandReply::Rejected(err));
    let truncated = policy::query::split_payload(&data, 2, 1, 4);
    let over = policy::query::split_payload(&data, 3, 1, 2);
    let overflow = policy::query::split_payload(&data, usize::MAX, 1, usize::MAX);
    assert(truncated.is_none() && over.is_none() && overflow.is_none());
    let first_newline = policy::query::header_end(&data);
    assert(first_newline == Some(2usize));
    let incomplete = policy::query::header_end(&[255u8, 0]);
    assert(incomplete.is_none());
}

#[cfg(verus_keep_ghost)]
fn empty_collection_acknowledgement_and_complete_session_line_are_distinct() {
    let empty = policy::projects::collection_acknowledged(&[]);
    let extra = policy::projects::collection_acknowledged(&[10u8]);
    let complete = policy::session::complete_reply(&[0u8, 10]);
    let partial = policy::session::complete_reply(&[10u8, 0]);
    let eof = policy::session::complete_reply(&[]);
    assert(empty && !extra && complete && !partial && !eof);
}

#[cfg(verus_keep_ghost)]
fn route_and_registration_bind_their_own_request_and_frame() {
    let route = policy::routes::context_matches("a", "a", true);
    let changed = policy::routes::context_matches("a", "a", false);
    let other = policy::routes::context_matches("a", "b", true);
    let ordinary = policy::registration::reply_matches("a", Some("a"), true);
    let missing = policy::registration::reply_matches("a", None, true);
    let wrong_commits = policy::registration::reply_matches("a", Some("a"), false);
    proof { reveal_strlit("a"); reveal_strlit("b"); assert("a"@[0] != "b"@[0]); }
    assert(route && !changed && !other && ordinary && !missing && !wrong_commits);
    let base = policy::facts::endpoint_name(policy::facts::Endpoint::Base);
    let candidate = policy::facts::endpoint_name(policy::facts::Endpoint::Candidate);
    assert(base@ == "base"@ && candidate@ == "candidate"@);
}

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


}

#[cfg(not(verus_keep_ghost))]
#[test]
fn mock_runner_output_preserves_error_precedence_and_the_original_command_streams() {
    use chrono_judge_workflow::decode_query_result;
    use std::{
        os::unix::process::ExitStatusExt,
        process::{ExitStatus, Output},
    };

    let reply = |host_exit, written: Result<(), String>, bytes: &[u8], host_err: &[u8]| {
        decode_query_result(
            "chrono-judge-projects",
            "read-results-v1",
            4,
            written,
            Output {
                status: ExitStatus::from_raw(host_exit << 8),
                stdout: bytes.to_vec(),
                stderr: host_err.to_vec(),
            },
        )
    };
    assert_eq!(
        reply(2, Err("write failed".into()), b"", b"timed out\n"),
        Err("timed out".into())
    );
    assert_eq!(
        reply(0, Err("write failed".into()), b"", b""),
        Err("write failed".into())
    );
    assert_eq!(
        reply(0, Ok(()), b"{}", b""),
        Err("missing command result header".into())
    );
    let frame = |code, stdout: usize, stderr: usize, bytes: &[u8]| {
        let mut row = format!(
            "{{\"exit_code\":{code},\"stdout_length\":{stdout},\"stderr_length\":{stderr}}}\n"
        )
        .into_bytes();
        row.extend_from_slice(bytes);
        row
    };
    assert_eq!(
        reply(0, Ok(()), &frame(0, 3, 2, &[0, 10, 255, 254, 0]), b""),
        Ok(vec![0, 10, 255])
    );
    assert_eq!(
        reply(0, Ok(()), &frame(9, 1, 4, b"xerr\n"), b""),
        Err("err".into())
    );
    assert_eq!(
        reply(0, Ok(()), &frame(9, 1, 0, b"x"), b""),
        Err("chrono-judge-projects read-results-v1 exited with 9".into())
    );
    assert!(
        reply(0, Ok(()), &frame(9, 0, 1, &[255]), b"")
            .unwrap_err()
            .contains("utf-8")
    );
    for (stdout, stderr, bytes) in [
        (2, 1, &b"ab"[..]),
        (0, 0, &b"x"[..]),
        (5, 0, &b"12345"[..]),
        (0, 5, &b"12345"[..]),
    ] {
        assert_eq!(
            reply(0, Ok(()), &frame(0, stdout, stderr, bytes), b""),
            Err("invalid command result lengths".into())
        );
    }
}
