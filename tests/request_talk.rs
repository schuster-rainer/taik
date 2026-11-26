#![cfg(target_os = "macos")]

//! Answers of the user to a question of the orchestrator about a request. No worker and no sandbox:
//! the test lays out request and conversation itself.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn factory() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
}

fn reply(state: &Path, name: &str, text: &str) -> Output {
    let mut child = factory()
        .arg("reply")
        .arg(state)
        .arg(name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn shown(state: &Path, request: &Path) -> String {
    let output = factory()
        .arg("orchestrator-prompt")
        .arg(state)
        .arg(request)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn state() -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("taik-request-talk-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = root.path().join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    fs::create_dir_all(state.join("requests")).unwrap();
    (root, state)
}

fn wish(state: &Path, name: &str) -> std::path::PathBuf {
    let request = state.join("requests").join(format!("{name}.run"));
    fs::write(
        &request,
        format!(
            "decision=observed\nidentity={name}\nintent=Eine Seite mit Titel\nobservation=\ncontinuation=Derive one job.\nend\n"
        ),
    )
    .unwrap();
    request
}

#[test]
fn an_answer_to_a_question_of_the_orchestrator_goes_into_the_conversation_of_the_request() {
    let (_root, state) = state();
    let request = wish(&state, "ticket-001");
    let talk = state.join("requests/ticket-001.talk");

    // No conversation: nothing is shown and a reply has no place to go.
    assert!(!shown(&state, &request).contains("CONVERSATION ABOUT THIS WORK ORDER"));
    assert_eq!(reply(&state, "ticket-001", "Blau.").status.code(), Some(2));
    assert!(!talk.join("message-001.user.md").exists());

    fs::create_dir(&talk).unwrap();
    fs::write(
        talk.join("message-001.agent.md"),
        "round=0\n\nWelche Farbe soll der Titel haben?\n",
    )
    .unwrap();
    let jobs_before = fs::read_dir(state.join("jobs")).unwrap().count();
    let answered = reply(&state, "ticket-001", "  Blau.\n");
    assert_eq!(answered.status.code(), Some(0), "{answered:?}");
    let message = talk.join("message-002.user.md");
    assert_eq!(fs::read_to_string(&message).unwrap(), "round=0\n\nBlau.\n");
    assert_eq!(
        String::from_utf8_lossy(&answered.stdout).trim(),
        message.to_str().unwrap()
    );
    // The wake-up leaves nothing behind in the job directory.
    assert_eq!(
        fs::read_dir(state.join("jobs")).unwrap().count(),
        jobs_before
    );
    assert!(!state.join("proposals/ticket-001").exists());

    // The user wrote last: a second reply is refused and written nowhere.
    let again = reply(&state, "ticket-001", "Doch rot.");
    assert_eq!(again.status.code(), Some(2), "{again:?}");
    assert!(!talk.join("message-003.user.md").exists());
    assert!(!talk.join("message-003.agent.md").exists());

    let prompt = shown(&state, &request);
    assert!(
        prompt.contains("CONVERSATION ABOUT THIS WORK ORDER")
            && prompt.contains("[you] Welche Farbe soll der Titel haben?\n[user] Blau.\n"),
        "{prompt}"
    );
}

#[test]
fn without_a_request_or_with_a_proposal_reply_behaves_as_before() {
    let (_root, state) = state();
    // No request at all.
    assert_eq!(reply(&state, "ticket-007", "x").status.code(), Some(2));
    assert!(!state.join("requests/ticket-007.talk").exists());

    // A waiting question, but a proposal exists: the comment goes into the proposal.
    wish(&state, "ticket-002");
    let talk = state.join("requests/ticket-002.talk");
    fs::create_dir(&talk).unwrap();
    fs::write(
        talk.join("message-001.agent.md"),
        "round=0\n\nWelches Repo?\n",
    )
    .unwrap();
    let proposal = state.join("proposals/ticket-002");
    fs::create_dir_all(&proposal).unwrap();
    fs::write(proposal.join("brief.md"), "Eine Seite mit Titel\n").unwrap();
    let comment = reply(&state, "ticket-002", "Bitte dunkelblau.");
    assert_eq!(comment.status.code(), Some(0), "{comment:?}");
    assert_eq!(
        fs::read_to_string(proposal.join("message-001.user.md")).unwrap(),
        "round=0\n\nBitte dunkelblau.\n"
    );
    assert!(!talk.join("message-002.user.md").exists());

    // A proposal folder without a brief: neither the request nor the proposal takes the reply.
    wish(&state, "ticket-003");
    let talk = state.join("requests/ticket-003.talk");
    fs::create_dir(&talk).unwrap();
    fs::write(
        talk.join("message-001.agent.md"),
        "round=0\n\nWelches Repo?\n",
    )
    .unwrap();
    fs::create_dir_all(state.join("proposals/ticket-003")).unwrap();
    assert_eq!(reply(&state, "ticket-003", "x").status.code(), Some(2));
    assert!(!talk.join("message-002.user.md").exists());

    // A request without a conversation and a job name keep their old answers.
    wish(&state, "ticket-004");
    assert_eq!(reply(&state, "ticket-004", "x").status.code(), Some(2));
    assert_eq!(reply(&state, "job-ticket-004", "x").status.code(), Some(2));
    assert!(!state.join("requests/ticket-004.talk").exists());
}
