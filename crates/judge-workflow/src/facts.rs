#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
pub enum Endpoint { Base, Candidate }

/// Both snapshot consumers name the same ordered endpoints in the runner protocol.
pub fn endpoint_name(endpoint: Endpoint) -> (name: &'static str)
    ensures match endpoint {
        Endpoint::Base => name@ == "base"@,
        Endpoint::Candidate => name@ == "candidate"@,
    },
{
    match endpoint { Endpoint::Base => "base", Endpoint::Candidate => "candidate" }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{session::Connection, wire::Request};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, path::Path};
#[cfg(not(verus_keep_ghost))]
pub struct Reader {
    session: Connection,
    pub bound: bool,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub struct Snapshot {
    pub values: BTreeMap<String, Value>,
    pub originals: BTreeMap<String, String>,
    pub effective_path: String,
}
#[cfg(not(verus_keep_ghost))]
impl Reader {
    pub fn for_request(req: &Request) -> Result<Self, String> {
        Self::open(
            req.runner_path()?,
            &req.candidate.root,
            json!({"mode":"request","request":req}),
        )
    }
    pub fn legacy(root: &Path) -> Result<Self, String> {
        Self::open(Path::new("chrono-harness"), root, json!({"mode":"history"}))
    }
    fn open(program: &Path, root: &Path, header: Value) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Open {
            bound: bool,
        }
        let session = Connection::open(program, root, "git-facts-v1")?;
        let Open { bound } = session.call(&header)?;
        Ok(Self { session, bound })
    }
    pub fn call<T: DeserializeOwned>(&self, op: Value) -> Result<T, String> {
        self.session.call(&op)
    }
    pub fn verify_oid(&self, commit: &str) -> Result<String, String> {
        self.call(json!({"op":"verify-commit","commit":commit}))
    }
    pub fn snapshot(&self, endpoint: Endpoint) -> Result<Snapshot, String> {
        self.call(json!({"op":"registry-snapshot","endpoint":endpoint_name(endpoint)}))
    }
    pub fn observation(&self) -> Result<Value, String> {
        self.call(json!({"op":"observation"}))
    }
    pub fn finish(self) -> Result<(), String> {
        self.session.finish()
    }
}
