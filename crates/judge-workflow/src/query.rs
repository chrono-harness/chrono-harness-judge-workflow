#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
pub enum CommandReply<'a> { Success(&'a [u8]), Rejected(&'a [u8]) }

pub fn selected_stream<'a>(exit: i32, stdout: &'a [u8], stderr: &'a [u8]) -> (stream: CommandReply<'a>)
    ensures stream == if exit == 0 { CommandReply::Success(stdout) } else { CommandReply::Rejected(stderr) },
{
    if exit == 0 { CommandReply::Success(stdout) } else { CommandReply::Rejected(stderr) }
}

/// Locate the first protocol separator. Payload bytes may contain further newlines.
pub fn header_end(frame: &[u8]) -> (end: Option<usize>)
    ensures match end {
        Some(i) => i < frame.len() && frame@[i as int] == 10u8
            && forall|j: int| 0 <= j < i ==> frame@[j] != 10u8,
        None => forall|j: int| 0 <= j < frame.len() ==> frame@[j] != 10u8,
    },
{
    for i in 0..frame.len()
        invariant forall|j: int| 0 <= j < i ==> frame@[j] != 10u8,
    {
        if frame[i] == 10 { return Some(i); }
    }
    None
}

/// Admit two complete, individually bounded streams and return the original slices.
pub fn split_payload<'a>(data: &'a [u8], stdout: usize, stderr: usize, limit: usize)
    -> (out: Option<(&'a [u8], &'a [u8])>)
    ensures out.is_some() == (stdout <= limit && stderr <= limit
        && stdout as int + stderr as int == data.len() as int),
        match out {
            Some((first,last)) => first@ == data@.take(stdout as int)
                && last@ == data@.skip(stdout as int),
            None => true,
        },
{
    if stdout > limit || stderr > limit || stdout > data.len() || stderr != data.len() - stdout {
        return None;
    }
    Some(data.split_at(stdout))
}
}

#[cfg(not(verus_keep_ghost))]
use crate::registration::Registry;
#[cfg(not(verus_keep_ghost))]
use crate::wire::Request;
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use serde_json::json;
#[cfg(not(verus_keep_ghost))]
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[cfg(not(verus_keep_ghost))]
pub(crate) fn invoke(
    req: &Request,
    registry: &Registry,
    judge_id: &str,
    command: &str,
    query: &Value,
) -> Result<Vec<u8>, String> {
    run(
        req,
        &registry.config,
        &registry.judges,
        judge_id,
        command,
        query,
    )
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn run(
    req: &Request,
    config: &Value,
    judges: &Value,
    judge_id: &str,
    command: &str,
    query: &Value,
) -> Result<Vec<u8>, String> {
    let name = format!("chrono-judge-{judge_id}");
    let tools = config["tools"].as_array().ok_or("missing workflow tools")?;
    let judges = judges["judges"]
        .as_array()
        .ok_or("missing workflow judges")?;
    let registered = tools.iter().find(|tool| tool["id"] == name);
    let bound = judges.iter().find(|judge| judge["id"] == judge_id);
    let (program, digest) = if let Some(tool) = registered {
        (
            tool["program"]
                .as_str()
                .ok_or_else(|| format!("invalid registered {name} program"))?,
            None,
        )
    } else if let Some(judge) = bound {
        (
            judge["executable"]
                .as_str()
                .ok_or_else(|| format!("invalid {judge_id} judge executable"))?,
            Some(
                judge["sha256"]
                    .as_str()
                    .ok_or_else(|| format!("missing {judge_id} judge checksum"))?,
            ),
        )
    } else {
        (name.as_str(), None)
    };
    let program = if digest.is_some() {
        req.candidate
            .root
            .join(program)
            .into_os_string()
            .into_string()
            .map_err(|_| format!("invalid {judge_id} judge path"))?
    } else {
        program.to_owned()
    };
    let limit = config["protocol"]["stdout_limit_bytes"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or("invalid protocol output bound")?;
    let environment: std::collections::BTreeMap<String, String> =
        serde_json::from_value(req.observations["environment"]["effective"].clone())
            .map_err(|e| e.to_string())?;
    let timeout = config["protocol"]["timeout_seconds"]
        .as_u64()
        .ok_or("invalid protocol timeout")?;
    let mut header = json!({"program":program,"args":[command],"env":environment,"timeout_seconds":timeout,"output_limit_bytes":limit});
    if let Some(digest) = digest {
        header["expected_sha256"] = json!(digest);
    }
    let mut child = Command::new(req.runner_path()?)
        .arg("run-command-v1")
        .current_dir(&req.candidate.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().unwrap();
    let written = (|| -> Result<(), String> {
        serde_json::to_writer(&mut input, &header).map_err(|e| e.to_string())?;
        input.write_all(b"\n").map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut input, query).map_err(|e| e.to_string())
    })();
    drop(input);
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    decode_query_result(&name, command, limit, written, output)
}

/// Decode the runner's outer status and its complete delegated command result.
#[cfg(not(verus_keep_ghost))]
pub fn decode_query_result(
    name: &str,
    command: &str,
    limit: usize,
    written: Result<(), String>,
    output: std::process::Output,
) -> Result<Vec<u8>, String> {
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .trim_end_matches('\n')
            .into());
    }
    written?;
    #[derive(Deserialize)]
    struct Lengths {
        exit_code: i32,
        stdout_length: usize,
        stderr_length: usize,
    }
    let end = header_end(&output.stdout).ok_or("missing command result header")?;
    let lengths: Lengths =
        serde_json::from_slice(&output.stdout[..end]).map_err(|e| e.to_string())?;
    let data = &output.stdout[end + 1..];
    let (stdout, stderr) = split_payload(data, lengths.stdout_length, lengths.stderr_length, limit)
        .ok_or("invalid command result lengths")?;
    match selected_stream(lengths.exit_code, stdout, stderr) {
        CommandReply::Success(bytes) => Ok(bytes.to_owned()),
        CommandReply::Rejected(bytes) => {
            let diagnostic = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
            Err(if diagnostic.is_empty() {
                format!("{name} {command} exited with {}", lengths.exit_code)
            } else {
                diagnostic.trim_end_matches('\n').into()
            })
        }
    }
}
