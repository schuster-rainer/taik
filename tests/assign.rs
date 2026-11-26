#![cfg(target_os = "macos")]

//! The user assigns the agent that works a waiting proposal before approving it. The test lays out
//! proposal, plan and crew itself in the state folder. No worker and no sandbox run.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn factory() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
}

fn run(arguments: &[&str]) -> Output {
    factory().args(arguments).output().unwrap()
}

// Every file and folder below the state with its content: what a refused command must leave alone.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut found = BTreeMap::new();
    let mut open = vec![root.to_path_buf()];
    while let Some(folder) = open.pop() {
        for entry in fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                found.insert(relative, None);
                open.push(path);
            } else {
                found.insert(relative, Some(fs::read(&path).unwrap()));
            }
        }
    }
    found
}

fn state() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("taik-assign-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = root.path().canonicalize().unwrap().join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    let crew = state.join("crew");
    fs::create_dir_all(&crew).unwrap();
    for (name, harness) in [
        ("anna", "claude"),
        ("bert", "pi"),
        ("carl", "codex"),
        ("dora", "claude"),
    ] {
        fs::write(
            crew.join(format!("{name}.agent")),
            format!("harness={harness}\nmodel=m\neffort=high\nend\n"),
        )
        .unwrap();
    }
    (root, state)
}

const BRIEF: &str = "Eine Seite mit blauem Titel bauen\n\nDer Titel ist blau.\n";

fn document(state: &Path, name: &str, gate: &str, work: &str) -> PathBuf {
    let folder = state.join("proposals").join(name);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("brief.md"), BRIEF).unwrap();
    fs::write(folder.join("verify.mjs"), "console.log('PASS: ok');\n").unwrap();
    fs::write(
        folder.join("origin.txt"),
        "derived_from=x\nproposed_by=test\nstate=waiting for approval\n",
    )
    .unwrap();
    fs::write(
        folder.join("plan.txt"),
        format!("kind=dokument\nrepo=\nrepo_path=\ngate={gate}\nwork={work}\nreason=Test.\n"),
    )
    .unwrap();
    folder
}

fn git(repo: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
}

fn session(state: &Path, name: &str, gate: &str, work: &str) -> PathBuf {
    let repo = state.parent().unwrap().join("repo");
    if !repo.exists() {
        fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        fs::write(repo.join("README.md"), "Ein Test.\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-q", "-m", "start"]);
    }
    let folder = state.join("proposals").join(name);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("brief.md"), BRIEF).unwrap();
    fs::write(folder.join("gate.sh"), "echo 'PASS: ok'\n").unwrap();
    fs::write(
        folder.join("origin.txt"),
        "derived_from=x\nproposed_by=test\nstate=waiting for approval\n",
    )
    .unwrap();
    fs::write(
        folder.join("plan.txt"),
        format!(
            "kind=sitzung\nrepo=probe\nrepo_path={}\ngate={gate}\nwork={work}\nreason=Test.\n",
            repo.display()
        ),
    )
    .unwrap();
    folder
}

fn approve(state: &Path, name: &str) -> PathBuf {
    let output = run(&["approve", state.to_str().unwrap(), name]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let job = state.join("jobs").join(format!("job-{name}"));
    assert!(job.is_dir(), "{output:?}");
    job
}

#[test]
fn an_assignment_is_written_and_replaced_while_the_proposal_waits() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    let folder = document(&state, "titel", "anna", "bert");
    let plan = fs::read(folder.join("plan.txt")).unwrap();
    for agent in ["carl", "standard", "bert"] {
        let output = run(&["assign", home, "titel", agent]);
        assert_eq!(output.status.code(), Some(0), "{agent}: {output:?}");
        assert_eq!(
            fs::read_to_string(folder.join("worker")).unwrap(),
            format!("{agent}\n")
        );
    }
    assert_eq!(fs::read(folder.join("plan.txt")).unwrap(), plan);
    // No temporary file stays behind.
    let names: Vec<String> = fs::read_dir(&folder)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().all(|name| !name.starts_with('.')), "{names:?}");
}

#[test]
fn an_approved_job_takes_the_assigned_agent() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    let folder = document(&state, "titel", "anna", "bert");
    let output = run(&["assign", home, "titel", "carl"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let job = approve(&state, "titel");
    assert_eq!(fs::read_to_string(job.join("agent")).unwrap(), "carl\n");
    assert_eq!(
        fs::read(job.join("worker")).unwrap(),
        fs::read(folder.join("worker")).unwrap()
    );
    assert_eq!(
        fs::read(job.join("plan.txt")).unwrap(),
        fs::read(folder.join("plan.txt")).unwrap()
    );
    assert!(
        fs::read_to_string(folder.join("plan.txt"))
            .unwrap()
            .contains("work=bert\n")
    );
}

#[test]
fn the_standard_worker_means_no_agent_file() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    let folder = document(&state, "titel", "anna", "bert");
    let output = run(&["assign", home, "titel", "standard"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let job = approve(&state, "titel");
    assert!(!job.join("agent").exists());
    assert_eq!(
        fs::read_to_string(job.join("worker")).unwrap(),
        "standard\n"
    );
    assert_eq!(
        fs::read(job.join("plan.txt")).unwrap(),
        fs::read(folder.join("plan.txt")).unwrap()
    );
}

#[test]
fn without_an_assignment_the_plan_decides() {
    let (_root, state) = state();
    document(&state, "titel", "anna", "bert");
    let job = approve(&state, "titel");
    assert_eq!(fs::read_to_string(job.join("agent")).unwrap(), "bert\n");
    assert!(!job.join("worker").exists());
}

#[test]
fn a_session_takes_an_assigned_agent_on_the_claude_harness() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    let folder = session(&state, "sitzung", "anna", "standard");
    let output = run(&["assign", home, "sitzung", "dora"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let job = approve(&state, "sitzung");
    assert_eq!(fs::read_to_string(job.join("agent")).unwrap(), "dora\n");
    assert_eq!(fs::read_to_string(job.join("worker")).unwrap(), "dora\n");
    assert_eq!(
        fs::read(job.join("plan.txt")).unwrap(),
        fs::read(folder.join("plan.txt")).unwrap()
    );
}

#[test]
fn wrong_names_states_and_agents_are_refused_without_writing() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    document(&state, "offen", "anna", "bert");
    document(&state, "frei", "anna", "bert");
    let declined = document(&state, "weg", "anna", "bert");
    fs::write(declined.join("declined.md"), "Nein.\n").unwrap();
    session(&state, "sitzung", "anna", "dora");
    approve(&state, "frei");

    let before = snapshot(&state);
    let long_name = "a".repeat(33);
    let refused: [(&str, &str); 15] = [
        ("unbekannt", "bert"),
        ("Offen", "bert"),
        ("of_fen", "bert"),
        ("../offen", "bert"),
        (&long_name, "bert"),
        ("", "bert"),
        ("frei", "carl"),
        ("weg", "carl"),
        ("offen", "niemand"),
        ("offen", "Bert"),
        ("offen", "../crew/bert"),
        ("offen", ""),
        ("sitzung", "bert"),
        ("sitzung", "carl"),
        ("offen", "anna"),
    ];
    for (name, agent) in refused {
        let output = run(&["assign", home, name, agent]);
        assert_eq!(output.status.code(), Some(2), "{name} {agent}: {output:?}");
        assert_eq!(snapshot(&state), before, "{name} {agent} wrote something");
    }
    // The author of the gate of a session is refused as well.
    let output = run(&["assign", home, "sitzung", "anna"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(snapshot(&state), before);
    for name in ["offen", "frei", "weg", "sitzung"] {
        assert!(!state.join("proposals").join(name).join("worker").exists());
    }
}
