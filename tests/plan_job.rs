#![cfg(target_os = "macos")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn state_in(root: &Path) -> PathBuf {
    let state = root.join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    fs::create_dir_all(state.join("proposals")).unwrap();
    state
}

fn proposal(state: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let folder = state.join("proposals").join(name);
    fs::create_dir_all(&folder).unwrap();
    for (file, text) in files {
        fs::write(folder.join(file), text).unwrap();
    }
    folder
}

fn approve(state: &Path, name: &str) -> PathBuf {
    let output = Command::new(env!("CARGO_BIN_EXE_taik-seed"))
        .arg("approve")
        .arg(state)
        .arg(name)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let job = state.join("jobs").join(format!("job-{name}"));
    assert!(job.is_dir(), "{output:?}");
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(
        printed.trim().ends_with(&format!("job-{name}")),
        "{printed}"
    );
    assert!(
        state
            .join("proposals")
            .join(name)
            .join("approved.txt")
            .is_file()
    );
    job
}

fn git(repo: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
}

const BRIEF: &str = "Eine Seite mit Titel bauen\n\nDie Seite hat einen Titel.\n";
const GATE: &str = "import assert from 'node:assert/strict';\nconsole.log('PASS: ok');\n";

#[test]
fn a_document_job_keeps_the_plan_of_its_proposal() {
    let root = tempfile::Builder::new()
        .prefix("taik-plan-job-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = state_in(root.path());
    let plan = "kind=dokument\nrepo=\nrepo_path=\ngate=standard\nwork=standard\nreason=Eine Seite.\nunbekannt=bleibt erhalten\n";
    proposal(
        &state,
        "titel",
        &[
            ("brief.md", BRIEF),
            ("verify.mjs", GATE),
            ("plan.txt", plan),
        ],
    );
    let job = approve(&state, "titel");
    assert_eq!(fs::read(job.join("plan.txt")).unwrap(), plan.as_bytes());
}

#[test]
fn a_session_job_keeps_the_plan_of_its_proposal() {
    let root = tempfile::Builder::new()
        .prefix("taik-plan-job-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = state_in(root.path());
    let repo = root.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    fs::write(repo.join("README.md"), "Ein kleines Repository.\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-q",
            "-m",
            "Anfang",
        ],
    );
    let plan = format!(
        "kind=sitzung\nrepo=klein\nrepo_path={}\ngate=standard\nwork=standard\nreason=Eine Sitzung.\nunbekannt=bleibt erhalten\n",
        repo.display()
    );
    proposal(
        &state,
        "sitzung",
        &[
            ("brief.md", BRIEF),
            ("gate.sh", "echo 'PASS: ok'\n"),
            ("plan.txt", &plan),
        ],
    );
    let job = approve(&state, "sitzung");
    assert_eq!(fs::read(job.join("plan.txt")).unwrap(), plan.as_bytes());
}

#[test]
fn a_proposal_without_a_plan_is_approved_without_one() {
    let root = tempfile::Builder::new()
        .prefix("taik-plan-job-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = state_in(root.path());
    proposal(
        &state,
        "ohne-plan",
        &[("brief.md", BRIEF), ("verify.mjs", GATE)],
    );
    let job = approve(&state, "ohne-plan");
    assert!(!job.join("plan.txt").exists());
    assert!(job.join("submission.brief.md").is_file());
}
