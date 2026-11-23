//! Public evidence from the installed projects judge. Only workflow's inputs are projected.
use chrono_harness::{CommandSpec, run_process, run_process_observed, wire::Request};
use chrono_judge_registration::Registrations;
use chrono_judge_routes::{Execution, Receipt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Operation {
    pub operation: String,
    pub status: String,
    pub error: Option<String>,
    pub receipt: Option<Receipt>,
}

#[derive(Deserialize)]
pub struct Collection {
    pub schema: String,
    pub status: String,
    pub reports: Vec<Value>,
    pub global_selected: Vec<String>,
    pub required_units: Vec<String>,
    pub tools: Value,
}

#[derive(Deserialize)]
pub struct Results {
    pub plan: String,
    pub scope: Option<String>,
    pub completion: Option<Collection>,
    pub selected: Vec<String>,
    pub tests: BTreeMap<String, String>,
    pub removed: BTreeMap<String, Option<String>>,
    pub executed: Vec<Operation>,
    pub blocked: Vec<Value>,
}

pub struct ProjectEvidence {
    pub wire: Value,
    pub results: Results,
    pub passed: bool,
}

fn invoke(
    req: &Request,
    registry: &Registrations,
    command: &str,
    query: &Value,
) -> Result<Vec<u8>, String> {
    let config = registry.config();
    let tools = config["tools"].as_array().ok_or("missing workflow tools")?;
    let judges = registry.judges()["judges"]
        .as_array()
        .ok_or("missing workflow judges")?;
    let registered = tools
        .iter()
        .find(|tool| tool["id"] == "chrono-judge-projects");
    let bound = judges.iter().find(|judge| judge["id"] == "projects");
    let (program, digest) = if let Some(tool) = registered {
        (
            tool["program"]
                .as_str()
                .ok_or("invalid registered chrono-judge-projects program")?,
            None,
        )
    } else if let Some(judge) = bound {
        (
            judge["executable"]
                .as_str()
                .ok_or("invalid projects judge executable")?,
            Some(
                judge["sha256"]
                    .as_str()
                    .ok_or("missing projects judge checksum")?,
            ),
        )
    } else {
        ("chrono-judge-projects", None)
    };
    let program = if digest.is_some() {
        req.candidate
            .root
            .join(program)
            .into_os_string()
            .into_string()
            .map_err(|_| "invalid projects judge path")?
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
            format!(
                "chrono-judge-projects {command} exited with {}",
                process.exit_code
            )
        } else {
            diagnostic.trim_end_matches('\n').into()
        });
    }
    Ok(process.stdout_bytes)
}

#[derive(Deserialize)]
struct Reply {
    passed: bool,
    results: Value,
}

pub fn read(
    req: &Request,
    registry: &Registrations,
    raw: &Value,
) -> Result<ProjectEvidence, String> {
    let output = invoke(req, registry, "read-results-v1", raw)?;
    let reply: Reply = serde_json::from_slice(&output).map_err(|e| e.to_string())?;
    let results = Results::deserialize(&reply.results).map_err(|e| e.to_string())?;
    Ok(ProjectEvidence {
        wire: reply.results,
        results,
        passed: reply.passed,
    })
}

pub fn check_collection(
    req: &Request,
    registry: &Registrations,
    effective: &Value,
    plan: &Execution,
    results: &ProjectEvidence,
    retained: Option<Value>,
) -> Result<(), String> {
    let input = json!({"request":req,"effective_inputs":effective,"plan":plan,"results":results.wire,"retained":retained});
    let output = invoke(req, registry, "check-collection-v1", &input)?;
    if output.is_empty() {
        Ok(())
    } else {
        Err("unexpected check-collection-v1 response".into())
    }
}
