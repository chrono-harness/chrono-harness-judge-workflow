use crate::registration::Registry;
use crate::wire::Request;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use std::{
    io::Write,
    process::{Command, Stdio},
};

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
    let mut child = Command::new("chrono-harness")
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
    let end = output
        .stdout
        .iter()
        .position(|b| *b == b'\n')
        .ok_or("missing command result header")?;
    let lengths: Lengths =
        serde_json::from_slice(&output.stdout[..end]).map_err(|e| e.to_string())?;
    let data = &output.stdout[end + 1..];
    if lengths.stdout_length > limit
        || lengths.stderr_length > limit
        || lengths.stdout_length.checked_add(lengths.stderr_length) != Some(data.len())
    {
        return Err("invalid command result lengths".into());
    }
    let (stdout, stderr) = data.split_at(lengths.stdout_length);
    if lengths.exit_code != 0 {
        let diagnostic = std::str::from_utf8(stderr).map_err(|e| e.to_string())?;
        return Err(if diagnostic.is_empty() {
            format!("{name} {command} exited with {}", lengths.exit_code)
        } else {
            diagnostic.trim_end_matches('\n').into()
        });
    }
    Ok(stdout.to_owned())
}
