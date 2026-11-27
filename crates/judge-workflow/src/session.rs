//! One line per operation. Closing stdin releases the runner's invocation-scoped capabilities.
#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
/// A runner session reply must end with its delimiter, even when the preceding bytes decode.
pub fn complete_reply(line: &[u8]) -> (complete: bool)
    ensures complete == (line.len() > 0 && line@[line.len() - 1] == 10u8),
{
    !line.is_empty() && line[line.len() - 1] == 10
}
}

#[cfg(not(verus_keep_ghost))]
use serde::{Serialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
#[cfg(not(verus_keep_ghost))]
struct Pipes {
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    child: Child,
}
#[cfg(not(verus_keep_ghost))]
pub struct Connection {
    pipes: RefCell<Pipes>,
    operation: &'static str,
}
#[cfg(not(verus_keep_ghost))]
impl Connection {
    pub fn open(program: &Path, root: &Path, operation: &'static str) -> Result<Self, String> {
        let mut child = Command::new(program)
            .arg(operation)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("chrono-harness {operation}: {e}"))?;
        Ok(Self {
            pipes: RefCell::new(Pipes {
                input: child.stdin.take(),
                output: BufReader::new(child.stdout.take().unwrap()),
                child,
            }),
            operation,
        })
    }
    pub fn call<T: DeserializeOwned>(&self, request: &impl Serialize) -> Result<T, String> {
        let mut pipes = self.pipes.borrow_mut();
        let input = pipes.input.as_mut().ok_or("runner session closed")?;
        serde_json::to_writer(&mut *input, request).map_err(|e| e.to_string())?;
        input
            .write_all(b"\n")
            .and_then(|()| input.flush())
            .map_err(|e| e.to_string())?;
        let mut line = Vec::new();
        pipes
            .output
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if !complete_reply(&line) {
            return Err(format!(
                "chrono-harness {}: incomplete reply",
                self.operation
            ));
        }
        serde_json::from_slice::<Result<T, String>>(&line).map_err(|e| e.to_string())?
    }
    pub fn finish(self) -> Result<(), String> {
        let mut pipes = self.pipes.borrow_mut();
        drop(pipes.input.take());
        let status = pipes.child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("chrono-harness {}: {status}", self.operation))
        }
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for Connection {
    fn drop(&mut self) {
        let pipes = self.pipes.get_mut();
        drop(pipes.input.take());
        let _ = pipes.child.wait();
    }
}
