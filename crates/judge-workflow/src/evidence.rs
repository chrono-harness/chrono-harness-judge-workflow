#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
#[derive(Copy, Clone)]
pub enum InputIdentity { Endpoints, SchemaAndEndpoints }

pub fn input_identity(present: bool, recognized: bool) -> (mode: Option<InputIdentity>)
    ensures match mode {
        Some(InputIdentity::Endpoints) => !present,
        Some(InputIdentity::SchemaAndEndpoints) => present && recognized,
        None => present && !recognized,
    },
{
    if !present { Some(InputIdentity::Endpoints) }
    else if recognized { Some(InputIdentity::SchemaAndEndpoints) }
    else { None }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::projects::{self, ProjectEvidence};
#[cfg(not(verus_keep_ghost))]
use crate::registration::Registry;
#[cfg(not(verus_keep_ghost))]
use crate::routes::{self, Execution};
#[cfg(not(verus_keep_ghost))]
use crate::verified_evidence;
#[cfg(not(verus_keep_ghost))]
use crate::verified_execution;
#[cfg(not(verus_keep_ghost))]
use crate::wire::{self, Process as ProcessResult, Request, Response, Scope, Status, sha256};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[cfg(not(verus_keep_ghost))]
pub fn output<'a>(req: &'a Request, key: &str) -> Result<&'a Value, String> {
    let values: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get(key))
        .collect();
    crate::verified_certificate::agreed_output(
        &values,
        |i| !values[i].is_null(),
        |a, b| values[a] == values[b],
    )
    .ok_or_else(|| format!("E_WORKFLOW_INPUT: missing/conflicting {key}"))
}
#[cfg(not(verus_keep_ghost))]
pub fn process(req: &Request, p: &ProcessResult) -> Result<(), String> {
    req.evidence(json!({"operation":"check-process","process":p}))
}
#[cfg(not(verus_keep_ghost))]
pub fn record(req: &Request, v: &Value) -> Result<(Response, ProcessResult), String> {
    #[derive(serde::Deserialize)]
    struct Observed {
        record: Value,
        stdout_matches: bool,
    }
    let observed: Observed = req
        .evidence(json!({"operation":"observe-record","record":v}))
        .map_err(|error| match error.strip_prefix("E_RECORD_SHAPE: ") {
            Some(cause) => format!("E_WORKFLOW_EVIDENCE: {cause}"),
            None => error,
        })?;
    let v = &observed.record;
    let r: Response = serde_json::from_value(v["response"].clone())
        .map_err(|e| format!("E_WORKFLOW_EVIDENCE: {e}"))?;
    let p: ProcessResult = serde_json::from_value(v["process"].clone())
        .map_err(|e| format!("E_WORKFLOW_EVIDENCE: {e}"))?;
    let admitted = verified_evidence::admit_record(
        v["state"] == "executed",
        v["id"] == r.judge_id,
        v["request_id"] == r.request_id,
        v["request_digest"] == p.stdin_sha256,
        v["exit_code"] == 0,
        r.status < Status::Fail,
        observed.stdout_matches,
    );
    if !admitted {
        return Err("E_WORKFLOW_EVIDENCE: response/process identity".into());
    }
    Ok((r, p))
}
#[cfg(not(verus_keep_ghost))]
pub fn execution(
    req: &Request,
    registry: &Registry,
    plan: &Execution,
    evidence: &ProjectEvidence,
) -> Result<(), String> {
    execution_inner(req, registry, plan, evidence, false)
}
#[cfg(not(verus_keep_ghost))]
fn execution_inner(
    req: &Request,
    registry: &Registry,
    plan: &Execution,
    evidence: &ProjectEvidence,
    contribution: bool,
) -> Result<(), String> {
    let results = &evidence.results;
    let receipts: BTreeMap<_, _> = results
        .executed
        .iter()
        .filter_map(|row| {
            row.receipt
                .as_ref()
                .map(|receipt| (row.operation.as_str(), receipt))
        })
        .collect();
    let reply = crate::query::invoke(
        req,
        registry,
        "routes",
        "check-plan-receipts-v1",
        &json!({"plan":plan,"receipts":receipts}),
    )?;
    #[derive(serde::Deserialize)]
    struct ReceiptChecks {
        plan: String,
        operations: BTreeMap<String, Option<String>>,
    }
    let checks: ReceiptChecks = serde_json::from_slice(&reply).map_err(|e| e.to_string())?;
    if checks.plan != plan.identity
        || checks.operations.len() != plan.operations.len()
        || !plan
            .operations
            .iter()
            .all(|op| checks.operations.contains_key(&op.method.operation))
    {
        return Err("E_WORKFLOW_EVIDENCE: route receipt query identity".into());
    }
    let selected: Vec<_> = plan.selected.keys().cloned().collect();
    if let Some(completion) = &results.completion {
        let admitted = verified_execution::admit_collected(
            completion.schema == "chrono-collected-completion/v1",
            completion.status == "passed",
            results.scope.as_deref() == Some("collect"),
            completion.global_selected == selected,
            results.executed.is_empty(),
            results.blocked.is_empty(),
            results.plan == plan.identity,
            results.selected == selected,
            results.tests.keys().cloned().collect::<Vec<_>>() == selected
                && results.tests.values().all(|status| status == "passed"),
        );
        if !admitted {
            return Err("E_WORKFLOW_EVIDENCE: invalid collected completion".into());
        }
        let required: BTreeSet<_> = completion.required_units.iter().cloned().collect();
        let mut report_units = BTreeSet::new();
        let unique_reports = completion.reports.iter().all(|report| {
            report["unit"]
                .as_str()
                .is_some_and(|unit| report_units.insert(unit.to_owned()))
        });
        if required.len() != completion.required_units.len()
            || !unique_reports
            || completion.reports.len() < required.len()
            || completion
                .reports
                .iter()
                .filter_map(|report| report["unit"].as_str())
                .filter(|unit| required.contains(*unit))
                .collect::<BTreeSet<_>>()
                != required.iter().map(String::as_str).collect::<BTreeSet<_>>()
        {
            return Err("E_WORKFLOW_EVIDENCE: collected unit coverage is incomplete".into());
        }
        return Ok(());
    }
    let admitted = verified_execution::admit_execution(
        results.plan == plan.identity,
        results.scope.is_none() || contribution,
        results.selected == selected,
        results.tests.keys().cloned().collect::<Vec<_>>() == selected,
        evidence.passed,
        results.executed.len() == plan.operations.len(),
    );
    if !admitted {
        return Err("E_WORKFLOW_EVIDENCE: incomplete test execution".into());
    }
    let mut seen = BTreeSet::new();
    for op in &plan.operations {
        let matches: Vec<_> = results
            .executed
            .iter()
            .filter(|x| x.operation == op.method.operation)
            .collect();
        if matches.len() != 1 || !seen.insert(&op.method.operation) {
            return Err("E_WORKFLOW_EVIDENCE: duplicate/missing operation".into());
        }
        let result = matches[0];
        if result.error.is_some() || result.status != "passed" {
            return Err("E_WORKFLOW_EVIDENCE: failed operation".into());
        }
        if let Some(error) = &checks.operations[&op.method.operation] {
            return Err(error.clone());
        }
        let receipt = result
            .receipt
            .as_ref()
            .ok_or("E_ROUTE_MISSING: missing execution receipt")?;
        process(req, &receipt.process)?;
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| format!("host:{}", p.display()))
        .unwrap_or_else(|_| format!("external:{}", path.display()))
}
#[cfg(not(verus_keep_ghost))]
fn tools(req: &Request, plan: &Execution) -> Result<Value, String> {
    let mut out = BTreeMap::new();
    for (id, t) in &plan.tools {
        process(req, &t.version)?;
        if t.version.sha256 != t.sha256
            || t.version.executable != t.path
            || t.version.cwd != plan.root
            || t.version.environment != plan.environment
            || t.version.argv.first() != t.path.to_str().map(|s| s.to_string()).as_ref()
        {
            return Err("E_WORKFLOW_EVIDENCE: tool observation mismatch".into());
        }
        out.insert(id,json!({"program":t.program,"path":relative(&plan.root,&t.path),"sha256":t.sha256,"version_argv":&t.version.argv[1..],"version_stdout_sha256":t.version.stdout_sha256,"version_stderr_sha256":t.version.stderr_sha256}));
    }
    Ok(json!(out))
}
#[cfg(not(verus_keep_ghost))]
pub fn inputs(
    v: &Value,
    base: &str,
    candidate: &str,
    tree: &str,
    root: &Path,
) -> Result<Value, String> {
    const EXACT: &str = "chrono-effective-input-identity/v2";
    let marker = v.get("identity_schema");
    let digest = match input_identity(
        marker.is_some(),
        marker.and_then(Value::as_str) == Some(EXACT),
    ) {
        Some(InputIdentity::Endpoints) => wire::digest(&v["endpoints"])?,
        Some(InputIdentity::SchemaAndEndpoints) => {
            wire::digest(&(EXACT, &v["schema"], &v["endpoints"]))?
        }
        None => return Err("E_WORKFLOW_INPUT: unknown effective input identity schema".into()),
    };
    if !matches!(
        v["schema"].as_str(),
        Some("chrono-effective-inputs/v1" | "chrono-effective-inputs/v2")
    ) || v["identity"] != digest
        || v["endpoints"]["base"]["commit"] != base
        || v["endpoints"]["candidate"]["commit"] != candidate
    {
        return Err("E_WORKFLOW_INPUT: effective input identity".into());
    }
    let mut e = v["endpoints"].clone();
    e["candidate"]
        .as_object_mut()
        .ok_or("E_WORKFLOW_INPUT: endpoint")?
        .remove("commit");
    e["candidate"]["tree"] = tree.into();
    for name in ["base", "candidate"] {
        for f in e[name]["files"]
            .as_object_mut()
            .ok_or("E_WORKFLOW_INPUT: files")?
            .values_mut()
        {
            let p = f["location"].as_str().ok_or("E_WORKFLOW_INPUT: location")?;
            let p = Path::new(p);
            let normalized = if p.is_absolute() {
                relative(root, p)
            } else {
                format!("host:{}", p.display())
            };
            f["location"] = normalized.into();
        }
    }
    Ok(e)
}
#[cfg(not(verus_keep_ghost))]
pub fn binding(
    req: &Request,
    r: &Registry,
    plan: &Execution,
    evidence: &ProjectEvidence,
    impact: &Value,
    effective: &Value,
) -> Result<Value, String> {
    let results = &evidence.results;
    if let Some(Scope::Unit { unit }) = &req.scope {
        if results.scope.as_deref() != Some(&format!("unit:{unit}")) || results.completion.is_some()
        {
            return Err("E_WORKFLOW_EVIDENCE: contribution identity".into());
        }
        execution_inner(req, r, plan, evidence, true)?;
    } else {
        execution(req, r, plan, evidence)?;
        if results.completion.is_some() {
            projects::check_collection(req, r, effective, plan, evidence, None)?;
        }
    }
    let methods = r.methods.as_ref().map_err(Clone::clone);
    let mut templates = BTreeMap::new();
    if let Ok(methods) = &methods {
        for op in &plan.operations {
            if let Some([definition]) = methods.get(&op.method.operation).map(Vec::as_slice) {
                templates.insert(op.method.operation.as_str(), &definition.argv);
            }
        }
    }
    let context = routes::context(req, r, req, effective, &templates)?;
    if plan.binding != context.binding || plan.root != req.candidate.root {
        return Err("E_WORKFLOW_EVIDENCE: current plan binding".into());
    }
    let mut executables = BTreeMap::new();
    executables.insert("runner".to_string(),json!({"path":relative(&req.candidate.root,Path::new(&req.runner.path)),"sha256":req.runner.sha256}));
    let records = req.observations["judges"]
        .as_array()
        .ok_or("E_WORKFLOW_EVIDENCE: missing actual judge observations")?;
    let configured = r.judges["judges"].as_array().unwrap();
    let mut seen = BTreeSet::new();
    for v in records {
        let (response, p) = record(req, v)?;
        let id = &response.judge_id;
        let defs: Vec<_> = configured.iter().filter(|j| j["id"] == *id).collect();
        if defs.len() != 1
            || !seen.insert(id.clone())
            || v["binding"] != *defs[0]
            || !crate::installed_judge::matches(
                &req.candidate.root,
                defs[0]["executable"].as_str().unwrap(),
                &p.executable,
            )
            || defs[0]["sha256"] != p.sha256
            || p.cwd != req.candidate.root
            || p.environment != plan.environment
        {
            return Err("E_WORKFLOW_EVIDENCE: executed judge binding".into());
        }
        executables.insert(
            format!("judge:{id}"),
            json!({"path":relative(&req.candidate.root,&p.executable),"sha256":p.sha256}),
        );
    }
    let expected: BTreeSet<_> = configured
        .iter()
        .map(|j| j["id"].as_str().unwrap().to_string())
        .filter(|id| id != &req.judge_id)
        .collect();
    if expected != seen {
        return Err("E_WORKFLOW_EVIDENCE: workflow must follow all configured judges".into());
    }
    for p in &req.prior_results {
        let v = records
            .iter()
            .find(|v| v["id"] == p.judge_id)
            .ok_or("E_WORKFLOW_EVIDENCE: predecessor not observed")?;
        if v["response"] != serde_json::to_value(p).unwrap() {
            return Err("E_WORKFLOW_EVIDENCE: predecessor differs".into());
        }
    }
    let myself = std::env::current_exe().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&myself).map_err(|e| e.to_string())?;
    let defs: Vec<_> = configured
        .iter()
        .filter(|j| j["id"] == req.judge_id)
        .collect();
    if defs.len() != 1
        || defs[0]["sha256"] != sha256(&bytes)
        || !crate::installed_judge::matches(
            &req.candidate.root,
            defs[0]["executable"].as_str().unwrap(),
            &myself,
        )
    {
        return Err("E_WORKFLOW_EVIDENCE: current workflow executable".into());
    }
    executables.insert(
        format!("judge:{}", req.judge_id),
        json!({"path":relative(&req.candidate.root,&myself),"sha256":sha256(&bytes)}),
    );
    let methods = methods?;
    let mut contract = vec![];
    for op in &plan.operations {
        let d = methods
            .get(&op.method.operation)
            .filter(|v| v.len() == 1)
            .ok_or("E_WORKFLOW_EVIDENCE: method declaration")?;
        if op.method.tool != d[0].tool
            || context.expanded.get(&op.method.operation) != Some(&op.method.argv)
        {
            return Err("E_WORKFLOW_EVIDENCE: changed method".into());
        }
        contract.push(json!({"method":d[0],"predecessors":op.predecessors,"timeout_seconds":op.timeout_seconds,"output_limit_bytes":op.output_limit_bytes}));
    }
    Ok(
        json!({"base":req.base.commit,"candidate_tree":req.candidate.tree,"registry_digest":req.registries.digest,"executables":executables,"tools":if let Some(c)=&results.completion {c.tools.clone()} else {tools(req, plan)?},"environment":plan.environment,"effective_inputs":inputs(effective,&req.base.commit,&req.candidate.commit,&req.candidate.tree,&req.candidate.root)?,"required_tests":{"obligations":impact["required_tests"],"selected":plan.selected,"methods":contract}}),
    )
}
