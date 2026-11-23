//! Public route evidence used by workflow. The installed routes judge verifies the plan and receipts.
use crate::query;
use crate::registration::Registry;
use chrono_harness::{observation::Tool, wire::Request};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

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
#[derive(Serialize, Deserialize)]
pub struct Method {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(flatten)]
    rest: BTreeMap<String, Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub method: Method,
    pub predecessors: BTreeSet<String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

#[derive(Deserialize)]
pub struct Context {
    request_id: String,
    pub binding: Value,
    pub expanded: BTreeMap<String, Vec<String>>,
}

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
    if actual.request_id != original.request_id
        || !actual
            .expanded
            .keys()
            .map(String::as_str)
            .eq(templates.keys().copied())
    {
        return Err("E_WORKFLOW_EVIDENCE: route context query identity".into());
    }
    Ok(actual)
}

pub fn collection(
    req: &Request,
    registry: &Registry,
    original: &Execution,
    current: &Execution,
) -> Result<Value, String> {
    let input = json!({"root":original.root,"binding":original.binding,"selected":current.selected.keys().collect::<Vec<_>>(),
        "filemap":registry.filemap,"projects":registry.projects,"test_operations":registry.test_operations,"environment":current.environment});
    let output = query::invoke(req, registry, "routes", "collection-plan-v1", &input)?;
    chrono_harness::decode(&output)
}
