//! Workflow owns freshness, explicit evolution and integration evidence policy.
mod branch;
mod certificate;
mod evidence;
mod installed_judge;
mod projects;
mod query;
mod registration;
mod routes;
mod transition;
mod verified_branch;
mod verified_certificate;
mod verified_evidence;
mod verified_execution;
mod verified_transition;
use crate::routes::Execution;
pub use branch::{branch, check_observation_age, current_observation_age, observation_age};
mod facts;
mod session;
mod wire;
use crate::wire::{Request, Status};
use serde_json::{Value, json as value};

pub fn evaluate_original(
    input: impl std::io::Read,
    mut output: impl std::io::Write,
) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct Prepared {
        request: Request,
        admission: Result<(), String>,
    }
    let Prepared {
        mut request,
        admission,
    } = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let mut outputs = serde_json::Map::new();
    let run = || -> Result<Value, String> {
        admission?;
        let reader = facts::Reader::for_request(&request)?;
        let mut queries = registration::QueryFacts::default();
        let result = evaluate(&request, &reader, &mut queries);
        if reader.bound {
            outputs.insert("git_facts".into(), queries.finish(&reader)?);
        }
        reader.finish()?;
        result
    };
    let result = run();
    let (status, findings) = match request.finish().and(result) {
        Ok(verdict) => {
            outputs.insert("workflow".into(), verdict);
            (Status::Pass, Vec::new())
        }
        Err(message) => {
            let (code, severity) =
                verified_execution::response::classify_error(&message, Status::Fail, Status::Error);
            (
                severity,
                vec![
                    value!({"code":code,"level":"error","message":message,"delta_refs":["/request/context"],"causes":[]}),
                ],
            )
        }
    };
    serde_json::to_writer(
        &mut output,
        &value!({"status":status,"findings":findings,"outputs":outputs}),
    )
    .map_err(|e| e.to_string())
}
fn evaluate(
    req: &Request,
    reader: &facts::Reader,
    queries: &mut registration::QueryFacts,
) -> Result<Value, String> {
    let b: registration::Registry = serde_json::from_value(queries.read(req, reader, false)?)
        .map_err(|e| format!("invalid registration workflow view: {e}"))?;
    let ctx: Value = req.decode(&std::fs::read(&req.context.path).map_err(|e| e.to_string())?)?;
    if wire::digest(&ctx)? != req.context.sha256
        || ctx["schema_version"] != 2
        || ctx["candidate"] != req.candidate.commit
        || ctx["base"] != req.base.commit
    {
        return Err("E_BRANCH_CONTEXT: workflow requires matching explicit v2 context".into());
    }
    let mut state = branch::with_reader(reader, &req.candidate.commit, &ctx, &b.workflow)?;
    let observation = &req.observations["preparation"]["result"]["evidence"]["current_observation"];
    if !observation.is_null() {
        if observation["context_digest"] != req.context.sha256 {
            return Err("E_BRANCH_CONTEXT: current observation context binding differs".into());
        }
        let age = branch::current_observation_age(&ctx, &b.workflow, observation)?;
        state["age_seconds"] = value!(age.as_seconds_f64());
        state["current_observation"] = observation.clone();
    }
    if !matches!(ctx["run_kind"].as_str(), Some("integration" | "delivery"))
        || (ctx["run_kind"] == "integration" && state["kind"] != "integration")
    {
        return Err("E_BRANCH_CONTEXT: invalid explicit run kind".into());
    }
    let impact = evidence::output(req, "impact")?;
    if impact["schema"] != "chrono-filemap-impact/v6"
        || impact["delta"] != serde_json::to_value(&req.delta).unwrap()
        || impact["workflow"]["schema"] != "chrono-workflow-selection/v1"
    {
        return Err("E_WORKFLOW_INPUT: impact identity".into());
    }
    let required = impact["workflow"]["integration_required"]
        .as_bool()
        .ok_or("E_WORKFLOW_INPUT: integration selection")?;
    let plan: Execution = serde_json::from_value(evidence::output(req, "execution_plan")?.clone())
        .map_err(|e| e.to_string())?;
    let results = projects::read(req, &b, evidence::output(req, "tests")?)?;
    let effective = b.effective_inputs.as_ref().map_err(Clone::clone)?;
    let binding = evidence::binding(req, &b, &plan, &results, impact, effective)?;
    if let Some(wire::Scope::Unit { unit }) = &req.scope {
        let global_selected = req
            .prior_results
            .iter()
            .find_map(|result| result.outputs.get("global_selected"))
            .cloned()
            .unwrap_or_else(|| impact["tests"].clone());
        let selected: std::collections::BTreeSet<String> =
            serde_json::from_value(global_selected.clone())
                .map_err(|e| format!("E_WORKFLOW_INPUT: global selection: {e}"))?;
        let required_units:Vec<String>=req.evidence(value!({"operation":"required-units","units":req.observations["execution_units"]["units"],"selected":selected})).map_err(|e|format!("E_WORKFLOW_INPUT: execution units: {e}"))?;
        return Ok(value!({
            "schema":"chrono-workflow-verdict/v1",
            "mode":"contribution",
            "acceptance":"unit-only; global completion requires collection",
            "unit":unit,
            "global_selected":global_selected,
            "selected":plan.selected,
            "required_units":required_units,
            "integration_required":impact["workflow"]["integration_required"],
            "integration":Value::Null,
            "completion":"pending collection",
            "binding":binding
        }));
    }
    let transitions = transition::evaluate(
        reader,
        req,
        &b,
        impact,
        &results.results,
        evidence::output(req, "costs")?,
    )?;
    certificate::validate_bind(&b)?;
    let (mode, integration) = if ctx["run_kind"] == "integration" {
        (
            "integration_run",
            certificate::produce(req, &b, &ctx, binding, &plan, &results, effective)?,
        )
    } else if required {
        (
            "delivery",
            certificate::consume(
                queries,
                reader,
                req,
                &b,
                &ctx,
                &binding,
                (!observation.is_null()).then_some(observation),
            )?,
        )
    } else {
        ("ordinary_delta", Value::Null)
    };
    Ok(
        value!({"schema":"chrono-workflow-verdict/v1","mode":mode,"branch":state,"integration_required":required,"integration":integration,"transitions":transitions,"input_completeness_proven":false}),
    )
}
