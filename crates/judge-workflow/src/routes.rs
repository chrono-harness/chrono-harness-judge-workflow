//! Public route evidence used by workflow. The installed routes judge verifies the plan and receipts.
#![allow(unexpected_cfgs)]
use vstd::contrib::exec_spec::ExecSpecEq;
use vstd::prelude::*;

verus! {
pub fn context_matches(original: &str, actual: &str, expanded_keys_match: bool) -> (ok: bool)
    ensures ok == (original@ == actual@ && expanded_keys_match),
{
    <&str>::exec_eq(original, actual) && expanded_keys_match
}
}

#[cfg(not(verus_keep_ghost))]
use crate::query;
#[cfg(not(verus_keep_ghost))]
use crate::registration::Registry;
#[cfg(not(verus_keep_ghost))]
use crate::wire::{Request, Tool};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    schema: String,
    pub identity: String,
    pub binding: Value,
    pub root: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub selected: BTreeMap<String, Value>,
    pub operations: Vec<Operation>,
    pub tools: BTreeMap<String, Tool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scheduling: Option<Value>,
}

// Route queries validate the complete plan. Workflow reads only the method fields used in its binding.
#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
pub struct Method {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(flatten)]
    rest: BTreeMap<String, Value>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub method: Method,
    pub predecessors: BTreeSet<String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct Context {
    request_id: String,
    pub binding: Value,
    pub expanded: BTreeMap<String, Vec<String>>,
}

#[cfg(not(verus_keep_ghost))]
pub fn context(
    caller: &Request,
    registry: &Registry,
    original: &Request,
    effective: &Value,
    templates: &BTreeMap<&str, &Vec<String>>,
) -> Result<Context, String> {
    let input = json!({"request":original,"effective_inputs":effective,"templates":templates});
    let output = query::invoke(caller, registry, "routes", "plan-context-v1", &input)?;
    let actual: Context = serde_json::from_slice(&output).map_err(|e| e.to_string())?;
    let same_keys = actual
        .expanded
        .keys()
        .map(String::as_str)
        .eq(templates.keys().copied());
    if !context_matches(&original.request_id, &actual.request_id, same_keys) {
        return Err("E_WORKFLOW_EVIDENCE: route context query identity".into());
    }
    Ok(actual)
}

#[cfg(not(verus_keep_ghost))]
pub fn collection(
    req: &Request,
    registry: &Registry,
    original: &Execution,
    current: &Execution,
) -> Result<Value, String> {
    let input = json!({"root":original.root,"binding":original.binding,"selected":current.selected.keys().collect::<Vec<_>>(),
        "filemap":registry.filemap,"projects":registry.projects,"test_operations":registry.test_operations,"environment":current.environment});
    let output = query::invoke(req, registry, "routes", "collection-plan-v1", &input)?;
    req.decode(&output)
}
