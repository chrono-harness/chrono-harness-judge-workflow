#[path = "../../judge-workflow/src/installed_judge.rs"]
mod installed_judge;

#[cfg(unix)]
#[test]
fn registered_installation_link_must_resolve_to_the_observed_executable() {
    use std::fs;
    use std::os::unix::fs::symlink;

    let scratch = std::env::temp_dir().join(format!("workflow-installed-{}", std::process::id()));
    fs::create_dir(&scratch).unwrap();
    let host = scratch.join("host");
    let installed = scratch.join("release");
    let other = scratch.join("other-release");
    fs::create_dir(&host).unwrap();
    fs::write(&installed, b"release").unwrap();
    fs::write(&other, b"release").unwrap();
    let observed = installed.canonicalize().unwrap();
    let entry = host.join("judge");
    symlink(&installed, &entry).unwrap();
    assert!(installed_judge::matches(&host, "judge", &observed));
    assert!(!installed_judge::matches(&host, "judge", &other));
    fs::remove_file(&entry).unwrap();
    assert!(!installed_judge::matches(&host, "judge", &observed));
    symlink(&other, &entry).unwrap();
    assert!(!installed_judge::matches(&host, "judge", &observed));
    assert!(installed_judge::matches(&scratch, "release", &observed));
    fs::remove_dir_all(scratch).unwrap();
}
