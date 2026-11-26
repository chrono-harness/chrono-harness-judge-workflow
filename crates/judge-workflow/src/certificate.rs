//! Producer branch metadata shared by the integration writer and delivery reader.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Compare the retained projection in the same order that the writer copies it.
pub fn producer_fields() -> (fields: [&'static str; 4])
    ensures fields[0]@ == "branch_ref"@, fields[1]@ == "fork_point"@,
        fields[2]@ == "branch_started_at"@, fields[3]@ == "observed_at"@,
{
    ["branch_ref", "fork_point", "branch_started_at", "observed_at"]
}
}

#[cfg(not(verus_keep_ghost))]
use crate::projects::{self, ProjectEvidence};
#[cfg(not(verus_keep_ghost))]
use crate::registration::Registry;
#[cfg(not(verus_keep_ghost))]
use crate::routes::{self, Execution};
#[cfg(not(verus_keep_ghost))]
use crate::wire::{self, Request, sha256};
#[cfg(not(verus_keep_ghost))]
use crate::{evidence, verified_certificate};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json as value};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeSet, fs, path::Path};
#[cfg(not(verus_keep_ghost))]
fn location(req: &Request, r: &Registry) -> Result<std::path::PathBuf, String> {
    let path = r.workflow["integration"]["evidence"]
        .as_str()
        .ok_or("E_INTEGRATION_BINDING: evidence path")?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("E_INTEGRATION_BINDING: evidence must be in state".into());
    }
    req.artifact(value!({"operation":"checked-path","path":path}))
}
#[cfg(not(verus_keep_ghost))]
pub fn validate_bind(r: &Registry) -> Result<(), String> {
    let keys = r.workflow["integration"]["bind"].as_array().unwrap();
    let supplied: BTreeSet<_> = keys.iter().filter_map(Value::as_str).collect();
    let fields = verified_certificate::binding_fields();
    let admitted = verified_certificate::admit_binding(
        keys.len() == fields.len() + 1,
        supplied == fields.into_iter().chain(["results_digest"]).collect(),
    );
    if !admitted {
        return Err(
            "E_INTEGRATION_BINDING: all specified evidence fields must be bound exactly once"
                .into(),
        );
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn produce(
    req: &Request,
    r: &Registry,
    ctx: &Value,
    mut binding: Value,
    plan: &Execution,
    results: &ProjectEvidence,
    effective: &Value,
) -> Result<Value, String> {
    let report = req.observations["report_path"]
        .as_str()
        .filter(|p| p.starts_with(".chrono-harness/state/"))
        .ok_or("E_WORKFLOW_EVIDENCE: retained runner report path missing")?;
    binding["schema_version"] = 1.into();
    binding["status"] = "passed".into();
    binding["results_digest"] = wire::digest(&results.wire)?.into();
    for k in producer_fields() {
        binding[k] = ctx[k].clone();
    }
    binding["producer"] = value!({"judge":req.judge_id,"request_id":req.request_id,"candidate":req.candidate.commit,"root":req.candidate.root,"report_path":report,"context_digest":req.context.sha256});
    if let Some(directory) =
        req.observations["preparation"]["request"]["native_artifacts"]["directory"].as_str()
    {
        binding["producer"]["artifact_directory"] = value!(directory);
    }
    binding["proof"] = value!({"context":ctx,"plan":plan,"results":results.wire,"effective_inputs":effective,"judges":req.observations["judges"]});
    let path = location(req, r)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = wire::canonical(&binding)?;
    fs::write(path, &bytes).map_err(|e| e.to_string())?;
    Ok(
        value!({"sha256":sha256(&bytes),"path":r.workflow["integration"]["evidence"],"producer_report":report,"validation_scope":"declared inputs; complete runtime closure remains a separate obligation"}),
    )
}
#[cfg(not(verus_keep_ghost))]
fn report_output<'a>(report: &'a Value, key: &str) -> Result<&'a Value, String> {
    let values: Vec<_> = report["judges"]
        .as_array()
        .ok_or("missing judges")?
        .iter()
        .filter_map(|j| j["response"]["outputs"].get(key))
        .collect();
    verified_certificate::agreed_output(&values, |_| true, |a, b| values[a] == values[b])
        .ok_or_else(|| format!("missing/conflicting report {key}"))
}
#[cfg(not(verus_keep_ghost))]
pub fn consume(
    queries: &mut crate::registration::QueryFacts,
    reader: &crate::facts::Reader,
    req: &Request,
    r: &Registry,
    ctx: &Value,
    binding: &Value,
    observation: Option<&Value>,
) -> Result<Value, String> {
    let digest = ctx["integration_evidence"]
        .as_str()
        .ok_or("E_INTEGRATION_REQUIRED: no evidence digest")?;
    let path = location(req, r)?;
    let relative = path
        .strip_prefix(&req.candidate.root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("certificate path UTF8")?;
    req.artifact::<()>(value!({"operation":"retain-read","path":relative}))?;
    let bytes = fs::read(path).map_err(|e| format!("E_INTEGRATION_REQUIRED: {e}"))?;
    let mut check = || -> Result<Value, String> {
        if sha256(&bytes) != digest {
            return Err("certificate byte digest differs".into());
        }
        let c: Value = req.decode(&bytes)?;
        if c["schema_version"] != 1 || c["status"] != "passed" {
            return Err("certificate did not succeed".into());
        }
        let fields = verified_certificate::binding_fields();
        if let Some(index) = verified_certificate::first_mismatch(fields.len(), |i| {
            let field = fields[i];
            c[field] == binding[field]
        }) {
            return Err(format!("different binding {}", fields[index]));
        }
        let proof = &c["proof"];
        let producer = &c["producer"];
        let candidate = producer["candidate"]
            .as_str()
            .ok_or("missing producer candidate")?;
        if reader.verify_oid(candidate)? != req.candidate.tree {
            return Err("producer candidate tree differs".into());
        }
        let old_ctx = &proof["context"];
        if old_ctx["schema_version"] != 2
            || old_ctx["run_kind"] != "integration"
            || old_ctx["candidate"] != candidate
            || wire::digest(old_ctx)? != producer["context_digest"]
        {
            return Err("producer context differs".into());
        }
        let fields = producer_fields();
        if let Some(index) = verified_certificate::first_mismatch(fields.len(), |i| {
            let field = fields[i];
            c[field] == old_ctx[field]
        }) {
            return Err(format!("producer {} differs", fields[index]));
        }
        // Evidence must still be fresh at this observation, not only at its original run.
        let timestamp = |v: &Value| {
            time::OffsetDateTime::parse(
                v["observed_at"].as_str().unwrap_or(""),
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| e.to_string())
        };
        if timestamp(old_ctx)? > timestamp(ctx)? {
            return Err("producer observation is in the future".into());
        }
        let mut now = old_ctx.clone();
        now["observed_at"] = ctx["observed_at"].clone();
        if crate::branch::with_reader(reader, candidate, &now, &r.workflow)?["kind"]
            != "integration"
        {
            return Err("producer is not integration".into());
        }
        if let Some(observation) = observation {
            crate::branch::current_observation_age(old_ctx, &r.workflow, observation)?;
        }
        let plan: Execution =
            serde_json::from_value(proof["plan"].clone()).map_err(|e| e.to_string())?;
        let results = projects::read(req, r, &proof["results"])?;
        evidence::execution(req, r, &plan, &results)?;
        if c["results_digest"] != wire::digest(&results.wire)?
            || plan.root != Path::new(producer["root"].as_str().ok_or("producer root")?)
            || plan.binding["candidate"] != candidate
            || plan.binding["base"] != req.base.commit
            || plan.binding["registry"] != req.registries.digest
            || plan.binding["context"] != producer["context_digest"]
        {
            return Err("producer plan/results binding differs".into());
        }
        if evidence::inputs(
            &proof["effective_inputs"],
            &req.base.commit,
            candidate,
            &req.candidate.tree,
            &plan.root,
        )? != c["effective_inputs"]
        {
            return Err("retained effective inputs differ".into());
        }
        let report_path = producer["report_path"]
            .as_str()
            .filter(|p| p.starts_with(".chrono-harness/state/"))
            .ok_or("retained report path")?;
        let physical:String=req.artifact(value!({"operation":"resolve-original","original":{"path":report_path,"sha256":""},"transport":null}))?;
        let report_file: std::path::PathBuf =
            req.artifact(value!({"operation":"checked-path","path":physical}))?;
        if results.results.completion.is_some()
            && fs::metadata(&report_file).map_err(|e| e.to_string())?.len()
                > r.config["execution_units"]["collection_limits"]["report_bytes"]
                    .as_u64()
                    .ok_or("collected producer report bound")?
        {
            return Err("collected producer report exceeds registered bound".into());
        }
        req.artifact::<()>(value!({"operation":"retain-read","path":physical}))?;
        let report_bytes = fs::read(&report_file)
            .map_err(|e| format!("producer report absent/not finalized: {e}"))?;
        let report: Value = req.decode(&report_bytes)?;
        if results.results.completion.is_some()
            && req.evidence::<u64>(value!({"operation":"report-transport-bytes","report":report,"metadata_bytes":report_bytes.len()}))?
                > r.config["execution_units"]["collection_limits"]["report_bytes"]
                    .as_u64()
                    .ok_or("collected producer report bound")?
        {
            return Err(
                "collected producer metadata and original streams exceed registered bound".into(),
            );
        }
        if !matches!(report["status"].as_str(), Some("pass" | "warn"))
            || report["report_path"] != report_path
            || report["base"] != req.base.commit
            || report["candidate"] != candidate
            || report["candidate_tree"] != req.candidate.tree
            || report["registry_digest"] != req.registries.digest
            || report["context_digest"] != producer["context_digest"]
        {
            return Err("producer runner did not finalize a matching successful report".into());
        }
        if report_output(&report, "execution_plan")? != &proof["plan"]
            || report_output(&report, "tests")? != &proof["results"]
            || report_output(&report, "effective_inputs")? != &proof["effective_inputs"]
        {
            return Err("producer report outputs differ".into());
        }
        let expanded = report["judges"]
            .as_array()
            .ok_or("producer judges")?
            .iter()
            .map(|record| {
                req.evidence::<Value>(value!({"operation":"expand-record","record":record}))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let records = &expanded;
        if results.results.completion.is_some() {
            let original: Request =
                serde_json::from_value(report["request"].clone()).map_err(|e| e.to_string())?;
            req.evidence::<()>(value!({"operation":"validate-request","request":original}))?;
            if original.candidate.root != plan.root
                || original.context.sha256 != producer["context_digest"]
            {
                return Err("collected producer request differs".into());
            }
            let parsed: Value = req.evidence(value!({
                "operation": "retained-judges",
                "request": original,
                "bindings": r.judges["judges"],
                "records": report["judges"],
                "judge": producer["judge"]
            }))?;
            let findings: Vec<Value> = records
                .iter()
                .flat_map(|row| {
                    row["response"]["findings"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .cloned()
                })
                .collect();
            if report["status"] != parsed["status"] || report["findings"] != value!(findings) {
                return Err("collected producer summary differs from actual judges".into());
            }
            let original: Request =
                serde_json::from_value(parsed["request"].clone()).map_err(|e| e.to_string())?;
            if original.request_id != producer["request_id"]
                || original.impact != req.impact
                || plan.binding
                    != routes::context(
                        req,
                        r,
                        &original,
                        &proof["effective_inputs"],
                        &Default::default(),
                    )?
                    .binding
            {
                return Err("collected producer stdin/global obligation binding differs".into());
            }
            let current_plan: Execution =
                serde_json::from_value(evidence::output(req, "execution_plan")?.clone())
                    .map_err(|e| e.to_string())?;
            let expected = routes::collection(req, r, &plan, &current_plan)?;
            if expected != proof["plan"] {
                return Err("collected global declaration plan differs".into());
            }
            // Retain the workflow reader's observations for the fixed registration endpoints.
            queries.read(req, reader, true)?;
            projects::check_collection(
                req,
                r,
                &proof["effective_inputs"],
                &plan,
                &results,
                Some(value!({"request":original,"artifacts":report["artifacts"]})),
            )?;
        }
        let observations = proof["judges"].as_array().ok_or("producer observations")?;
        if records.len() != observations.len() + 1 {
            return Err("producer judge set differs".into());
        }
        for old in observations {
            let expanded: Value =
                req.evidence(value!({"operation":"expand-record","record":old}))?;
            let old = &expanded;
            let index = verified_certificate::unique_record(records.len(), |i| {
                records[i]["id"] == old["id"]
            })
            .ok_or("producer judge observation missing/duplicate")?;
            for (k, v) in old.as_object().ok_or("producer observation")? {
                if records[index][k] != *v {
                    return Err("producer observation changed".into());
                }
            }
        }
        let mut seen = BTreeSet::new();
        let mut executable_map = serde_json::Map::new();
        let runner = &report["executables"][0];
        let rp = runner["path"].as_str().ok_or("runner path")?;
        executable_map.insert(
            "runner".into(),
            value!({"path":evidence::relative(&plan.root,Path::new(rp)),"sha256":runner["sha256"]}),
        );
        for row in records {
            let (response, p) = evidence::record(req, row)?;
            if !seen.insert(response.judge_id.clone()) {
                return Err("duplicate observed judge".into());
            }
            executable_map.insert(
                format!("judge:{}", response.judge_id),
                value!({"path":evidence::relative(&plan.root,&p.executable),"sha256":p.sha256}),
            );
        }
        if Value::Object(executable_map) != c["executables"] {
            return Err("producer executable observations differ".into());
        }
        let index = verified_certificate::unique_record(records.len(), |i| {
            records[i]["id"] == producer["judge"]
        })
        .ok_or("producer workflow result differs")?;
        let own = &records[index];
        let workflow = &own["response"]["outputs"]["workflow"];
        if !verified_certificate::producer_claim(
            own["request_id"] == producer["request_id"],
            workflow["mode"].as_str(),
            workflow["integration"]["sha256"].as_str(),
            digest,
        ) {
            return Err("producer workflow result differs".into());
        }
        Ok(
            value!({"sha256":digest,"producer_report":report_path,"producer_report_sha256":sha256(&report_bytes),"producer_candidate":candidate,"consumer_candidate":req.candidate.commit,"candidate_tree":req.candidate.tree,"commit_metadata_mapped":candidate!=req.candidate.commit}),
        )
    };
    check().map_err(|e| format!("E_INTEGRATION_MISMATCH: {e}"))
}
