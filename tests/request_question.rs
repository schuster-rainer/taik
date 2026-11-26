#![cfg(target_os = "macos")]

//! The orchestrator asks about an unclear request, the request waits for the answer and is planned
//! again afterwards. A stand-in plays the worker. The service runs in its sandbox.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn factory() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
}

fn say(arguments: &[&str], text: &str) -> Output {
    let mut child = factory()
        .args(arguments)
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

// The questions of the orchestrator: the messages of the agent in the conversation.
fn questions(talk: &Path) -> usize {
    fs::read_dir(talk).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".agent.md"))
            .count()
    })
}

#[test]
fn an_unclear_request_waits_for_the_answer_of_the_user() {
    let root = tempfile::Builder::new()
        .prefix("taik-question-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = root.path().canonicalize().unwrap();
    let state = root.join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    let reply = |text: &str| {
        serde_json::json!({"type": "result", "is_error": false, "result": text,
            "usage": {"output_tokens": 1}, "total_cost_usd": 0.01, "modelUsage": {"m": {}}})
        .to_string()
    };
    fs::write(
        root.join("question.json"),
        reply("Frage: Welche Farbe soll der Titel haben?"),
    )
    .unwrap();
    fs::write(
        root.join("plan.json"),
        reply("Art: dokument\nRepo: -\nGate: standard\nArbeit: standard\nGrund: Die Farbe ist jetzt klar."),
    )
    .unwrap();
    fs::write(
        root.join("files.json"),
        reply("=== brief.md ===\nEine Seite mit blauem Titel bauen\n\n=== verify.mjs ===\nconsole.log('PASS');\n"),
    )
    .unwrap();
    // The orchestrator plans once the user said "Blau.", and asks otherwise.
    let worker = root.join("lead");
    fs::write(
        &worker,
        format!(
            "#!/bin/sh\ncase \"$*\" in *'orchestrator of a software factory'*'[user] Blau.'*) cat '{0}/plan.json';; *'orchestrator of a software factory'*) cat '{0}/question.json';; *) cat '{0}/files.json';; esac\n",
            root.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let serve = || {
        factory()
            .arg("drain")
            .arg(&state)
            .env("TAIK_LEAD_WORKER", &worker)
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap()
    };
    let home = state.to_str().unwrap();

    let asked = say(&["request", home], "Eine Seite mit farbigem Titel");
    assert_eq!(String::from_utf8_lossy(&asked.stdout).trim(), "ticket-001");
    let served = serve();
    let talk = state.join("requests/ticket-001.talk");
    assert_eq!(
        fs::read_to_string(talk.join("message-001.agent.md"))
            .unwrap_or_else(|_| panic!("{served:?}")),
        "round=0\n\nWelche Farbe soll der Titel haben?\n"
    );
    assert!(!state.join("requests/ticket-001.plan").exists());
    assert!(!state.join("requests/ticket-001.failed").exists());
    assert!(!state.join("proposals/ticket-001").exists());
    assert_eq!(questions(&talk), 1);

    // While the question is open, the service neither plans nor derives the request.
    serve();
    assert_eq!(questions(&talk), 1);
    assert!(!talk.join("message-002.agent.md").exists());
    assert!(!state.join("requests/ticket-001.plan").exists());
    assert!(!state.join("requests/ticket-001.failed").exists());
    assert!(!state.join("proposals/ticket-001").exists());

    // The answer is planned with the conversation, and the proposal follows the plan.
    let answered = say(&["reply", home, "ticket-001"], "Blau.");
    assert_eq!(answered.status.code(), Some(0), "{answered:?}");
    let served = serve();
    let plan = fs::read_to_string(state.join("requests/ticket-001.plan"))
        .unwrap_or_else(|_| panic!("{served:?}"));
    assert!(
        plan.contains("reason=Die Farbe ist jetzt klar.\n"),
        "{plan}"
    );
    assert!(
        state.join("proposals/ticket-001/brief.md").is_file(),
        "{served:?}"
    );
    assert_eq!(questions(&talk), 1);

    // A worker that keeps asking: after three answers a fourth question is no plan.
    let asked = say(&["request", home], "Noch eine Seite");
    assert_eq!(String::from_utf8_lossy(&asked.stdout).trim(), "ticket-002");
    let talk = state.join("requests/ticket-002.talk");
    for (number, answer) in ["Rot.", "Grün.", "Gelb."].into_iter().enumerate() {
        serve();
        assert!(
            talk.join(format!("message-{:03}.agent.md", 2 * number + 1))
                .is_file()
        );
        assert!(!state.join("requests/ticket-002.plan").exists());
        let answered = say(&["reply", home, "ticket-002"], answer);
        assert_eq!(answered.status.code(), Some(0), "{answered:?}");
    }
    let served = serve();
    assert!(!talk.join("message-007.agent.md").exists());
    let plan = fs::read_to_string(state.join("requests/ticket-002.plan"))
        .unwrap_or_else(|_| panic!("{served:?}"));
    assert!(
        plan.contains("work=standard\n") && plan.contains("keinen gültigen Plan"),
        "{plan}"
    );
    assert!(
        state.join("proposals/ticket-002/brief.md").is_file(),
        "{served:?}"
    );
    // The fourth question was not kept.
    assert_eq!(questions(&talk), 3);
}
