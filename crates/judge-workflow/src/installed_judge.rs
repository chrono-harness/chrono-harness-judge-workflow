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
    use std::{env, fs, io, os::unix::fs::PermissionsExt};

    let entry = env::split_paths(search)
        .map(|dir| root.join(dir).join("chrono-harness"))
        .find(|path| {
            path.metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
        .ok_or("E_EXECUTABLE_BINDING: chrono-harness missing from judge PATH")?;
    let path = entry
        .canonicalize()
        .map_err(|e| format!("E_EXECUTABLE_BINDING: {}: {e}", entry.display()))?;
    let mut file = fs::File::open(&path)
        .map_err(|e| format!("E_EXECUTABLE_BINDING: {}: {e}", path.display()))?;
    let mut digest = Sha256::new();
    io::copy(&mut file, &mut digest)
        .map_err(|e| format!("E_EXECUTABLE_BINDING: {}: {e}", path.display()))?;
    let actual = format!("{:x}", digest.finalize());
    if actual != expected {
        return Err(format!(
            "E_EXECUTABLE_BINDING: chrono-harness {} has sha256 {actual}, expected {expected}",
            path.display()
        ));
    }
    Ok(path)
}
