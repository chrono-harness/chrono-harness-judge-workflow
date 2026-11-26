#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
/// Stop at the first entry the operating system reports as executable.
pub fn first_executable<F: Fn(usize) -> bool>(
    count: usize, executable: F,
) -> (first: Option<usize>)
    requires forall|i: usize| i < count ==> executable.requires((i,)),
    ensures match first {
        None => forall|i: usize| i < count ==> executable.ensures((i,), false),
        Some(index) => index < count && executable.ensures((index,), true)
            && forall|i: usize| i < index ==> executable.ensures((i,), false),
    },
{
    let mut i = 0;
    while i < count
        invariant i <= count,
            forall|j: usize| j < count ==> executable.requires((j,)),
            forall|j: usize| j < i ==> executable.ensures((j,), false),
        decreases count - i,
    {
        if executable(i) { return Some(i); }
        i += 1;
    }
    None
}
}

#[cfg(not(verus_keep_ghost))]
use std::path::Path;

/// The runner records the resolved executable. Re-resolve the registered path
/// when binding the observation so a missing or redirected link fails closed.
#[cfg(not(verus_keep_ghost))]
pub(crate) fn matches(root: &Path, registered: &str, observed: &Path) -> bool {
    root.join(registered)
        .canonicalize()
        .is_ok_and(|target| target == observed)
}

/// Resolve the delegated runner once in the judge's launch directory. A release
/// may be a separate installation of the same bytes on the current machine.
#[cfg(not(verus_keep_ghost))]
pub(crate) fn bound_runner(
    root: &Path,
    search: &std::ffi::OsStr,
    expected: &str,
) -> Result<std::path::PathBuf, String> {
    use sha2::{Digest, Sha256};
    use std::{env, fs, io};

    let entries: Vec<_> = env::split_paths(search)
        .map(|dir| root.join(dir).join("chrono-harness"))
        .collect();
    let index = first_executable(entries.len(), |i| executable_for_judge(&entries[i]))
        .ok_or("E_EXECUTABLE_BINDING: chrono-harness missing from judge PATH")?;
    let entry = &entries[index];
    let path = entry
        .canonicalize()
        .map_err(|e| format!("E_EXECUTABLE_BINDING: {}: {e}", entry.display()))?;
    let mut file = fs::File::open(&path)
        .map_err(|e| format!("E_EXECUTABLE_BINDING: {}: {e}", path.display()))?;
    let mut digest = digest_io::IoWrapper(Sha256::new());
    io::copy(&mut file, &mut digest)
        .map_err(|e| format!("E_EXECUTABLE_BINDING: {}: {e}", path.display()))?;
    let actual = hex::encode(digest.0.finalize());
    crate::verified_evidence::bind_digest(path, &actual, expected).map_err(|path| {
        format!(
            "E_EXECUTABLE_BINDING: chrono-harness {} has sha256 {actual}, expected {expected}",
            path.display()
        )
    })
}

/// Ask the kernel about execution for this process, including its effective
/// identity and the filesystem's access-control policy.
#[cfg(not(verus_keep_ghost))]
fn executable_for_judge(path: &Path) -> bool {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    if !path.is_file() {
        return false;
    }
    let Ok(name) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: name is NUL-terminated and remains alive for the syscall.
    unsafe { libc::faccessat(libc::AT_FDCWD, name.as_ptr(), libc::X_OK, libc::AT_EACCESS) == 0 }
}
