#![cfg(target_os = "macos")]

use std::fs;
use std::path::Path;
use std::process::Command;

fn field<'a>(record: &'a str, key: &str) -> &'a str {
    record
        .lines()
        .filter_map(|line| line.split_once('='))
        .find_map(|(name, value)| (name == key).then_some(value))
        .unwrap_or_else(|| panic!("missing {key} in\n{record}"))
}

fn promote(installed: &Path, job: &Path, brief: &Path, gate: &Path) -> std::process::Output {
    Command::new(installed)
        .arg("promote")
        .arg(job)
        .arg(brief)
        .arg(gate)
        .arg(installed)
        .arg("test-author")
        .output()
        .unwrap()
}

#[test]
fn the_installed_binary_judges_installs_and_keeps_its_predecessor() {
    let root = tempfile::tempdir().unwrap();
    let built = Path::new(env!("CARGO_BIN_EXE_taik-seed"));
    let installed = root.path().join("installed-taik-seed");
    // The installed predecessor differs from the candidate by trailing bytes that the loader ignores.
    let mut predecessor = fs::read(built).unwrap();
    predecessor.extend_from_slice(b"\0predecessor");
    fs::write(&installed, &predecessor).unwrap();
    fs::set_permissions(&installed, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Replace the installed binary\n").unwrap();
    let write = |name: &str, body: String| {
        let path = root.path().join(name);
        fs::write(&path, body).unwrap();
        path
    };
    let job = root.path().join("job-backend");

    // A binary that is not the installed one may not judge.
    let foreign = Command::new(built)
        .arg("promote")
        .arg(&job)
        .arg(&brief)
        .arg(&brief)
        .arg(&installed)
        .arg("test-author")
        .output()
        .unwrap();
    assert_eq!(foreign.status.code(), Some(2), "{foreign:?}");
    assert!(!job.exists());

    let gate = write(
        "gate.sh",
        format!("echo building\ncp '{}' \"$TAIK_CANDIDATE_OUT\"\necho 'PASS: built'\n", built.display()),
    );
    let accepted = promote(&installed, &job, &brief, &gate);
    assert_eq!(accepted.status.code(), Some(0), "{accepted:?}");
    let record = fs::read_to_string(job.join("round-01.run")).unwrap();
    assert_eq!(field(&record, "decision"), "accepted");
    assert_eq!(field(&record, "worker"), "test-author");
    assert!(field(&record, "decided_by").starts_with("the installed taik-seed "));
    assert!(field(&record, "checks").starts_with("PASS: built; candidate seeded"));
    assert_eq!(field(&record, "artifact"), "round-01-taik-seed");
    assert_eq!(fs::read(&installed).unwrap(), fs::read(built).unwrap());
    assert_eq!(fs::read(job.join("round-01-taik-seed")).unwrap(), fs::read(built).unwrap());
    assert_eq!(fs::read(job.join("round-01.rollback-taik-seed")).unwrap(), predecessor);
    assert!(fs::read_to_string(job.join("round-01.checks.log")).unwrap().contains("building"));

    // The gate is pinned: a different gate cannot judge in the same job.
    let other = write("other.sh", "exit 0\n".into());
    let changed = promote(&installed, &job, &brief, &other);
    assert_eq!(changed.status.code(), Some(2), "{changed:?}");
    assert!(!job.join("round-02.run").exists());

    // A failing gate installs nothing.
    let before = fs::read(&installed).unwrap();
    let failing_job = root.path().join("job-failing");
    let failing = write("failing.sh", "echo 'test result: FAILED'\nexit 1\n".into());
    let rejected = promote(&installed, &failing_job, &brief, &failing);
    assert_eq!(rejected.status.code(), Some(1), "{rejected:?}");
    let record = fs::read_to_string(failing_job.join("round-01.run")).unwrap();
    assert_eq!(field(&record, "decision"), "rejected");
    assert!(field(&record, "checks").contains("test result: FAILED"));
    assert_eq!(field(&record, "artifact"), "");
    assert_eq!(fs::read(&installed).unwrap(), before);
    assert!(!failing_job.join("round-01-taik-seed").exists());

    // A gate that passes a candidate which cannot do the seed work installs nothing either.
    let hollow_job = root.path().join("job-hollow");
    let hollow = write(
        "hollow.sh",
        "printf '#!/bin/sh\\nexit 0\\n' > \"$TAIK_CANDIDATE_OUT\"\necho 'PASS: claimed'\n".into(),
    );
    let rejected = promote(&installed, &hollow_job, &brief, &hollow);
    assert_eq!(rejected.status.code(), Some(1), "{rejected:?}");
    let record = fs::read_to_string(hollow_job.join("round-01.run")).unwrap();
    assert_eq!(field(&record, "decision"), "rejected");
    assert!(field(&record, "checks").contains("seed record"), "{record}");
    assert_eq!(fs::read(&installed).unwrap(), before);
}

#[test]
fn a_running_service_leaves_the_next_job_to_a_replaced_binary() {
    let root = tempfile::tempdir().unwrap();
    let built = Path::new(env!("CARGO_BIN_EXE_taik-seed"));
    let installed = root.path().join("taik-seed");
    fs::copy(built, &installed).unwrap();
    let state = root.path().join("state");
    let seed = root.path().join("seed.run");
    assert!(Command::new(built).arg("seed").arg(&seed).output().unwrap().status.success());
    let mut service = Command::new(&installed)
        .arg("serve")
        .arg(&state)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let wait_for = |path: &Path| {
        for _ in 0..200 {
            if path.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        panic!("{} did not appear", path.display());
    };
    wait_for(&state.join("service.lock"));
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(service.try_wait().unwrap().is_none(), "the service ended before any change");

    let next = root.path().join("taik-seed.next");
    fs::copy(built, &next).unwrap();
    fs::rename(&next, &installed).unwrap();
    let queued = Command::new(built).arg("queue").arg(&state).arg(&seed).output().unwrap();
    assert!(queued.status.success(), "{queued:?}");
    let job = std::path::PathBuf::from(String::from_utf8(queued.stdout).unwrap().trim());

    let mut status = None;
    for _ in 0..200 {
        status = service.try_wait().unwrap();
        if status.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let _ = service.kill();
    assert_eq!(status.and_then(|status| status.code()), Some(0), "the service kept running");
    assert!(!job.join("round-01.run").exists(), "the replaced binary still did the work");

    // The successor finishes the job.
    let drained = Command::new(&installed).arg("drain").arg(&state).output().unwrap();
    assert!(drained.status.success(), "{drained:?}");
    assert!(job.join("round-01.run").exists());
}
