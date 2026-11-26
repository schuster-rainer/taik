#![cfg(target_os = "macos")]

//! The user declines a waiting proposal or withdraws a request. The test lays out proposal and
//! request itself in the state folder. No worker and no sandbox run.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn factory() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
}

fn say(arguments: &[&str], text: &[u8]) -> Output {
    let mut child = factory()
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A command that refuses early may close its input before all of it is read.
    let _ = child.stdin.take().unwrap().write_all(text);
    child.wait_with_output().unwrap()
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
        .prefix("taik-decline-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = root.path().canonicalize().unwrap().join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    (root, state)
}

fn proposal(state: &Path, name: &str) -> PathBuf {
    let folder = state.join("proposals").join(name);
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("brief.md"),
        "Eine Seite mit blauem Titel bauen\n\nDer Titel ist blau.\n",
    )
    .unwrap();
    fs::write(folder.join("verify.mjs"), "console.log('PASS: ok');\n").unwrap();
    fs::write(
        folder.join("origin.txt"),
        "derived_from=x\nproposed_by=test\nstate=waiting for approval\n",
    )
    .unwrap();
    folder
}

// A request whose orchestrator asked a question that nobody has answered yet.
fn asked_request(state: &Path, name: &str) -> PathBuf {
    let requests = state.join("requests");
    fs::create_dir_all(&requests).unwrap();
    let request = requests.join(format!("{name}.run"));
    fs::write(
        &request,
        format!("decision=observed\nidentity={name}\nintent=Eine Seite\nobservation=\ncontinuation=Derive one job.\nend\n"),
    )
    .unwrap();
    let talk = requests.join(format!("{name}.talk"));
    fs::create_dir_all(&talk).unwrap();
    fs::write(
        talk.join("message-001.agent.md"),
        "round=0\n\nWelche Farbe soll der Titel haben?\n",
    )
    .unwrap();
    request
}

fn jobs(state: &Path) -> usize {
    fs::read_dir(state.join("jobs")).unwrap().count()
}

#[test]
fn a_waiting_proposal_is_declined_once_and_cannot_be_approved() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    let folder = proposal(&state, "titel");
    let reason = "Das brauchen wir nicht.\nZu klein.\n";

    let declined = say(&["decline", home, "titel"], reason.as_bytes());
    assert_eq!(declined.status.code(), Some(0), "{declined:?}");
    assert_eq!(fs::read(folder.join("declined.md")).unwrap(), reason.as_bytes());

    // A second decline is refused and the first reason stays.
    let before = snapshot(&state);
    let again = say(&["decline", home, "titel"], b"Anders.");
    assert_eq!(again.status.code(), Some(2), "{again:?}");
    assert_eq!(snapshot(&state), before);

    // A declined proposal cannot be approved: no job, no approval.
    let approved = factory()
        .arg("approve")
        .arg(&state)
        .arg("titel")
        .output()
        .unwrap();
    assert_eq!(approved.status.code(), Some(2), "{approved:?}");
    assert!(!folder.join("approved.txt").exists());
    assert_eq!(jobs(&state), 0);
    assert_eq!(snapshot(&state), before);
}

#[test]
fn a_request_is_withdrawn_once_and_takes_no_reply() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    asked_request(&state, "ticket-001");
    let reason = "Hat sich erledigt.";

    let withdrawn = say(&["decline", home, "ticket-001"], reason.as_bytes());
    assert_eq!(withdrawn.status.code(), Some(0), "{withdrawn:?}");
    assert_eq!(
        fs::read(state.join("requests/ticket-001.withdrawn")).unwrap(),
        reason.as_bytes()
    );
    assert!(!state.join("proposals/ticket-001").exists());

    // The answer to the open question is refused and nothing is written.
    let before = snapshot(&state);
    let replied = say(&["reply", home, "ticket-001"], b"Blau.");
    assert_eq!(replied.status.code(), Some(2), "{replied:?}");
    assert_eq!(snapshot(&state), before);
    assert!(
        !state
            .join("requests/ticket-001.talk/message-002.user.md")
            .exists()
    );

    // A second withdrawal is refused and the first reason stays.
    let again = say(&["decline", home, "ticket-001"], b"Nochmal.");
    assert_eq!(again.status.code(), Some(2), "{again:?}");
    assert_eq!(snapshot(&state), before);
}

#[test]
fn a_request_that_is_not_withdrawn_still_takes_its_answer() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    asked_request(&state, "ticket-001");
    let replied = say(&["reply", home, "ticket-001"], b"Blau.");
    assert_eq!(replied.status.code(), Some(0), "{replied:?}");
    assert!(
        state
            .join("requests/ticket-001.talk/message-002.user.md")
            .is_file()
    );
}

#[test]
fn wrong_names_reasons_and_states_are_refused_without_writing() {
    let (_root, state) = state();
    let home = state.to_str().unwrap();
    proposal(&state, "offen");
    proposal(&state, "frei");
    asked_request(&state, "ticket-001");

    // An approved proposal is no longer declined.
    let approved = factory()
        .arg("approve")
        .arg(&state)
        .arg("frei")
        .output()
        .unwrap();
    assert_eq!(approved.status.code(), Some(0), "{approved:?}");
    assert!(state.join("proposals/frei/approved.txt").is_file());

    let before = snapshot(&state);
    let long = "x".repeat(2001);
    let long_name = "a".repeat(33);
    let refused: [(&str, &[u8]); 10] = [
        ("unbekannt", b"Nein."),
        ("Offen", b"Nein."),
        ("of_fen", b"Nein."),
        ("../offen", b"Nein."),
        (&long_name, b"Nein."),
        ("", b"Nein."),
        ("offen", b""),
        ("offen", long.as_bytes()),
        ("ticket-001", b""),
        ("frei", b"Nein."),
    ];
    for (name, reason) in refused {
        let output = say(&["decline", home, name], reason);
        assert_eq!(output.status.code(), Some(2), "{name}: {output:?}");
        assert_eq!(snapshot(&state), before, "{name} wrote something");
    }
    assert!(!state.join("proposals/offen/declined.md").exists());
    assert!(!state.join("proposals/frei/declined.md").exists());
    assert!(!state.join("requests/ticket-001.withdrawn").exists());

    // A reason of exactly 2000 bytes is kept byte for byte, even when it is not UTF-8.
    let mut reason = vec![b'y'; 1998];
    reason.extend_from_slice(&[0xff, b'\n']);
    let kept = say(&["decline", home, "offen"], &reason);
    assert_eq!(kept.status.code(), Some(0), "{kept:?}");
    assert_eq!(
        fs::read(state.join("proposals/offen/declined.md")).unwrap(),
        reason
    );
}
