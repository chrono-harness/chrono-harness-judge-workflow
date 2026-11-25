use crate::projects::Results;
use crate::registration::{Changes, Registry};
use crate::verified_transition::{self, RetirementError};
use crate::{facts, wire::Request};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
/// Adjudicate changed/removed identities; unchanged historical records are context.
pub fn evaluate(
    reader: &facts::Reader,
    req: &Request,
    b: &Registry,
    impact: &Value,
    results: &Results,
    costs: &Value,
) -> Result<Value, String> {
    let (decisions, ambiguities) = retirement_reports(
        b.changes.as_ref(),
        impact,
        &results.tests,
        &results.removed,
        &costs["declared_before"],
    )?;
    let base = reader.snapshot("base")?;
    let current = reader.snapshot("candidate")?;
    let (raw, raw_config) = (base.values, base.effective_path);
    let (now, now_config) = (current.values, current.effective_path);
    let mut changes = BTreeSet::new();
    for key in ["config", "filemap", "projects", "judges", "workflow"] {
        let op = if key == "config" {
            raw_config.as_str()
        } else {
            raw[&raw_config]["registries"][key].as_str().unwrap()
        };
        let np = if key == "config" {
            now_config.as_str()
        } else {
            now[&now_config]["registries"][key].as_str().unwrap()
        };
        let ov = raw[op]["schema_version"].as_u64().unwrap();
        let nv = now[np]["schema_version"].as_u64().unwrap();
        if ov != nv {
            changes.insert((ov, nv));
        }
    }
    let mut migrations = vec![];
    for (from, to) in changes {
        let rows: Vec<_> = b.workflow["migrations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["from_version"] == from && m["to_version"] == to)
            .collect();
        super::verified_transition::admit_declaration(rows.len())
            .map_err(super::verified_transition::MigrationError::message)?;
        let m = rows[0];
        let c = &b.conversion;
        let test = m
            .get("test")
            .and_then(Value::as_str)
            .map(|name| format!("test:{name}"));
        let replacement_test_passed = test
            .as_ref()
            .and_then(|test| results.tests.get(test))
            .map(String::as_str)
            == Some("passed");
        let conversion = serde_json::from_value::<crate::wire::Process>(c["process"].clone())
            .ok()
            .map(|p| super::verified_transition::ConversionFacts {
                process_succeeded: super::evidence::process(req, &p).is_ok(),
                declared_input_digest_matches: c["input_digest"] == p.stdin_sha256,
                actual_input_digest_matches: crate::wire::digest(&c["input"])
                    .map(|digest| digest == p.stdin_sha256)
                    .ok(),
                output_digest_matches: c["output_digest"] == p.stdout_sha256,
                decoded_output_matches: req
                    .decode::<Value>(&p.stdout_bytes)
                    .map(|output| c["output"] == output)
                    .ok(),
                mappings_match: c["output"]["mappings"] == m["mappings"],
            });
        super::verified_transition::admit_migration(super::verified_transition::MigrationFacts {
            declaration_count: rows.len(),
            has_script: m.get("script").is_some(),
            conversion_present: !c.is_null(),
            replacement_test_passed,
            script_matches: m["script"] == c["input"]["profile"]["script"],
            test_matches: m["test"] == c["input"]["profile"]["test"],
            conversion,
        })
        .map_err(super::verified_transition::MigrationError::message)?;
        migrations.push(m.clone());
    }
    Ok(
        json!({"retirements":decisions,"ambiguities":ambiguities,"migrations":migrations,"history_executed":false}),
    )
}

/// Resolve required repairs before reporting the declared test and owner retirements.
pub fn retirement_reports(
    changes: Option<&Changes>,
    impact: &Value,
    tests: &BTreeMap<String, String>,
    receipts: &BTreeMap<String, Option<String>>,
    old_costs: &Value,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let declarations = changes.map(|c| (c, c.requests.as_ref().map_err(String::as_str)));
    let (changes, requests, required_tests) =
        verified_transition::transition_inputs(declarations, impact["required_tests"].as_array())?;
    let mut decisions = vec![];
    let mut ambiguities = vec![];
    for required in required_tests {
        let node = verified_transition::repair_node(required["node"].as_str())?;
        let Some(ambiguity) = changes.ambiguities.get(node) else {
            continue;
        };
        let definitions = &ambiguity.definitions;
        let target =
            verified_transition::repair_target(node, requests.get(node).map(Option::as_deref));
        if !verified_transition::repair_is_complete(
            ambiguity.repaired,
            tests.get(target).map(String::as_str),
            old_costs.get(node).is_some(),
        ) {
            return Err(format!(
                "E_RETIREMENT: ambiguity needs explicit repair, old costs and executed replacement: {node}"
            ));
        }
        ambiguities.push(
            json!({"node":node,"definitions":definitions,"replacement":target,"status":"repaired"}),
        );
    }
    for (node, replacement) in &changes.removed_tests {
        decisions.push(retired_test_report(
            node,
            replacement,
            impact,
            tests,
            receipts,
            old_costs,
        )?);
    }
    for owner in &changes.removed_owners {
        let replacement = owner.replacement.as_ref().map_err(Clone::clone)?;
        decisions.push(json!({"kind": owner.kind, "id": owner.id, "replacement": replacement, "status":"declared-removal"}));
    }
    Ok((decisions, ambiguities))
}

/// Assemble the workflow report for one removed test from the registered views.
pub fn retired_test_report(
    node: &str,
    replacement: &Result<Option<String>, String>,
    impact: &Value,
    tests: &BTreeMap<String, String>,
    receipts: &BTreeMap<String, Option<String>>,
    old_costs: &Value,
) -> Result<Value, String> {
    let id = node
        .strip_prefix("test:")
        .ok_or("invalid removed test identity")?;
    let request = replacement.as_ref().map_err(Clone::clone)?.as_deref();
    let execution = request.and_then(|to| tests.get(to));
    let listed = impact["retired_tests"]
        .as_array()
        .ok_or("E_RETIREMENT: missing retired tests")?
        .contains(&json!(node));
    let status = verified_transition::retirement_status(
        request,
        execution.map(String::as_str),
        listed,
        receipts.get(node).map(Option::as_deref),
        old_costs.get(node).is_some(),
    )
    .map_err(|error| match error {
        RetirementError::ReplacementNotExecuted => {
            format!("E_REQUIRED_TEST_REMOVED: replacement not executed: {node}")
        }
        RetirementError::MissingObligation => {
            "E_RETIREMENT: execution did not preserve removed obligation".into()
        }
        RetirementError::MissingOldCost => format!("E_RETIREMENT: old cost missing: {node}"),
    })?;
    Ok(
        json!({"kind":"test","id":id,"replacement":request,"status":status,"executed_old_test":false}),
    )
}
