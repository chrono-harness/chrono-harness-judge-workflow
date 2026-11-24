use std::path::Path;

/// The runner records the resolved executable. Re-resolve the registered path
/// when binding the observation so a missing or redirected link fails closed.
pub(crate) fn matches(root: &Path, registered: &str, observed: &Path) -> bool {
    root.join(registered)
        .canonicalize()
        .is_ok_and(|target| target == observed)
}
