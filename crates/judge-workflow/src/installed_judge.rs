use std::path::Path;

/// The runner records the resolved executable. Re-resolve the registered path
/// when binding the observation so a missing or redirected link fails closed.
pub(crate) fn matches(root: &Path, registered: &str, observed: &Path) -> bool {
    root.join(registered)
        .canonicalize()
        .is_ok_and(|target| target == observed)
}

/// Resolve the delegated runner once in the judge's launch directory. A release
/// may be a separate installation of the same bytes on the current machine.
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
    let index = crate::verified_evidence::first_executable(entries.len(), |i| {
        executable_for_judge(&entries[i])
    })
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
