use crate::{session::Connection, wire::Request};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
pub struct Reader {
    session: Connection,
    pub bound: bool,
}
#[derive(Deserialize)]
pub struct Snapshot {
    pub values: BTreeMap<String, Value>,
    pub originals: BTreeMap<String, String>,
    pub effective_path: String,
}
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
    pub fn snapshot(&self, endpoint: &str) -> Result<Snapshot, String> {
        self.call(json!({"op":"registry-snapshot","endpoint":endpoint}))
    }
    pub fn observation(&self) -> Result<Value, String> {
        self.call(json!({"op":"observation"}))
    }
    pub fn finish(self) -> Result<(), String> {
        self.session.finish()
    }
}
