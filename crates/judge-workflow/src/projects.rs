//! Public evidence from the installed projects judge. Only workflow's inputs are projected.
#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
/// check-collection-v1 acknowledges success with an empty response.
pub fn collection_acknowledged(bytes: &[u8]) -> (ok: bool)
    ensures ok == (bytes@.len() == 0),
{
    bytes.is_empty()
}
}

#[cfg(not(verus_keep_ghost))]
use crate::query::invoke;
#[cfg(not(verus_keep_ghost))]
use crate::registration::Registry;
#[cfg(not(verus_keep_ghost))]
use crate::routes::Execution;
#[cfg(not(verus_keep_ghost))]
use crate::wire::Request;
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::collections::BTreeMap;

// Projects returns a fully decoded original receipt. Retain its route-owned fields for replay.
#[cfg(not(verus_keep_ghost))]
#[derive(serde::Serialize, Deserialize)]
pub struct Receipt {
    pub process: crate::wire::Process,
    #[serde(flatten)]
    fields: BTreeMap<String, Value>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct Operation {
    pub operation: String,
    pub status: String,
    pub error: Option<String>,
    pub receipt: Option<Receipt>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct Collection {
    pub schema: String,
    pub status: String,
    pub reports: Vec<Value>,
    pub global_selected: Vec<String>,
    pub required_units: Vec<String>,
    pub tools: Value,
}

#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
pub struct ProjectEvidence {
    pub wire: Value,
    pub results: Results,
    pub passed: bool,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Reply {
    passed: bool,
    results: Value,
}

#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
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
    if collection_acknowledged(&output) {
        Ok(())
    } else {
        Err("unexpected check-collection-v1 response".into())
    }
}
