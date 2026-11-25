#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn a_proposed_job_waits_until_it_is_approved() {
    let root = tempfile::Builder::new()
        .prefix("taik-proposal-")
        .tempdir_in("/tmp")
        .unwrap();
    let factory = || Command::new(env!("CARGO_BIN_EXE_taik-seed"));
    let state = root.path().join("state");
    fs::create_dir_all(state.join("jobs/job-origin")).unwrap();
    let record = state.join("jobs/job-origin/round-01.run");
    fs::write(
        &record,
        "decision=accepted\nkind=software\ncontinuation=Add a title to the page.\nadvance=false\nend\n",
    )
    .unwrap();
    let worker = root.path().join("lead");
    let answer = |text: &str| {
        let body = serde_json::json!({"type": "result", "is_error": false, "result": text,
            "usage": {"output_tokens": 1}, "total_cost_usd": 0.01, "modelUsage": {"m": {}}});
        fs::write(root.path().join("answer.json"), body.to_string()).unwrap();
        fs::write(
            &worker,
            format!(
                "#!/bin/sh\ncat '{}'\n",
                root.path().join("answer.json").display()
            ),
        )
        .unwrap();
        fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    };
    let propose = |name: &str| {
        factory()
            .arg("propose")
            .arg(&state)
            .arg(name)
            .arg(&record)
            .env("TAIK_LEAD_WORKER", &worker)
            .output()
            .unwrap()
    };

    answer("=== brief.md ===\nEine Seite mit Titel bauen\n\nMehr Text.\n");
    let incomplete = propose("ohne-gate");
    assert_eq!(incomplete.status.code(), Some(2), "{incomplete:?}");
    assert!(!state.join("proposals/ohne-gate").exists());

    answer(
        "=== brief.md ===\nEine Seite mit Titel bauen\n=== verify.mjs ===\nthis is not ( javascript\n",
    );
    let broken = propose("kaputtes-gate");
    assert_eq!(broken.status.code(), Some(2), "{broken:?}");
    assert!(!state.join("proposals/kaputtes-gate").exists());

    answer(
        "=== brief.md ===\nEine Seite mit Titel bauen\n\nDie Seite hat einen Titel.\n=== verify.mjs ===\nimport assert from 'node:assert/strict';\nconsole.log('PASS: ok');\n",
    );
    let proposed = propose("titel");
    assert_eq!(proposed.status.code(), Some(0), "{proposed:?}");
    let proposal = state.join("proposals/titel");
    assert!(
        fs::read_to_string(proposal.join("brief.md"))
            .unwrap()
            .starts_with("Eine Seite mit Titel bauen\n")
    );
    assert!(
        fs::read_to_string(proposal.join("verify.mjs"))
            .unwrap()
            .contains("PASS: ok")
    );
    let origin = fs::read_to_string(proposal.join("origin.txt")).unwrap();
    assert!(
        origin.contains("proposed_by=claude-cli") && origin.contains("round-01.run"),
        "{origin}"
    );
    assert!(
        !state.join("jobs/job-titel").exists(),
        "a proposal must not start work"
    );
    assert_eq!(
        propose("titel").status.code(),
        Some(2),
        "a name is used once"
    );

    let reflection =
        fs::read_to_string(state.join("jobs/job-origin/reflection-titel.run")).unwrap();
    assert!(
        reflection.contains("\nworker_cost_usd=0.01\n"),
        "{reflection}"
    );
    assert!(reflection.contains("outcome=proposal "), "{reflection}");

    answer("STOP: Aus dem Protokoll folgt kein weiterer Auftrag.");
    let stopped = propose("nichts");
    assert_eq!(stopped.status.code(), Some(0), "{stopped:?}");
    assert_eq!(String::from_utf8_lossy(&stopped.stdout).trim(), "stop");
    assert!(!state.join("proposals/nichts").exists());
    let note = fs::read_to_string(state.join("jobs/job-origin/reflection-nichts.run")).unwrap();
    assert!(
        note.contains("outcome=stop: Aus dem Protokoll folgt kein weiterer Auftrag."),
        "{note}"
    );

    let approved = factory()
        .arg("approve")
        .arg(&state)
        .arg("titel")
        .output()
        .unwrap();
    assert_eq!(approved.status.code(), Some(0), "{approved:?}");
    let job = state.join("jobs/job-titel");
    assert_eq!(
        fs::read(job.join("submission.gate.mjs")).unwrap(),
        fs::read(proposal.join("verify.mjs")).unwrap()
    );
    assert!(proposal.join("approved.txt").exists());
    assert!(
        !job.join("round-01.run").exists(),
        "approval queues the job and does not run it"
    );
}
