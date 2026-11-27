//! Consume interpreted registration data while the workflow reader owns its endpoint observations.
#![allow(unexpected_cfgs)]
use vstd::contrib::exec_spec::ExecSpecEq;
use vstd::prelude::*;

verus! {
pub fn reply_matches(expected: &str, actual: Option<&str>, endpoint_pair_matches: bool) -> (ok: bool)
    ensures ok == (endpoint_pair_matches && match actual {
        Some(id) => id@ == expected@, None => false,
    }),
{
    match actual { Some(id) => <&str>::exec_eq(expected, id) && endpoint_pair_matches, None => false }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{facts, wire::Request};
#[cfg(not(verus_keep_ghost))]
use crate::{query, routes::Method};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::collections::BTreeMap;

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct Ambiguity {
    pub repaired: bool,
    pub definitions: Vec<String>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct RemovedOwner {
    pub kind: String,
    pub id: String,
    pub replacement: Result<Option<String>, String>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct Changes {
    pub requests: Result<BTreeMap<String, Option<String>>, String>,
    pub ambiguities: BTreeMap<String, Ambiguity>,
    pub removed_tests: BTreeMap<String, Result<Option<String>, String>>,
    pub removed_owners: Vec<RemovedOwner>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct Registry {
    pub config: Value,
    pub projects: Value,
    pub filemap: Value,
    pub judges: Value,
    pub workflow: Value,
    pub methods: Result<BTreeMap<String, Vec<Method>>, String>,
    pub test_operations: BTreeMap<String, Vec<String>>,
    pub effective_inputs: Result<Value, String>,
    pub changes: Option<Changes>,
    pub conversion: Value,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Default)]
pub struct QueryFacts(Vec<(usize, Vec<Value>)>);

#[cfg(not(verus_keep_ghost))]
impl QueryFacts {
    fn record(&mut self, reader: &facts::Reader, mut extra: Value) -> Result<(), String> {
        let mut original = reader.observation()?;
        let header = original
            .as_object_mut()
            .ok_or("E_GIT_FACTS: unbound workflow reader")?;
        let other = extra
            .as_object_mut()
            .ok_or("E_GIT_FACTS: unbound registration reader")?;
        let Some(Value::Array(local)) = header.remove("processes") else {
            return Err("E_GIT_FACTS: invalid workflow processes".into());
        };
        let Some(Value::Array(processes)) = other.remove("processes") else {
            return Err("E_GIT_FACTS: invalid registration processes".into());
        };
        if header != other {
            return Err("E_GIT_FACTS: workflow and registration bindings differ".into());
        }
        self.0.push((local.len(), processes));
        Ok(())
    }

    pub fn finish(self, reader: &facts::Reader) -> Result<Value, String> {
        let mut observed = reader.observation()?;
        if let Some(processes) = observed["processes"].as_array_mut() {
            // Positions refer to the local reader before any external observations are inserted.
            for (position, records) in self.0.into_iter().rev() {
                processes.splice(position..position, records);
            }
        }
        Ok(observed)
    }

    pub fn read(
        &mut self,
        req: &Request,
        reader: &facts::Reader,
        replay: bool,
    ) -> Result<Value, String> {
        let base = reader.snapshot(facts::Endpoint::Base)?;
        let current = reader.snapshot(facts::Endpoint::Candidate)?;
        let config = &current.values[&current.effective_path];
        let judges = current
            .values
            .get(
                config["registries"]["judges"]
                    .as_str()
                    .ok_or("missing judges registry")?,
            )
            .ok_or("missing judges document")?;
        let input = json!({"request":req,"replay":replay,
            "originals":[base.originals,current.originals]});
        let raw = query::run(
            req,
            config,
            judges,
            "registration",
            "workflow-views-v1",
            &input,
        )?;
        let mut output: Value = req.decode(&raw)?;
        let same_commits = output["commits"] == json!([req.base.commit, req.candidate.commit]);
        if !reply_matches(&req.request_id, output["request_id"].as_str(), same_commits) {
            return Err("E_PROTOCOL: registration workflow query identity".into());
        }
        if let Some(extra) = output
            .as_object_mut()
            .and_then(|object| object.remove("git_facts"))
        {
            if !extra.is_null() {
                self.record(reader, extra)?;
            }
        }
        if let Some(error) = output.get("error") {
            return Err(error
                .as_str()
                .ok_or("invalid registration workflow error")?
                .to_owned());
        }
        if replay && output["replayed"] != true {
            return Err("missing replayed registration view".into());
        }
        Ok(output)
    }
}
