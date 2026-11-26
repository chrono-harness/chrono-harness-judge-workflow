#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy::{self as verified_evidence, installed as installed_judge};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
#[cfg(not(verus_keep_ghost))]
#[path = "../../judge-workflow/src/installed_judge.rs"]
mod installed_judge;
#[cfg(not(verus_keep_ghost))]
#[allow(dead_code)]
#[path = "../../judge-workflow/src/verified_evidence.rs"]
mod verified_evidence;

#[cfg(all(unix, not(verus_keep_ghost)))]
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

#[cfg(all(unix, not(verus_keep_ghost)))]
#[test]
fn request_uses_the_first_executable_on_its_path_only_if_its_bytes_match() {
    use sha2::{Digest, Sha256};
    use std::{
        env, fs,
        os::unix::fs::{PermissionsExt, symlink},
        path::Path,
    };

    let root = env::temp_dir().join(format!("workflow-runner-{}", std::process::id()));
    let first = root.join("first");
    let last = root.join("last");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&last).unwrap();
    let search = env::join_paths([Path::new("first"), &last]).unwrap();
    let expected = hex::encode(Sha256::digest(b"release"));
    let resolve = || installed_judge::bound_runner(&root, &search, &expected);
    assert!(resolve().unwrap_err().contains("missing"));
    let release = last.join("chrono-harness");
    fs::write(&release, b"release").unwrap();
    fs::set_permissions(&release, fs::Permissions::from_mode(0o700)).unwrap();
    let earlier = first.join("chrono-harness");
    fs::write(&earlier, b"older release").unwrap();
    fs::set_permissions(&earlier, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(resolve().unwrap(), release.canonicalize().unwrap());
    // These files belong to the caller. Group and other execution bits
    // do not grant their owner execute access when the owner bit is clear.
    // The superuser may execute a regular file with any execute bit.
    if unsafe { libc::geteuid() } != 0 {
        fs::set_permissions(&earlier, fs::Permissions::from_mode(0o611)).unwrap();
        assert_eq!(resolve().unwrap(), release.canonicalize().unwrap());
    }
    fs::set_permissions(&earlier, fs::Permissions::from_mode(0o700)).unwrap();
    let err = resolve().unwrap_err();
    assert!(err.contains(&hex::encode(Sha256::digest(b"older release"))));
    assert!(err.contains(&expected));
    fs::write(&earlier, b"release").unwrap();
    assert_eq!(resolve().unwrap(), earlier.canonicalize().unwrap());
    fs::remove_file(&earlier).unwrap();
    symlink(&release, &earlier).unwrap();
    assert_eq!(resolve().unwrap(), release.canonicalize().unwrap());
    fs::remove_dir_all(root).unwrap();
}

verus! {
fn expect(value: bool) requires value { if !value { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn path_order_stops_at_the_first_executable_and_checks_its_digest() {
    let available = |i: usize| -> (result: bool)
        ensures result == (i == 1 || i == 2), { i == 1 || i == 2 };
    expect(installed_judge::first_executable(0, available).is_none());
    expect(installed_judge::first_executable(1, available).is_none());
    let first = match installed_judge::first_executable(3, available) {
        Some(i) => {
            proof { if i == 2 { assert(available.ensures((1usize,), false)); } }
            expect(i == 1);
            i
        },
        None => {
            proof { assert(available.ensures((1usize,), false)); }
            expect(false);
            return;
        },
    };
    proof { reveal_strlit("aa"); reveal_strlit("ab"); assert("aa"@[1] != "ab"@[1]); }
    let digest = if first == 1 { "aa" } else { "ab" };
    match verified_evidence::bind_digest(first, digest, "ab") {
        Err(i) => expect(i == 1),
        Ok(_) => expect(false),
    }
    match verified_evidence::bind_digest(first, digest, "aa") {
        Ok(i) => expect(i == 1),
        Err(_) => expect(false),
    }
}

}
