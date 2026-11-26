#![cfg(target_os = "macos")]

//! A withdrawn request is neither planned nor derived by the service, also not after the user
//! answered the question of the orchestrator. A stand-in plays the worker. The service runs in its
//! sandbox.

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
    let _ = child.stdin.take().unwrap().write_all(text.as_bytes());
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

fn untouched(state: &Path, name: &str) {
    assert!(!state.join(format!("requests/{name}.plan")).exists());
    assert!(!state.join(format!("requests/{name}.failed")).exists());
    assert!(!state.join("proposals").join(name).exists());
}

#[test]
fn a_withdrawn_request_is_not_planned_even_after_an_answer() {
    let root = tempfile::Builder::new()
        .prefix("taik-withdraw-")
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

    // The orchestrator asks, the user answers and then withdraws the request.
    let asked = say(&["request", home], "Eine Seite mit farbigem Titel");
    assert_eq!(String::from_utf8_lossy(&asked.stdout).trim(), "ticket-001");
    let served = serve();
    let talk = state.join("requests/ticket-001.talk");
    assert_eq!(questions(&talk), 1, "{served:?}");
    let answered = say(&["reply", home, "ticket-001"], "Blau.");
    assert_eq!(answered.status.code(), Some(0), "{answered:?}");
    let withdrawn = say(&["decline", home, "ticket-001"], "Doch nicht.");
    assert_eq!(withdrawn.status.code(), Some(0), "{withdrawn:?}");
    for _ in 0..2 {
        serve();
        untouched(&state, "ticket-001");
        assert_eq!(questions(&talk), 1);
    }
    let refused = say(&["reply", home, "ticket-001"], "Rot.");
    assert_eq!(refused.status.code(), Some(2), "{refused:?}");

    // A request withdrawn while its question is open is not asked about again.
    let asked = say(&["request", home], "Noch eine Seite");
    assert_eq!(String::from_utf8_lossy(&asked.stdout).trim(), "ticket-002");
    let talk = state.join("requests/ticket-002.talk");
    serve();
    assert_eq!(questions(&talk), 1);
    let withdrawn = say(&["decline", home, "ticket-002"], "Erledigt.");
    assert_eq!(withdrawn.status.code(), Some(0), "{withdrawn:?}");
    serve();
    untouched(&state, "ticket-002");
    assert_eq!(questions(&talk), 1);

    // A request withdrawn before the service saw it is never planned.
    let asked = say(&["request", home], "Eine dritte Seite");
    assert_eq!(String::from_utf8_lossy(&asked.stdout).trim(), "ticket-003");
    let withdrawn = say(&["decline", home, "ticket-003"], "Zu früh.");
    assert_eq!(withdrawn.status.code(), Some(0), "{withdrawn:?}");
    serve();
    untouched(&state, "ticket-003");
    assert_eq!(questions(&state.join("requests/ticket-003.talk")), 0);
}
