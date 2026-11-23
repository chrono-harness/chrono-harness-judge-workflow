use chrono_harness::{CommandSpec, run_process, run_process_observed, wire::Request};
use chrono_judge_registration::Registrations;
use serde_json::Value;

pub(crate) fn invoke(
    req: &Request,
    registry: &Registrations,
    judge_id: &str,
    command: &str,
    query: &Value,
) -> Result<Vec<u8>, String> {
    let name = format!("chrono-judge-{judge_id}");
    let config = registry.config();
    let tools = config["tools"].as_array().ok_or("missing workflow tools")?;
    let judges = registry.judges()["judges"]
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
    let spec = CommandSpec {
        program,
        args: vec![command.into()],
        env: serde_json::from_value(req.observations["environment"]["effective"].clone())
            .map_err(|e| e.to_string())?,
        timeout_seconds: config["protocol"]["timeout_seconds"]
            .as_u64()
            .ok_or("invalid protocol timeout")?,
        output_limit_bytes: config["protocol"]["stdout_limit_bytes"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or("invalid protocol output bound")?,
    };
    let input = serde_json::to_vec(query).map_err(|e| e.to_string())?;
    let process = if let Some(digest) = digest {
        run_process_observed(&req.candidate.root, &spec, &input, digest)?
    } else {
        run_process(&req.candidate.root, &spec, &input)?
    };
    if let Some(cause) = process.failure {
        return Err(cause);
    }
    if process.exit_code != 0 {
        let diagnostic = String::from_utf8(process.stderr_bytes).map_err(|e| e.to_string())?;
        return Err(if diagnostic.is_empty() {
            format!("{name} {command} exited with {}", process.exit_code)
        } else {
            diagnostic.trim_end_matches('\n').into()
        });
    }
    Ok(process.stdout_bytes)
}
