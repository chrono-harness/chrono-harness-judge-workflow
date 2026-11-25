//! Fields consumed by workflow. Extra registered fields travel unchanged in replayed originals.
use crate::session::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{cell::OnceCell, collections::BTreeMap, path::PathBuf};

type Other = BTreeMap<String, Value>;
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_jcs::to_vec(value).map_err(|e| e.to_string())
}
pub fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(sha256(&canonical(value)?))
}
#[derive(Deserialize, Serialize)]
pub struct Endpoint {
    pub commit: String,
    pub tree: String,
    pub root: PathBuf,
}
#[derive(Deserialize, Serialize)]
pub struct Registries {
    pub digest: String,
    #[serde(flatten)]
    other: Other,
}
#[derive(Deserialize, Serialize)]
pub struct Original {
    pub path: PathBuf,
    pub sha256: String,
}
#[derive(Deserialize, Serialize)]
pub struct Program {
    pub path: String,
    pub sha256: String,
    #[serde(flatten)]
    other: Other,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}
#[derive(Deserialize, Serialize)]
pub struct Request {
    pub base: Endpoint,
    pub candidate: Endpoint,
    pub registries: Registries,
    pub context: Original,
    pub runner: Program,
    pub delta: Value,
    pub impact: Value,
    pub request_id: String,
    pub judge_id: String,
    pub config_path: String,
    pub prior_results: Vec<Response>,
    pub observations: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(flatten)]
    other: Other,
    #[serde(skip)]
    bound_runner: OnceCell<Result<PathBuf, String>>,
    #[serde(skip)]
    artifacts: OnceCell<Result<Connection, String>>,
    #[serde(skip)]
    evidence: OnceCell<Result<Connection, String>>,
}
impl Request {
    pub fn runner_path(&self) -> Result<&std::path::Path, String> {
        self.bound_runner
            .get_or_init(|| {
                let search =
                    std::env::var_os("PATH").ok_or("E_EXECUTABLE_BINDING: missing judge PATH")?;
                crate::installed_judge::bound_runner(
                    &self.candidate.root,
                    &search,
                    &self.runner.sha256,
                )
            })
            .as_deref()
            .map_err(Clone::clone)
    }

    pub fn artifact<T: serde::de::DeserializeOwned>(&self, op: Value) -> Result<T, String> {
        self.artifacts
            .get_or_init(|| {
                Connection::open(
                    self.runner_path()?,
                    &self.candidate.root,
                    "retained-artifacts-v1",
                )
            })
            .as_ref()
            .map_err(Clone::clone)?
            .call(&op)
    }
    pub fn evidence<T: serde::de::DeserializeOwned>(&self, op: Value) -> Result<T, String> {
        self.evidence
            .get_or_init(|| {
                Connection::open(
                    self.runner_path()?,
                    &self.candidate.root,
                    "judge-evidence-v1",
                )
            })
            .as_ref()
            .map_err(Clone::clone)?
            .call(&op)
    }
    pub fn decode<T: serde::de::DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, String> {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        self.evidence(serde_json::json!({"operation":"decode-json","text":text}))
    }
    pub fn finish(&mut self) -> Result<(), String> {
        if let Some(Ok(connection)) = self.evidence.take() {
            connection.finish()?;
        }
        if let Some(Ok(connection)) = self.artifacts.take() {
            connection.finish()?;
        }
        Ok(())
    }
}
#[derive(Clone, Deserialize, Serialize, PartialOrd, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Warn,
    Fail,
    Error,
}
#[derive(Deserialize, Serialize)]
pub struct Response {
    pub judge_id: String,
    pub request_id: String,
    pub status: Status,
    pub outputs: BTreeMap<String, Value>,
    #[serde(flatten)]
    other: Other,
}
#[derive(Deserialize, Serialize)]
pub struct Process {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub stdin_sha256: String,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub stdout_bytes: Vec<u8>,
    pub executable: PathBuf,
    pub sha256: String,
    #[serde(flatten)]
    other: Other,
}
#[derive(Deserialize, Serialize)]
pub struct Tool {
    pub program: String,
    pub path: PathBuf,
    pub sha256: String,
    pub version: Process,
    #[serde(flatten)]
    other: Other,
}
