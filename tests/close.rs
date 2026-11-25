use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn close(state: &Path, job: &str, reason: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_taik-seed"))
        .arg("close")
        .arg(state)
        .arg(job)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The command may refuse before it reads; a closed pipe is then expected.
    let _ = child.stdin.take().unwrap().write_all(reason);
    child.wait_with_output().unwrap()
}

#[test]
fn a_job_is_closed_once_with_exactly_the_given_reason() {
    let state = tempfile::tempdir().unwrap();
    let job = state.path().join("jobs").join("job-demo");
    fs::create_dir_all(&job).unwrap();
    let closed = job.join("closed.md");

    let reason = "Erledigt: nicht mehr nötig.\n".as_bytes();
    let output = close(state.path(), "job-demo", reason);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read(&closed).unwrap(), reason);

    let again = close(state.path(), "job-demo", b"anders");
    assert_eq!(again.status.code(), Some(2));
    assert_eq!(fs::read(&closed).unwrap(), reason);
}

#[test]
fn the_reason_is_between_1_and_2000_bytes() {
    let state = tempfile::tempdir().unwrap();
    let job = state.path().join("jobs").join("job-demo");
    fs::create_dir_all(&job).unwrap();
    let closed = job.join("closed.md");

    assert_eq!(close(state.path(), "job-demo", b"").status.code(), Some(2));
    assert!(!closed.exists());
    assert_eq!(
        close(state.path(), "job-demo", &[b'x'; 2001]).status.code(),
        Some(2)
    );
    assert!(!closed.exists());
    assert_eq!(fs::read_dir(&job).unwrap().count(), 0);

    assert_eq!(
        close(state.path(), "job-demo", &[b'x'; 2000]).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&closed).unwrap(), vec![b'x'; 2000]);
}

#[test]
fn an_unknown_job_is_refused() {
    let state = tempfile::tempdir().unwrap();
    fs::create_dir_all(state.path().join("jobs")).unwrap();
    let output = close(state.path(), "job-missing", b"Grund");
    assert_eq!(output.status.code(), Some(2));
    assert!(!state.path().join("jobs").join("job-missing").exists());
}
