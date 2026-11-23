//! Public evidence from the installed projects judge. Only workflow's inputs are projected.
use crate::query::invoke;
use crate::registration::Registry;
use crate::routes::Execution;
use crate::wire::Request;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

// Projects returns a fully decoded original receipt. Retain its route-owned fields for replay.
#[derive(serde::Serialize, Deserialize)]
pub struct Receipt {
    pub process: crate::wire::Process,
    #[serde(flatten)]
    fields: BTreeMap<String, Value>,
}

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

#[derive(Deserialize)]
struct Reply {
    passed: bool,
    results: Value,
}

pub fn read(req: &Request, registry: &Registry, raw: &Value) -> Result<ProjectEvidence, String> {
    let output = invoke(req, registry, "projects", "read-results-v1", raw)?;
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
    registry: &Registry,
    effective: &Value,
    plan: &Execution,
    results: &ProjectEvidence,
    retained: Option<Value>,
) -> Result<(), String> {
    let input = json!({"request":req,"effective_inputs":effective,"plan":plan,"results":results.wire,"retained":retained});
    let output = invoke(req, registry, "projects", "check-collection-v1", &input)?;
    if output.is_empty() {
        Ok(())
    } else {
        Err("unexpected check-collection-v1 response".into())
    }
}
