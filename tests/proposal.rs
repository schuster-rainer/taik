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

    // The loop: an open intent of the Factory and a product it may change. The worker takes the
    // intent and proposes a change of the product. Gate and starting source come from the product.
    fs::write(
        state.join("jobs/job-origin/observation.run"),
        "decision=observed\nidentity=o\ncontinuation=Make the title of the page blue.\nend\n",
    )
    .unwrap();
    let page = root.path().join("page.html");
    fs::write(&page, "<!doctype html><h1>Titel</h1>\n").unwrap();
    let page_gate = root.path().join("page-gate.mjs");
    fs::write(&page_gate, "console.log('PASS: page');\n").unwrap();
    fs::create_dir_all(state.join("products")).unwrap();
    fs::write(
        state.join("products/seite.product"),
        format!(
            "about=Eine Seite\nseed={}\ngate={}\nend\n",
            page.display(),
            page_gate.display()
        ),
    )
    .unwrap();
    answer(
        "INTENT: 1\n=== product seite ===\nDen Titel der Seite blau machen\n\nNur die Farbe ändert sich.\n",
    );
    let looped = propose("titel-blau");
    assert_eq!(looped.status.code(), Some(0), "{looped:?}");
    let blue = state.join("proposals/titel-blau");
    assert_eq!(
        fs::read_to_string(blue.join("brief.md")).unwrap(),
        "Den Titel der Seite blau machen\n\nNur die Farbe ändert sich.\n"
    );
    assert_eq!(
        fs::read(blue.join("verify.mjs")).unwrap(),
        fs::read(&page_gate).unwrap()
    );
    assert_eq!(
        fs::read(blue.join("seed.html")).unwrap(),
        fs::read(&page).unwrap()
    );
    let origin = fs::read_to_string(blue.join("origin.txt")).unwrap();
    assert!(
        origin.contains("product=seite\n") && origin.contains("observation.run\n"),
        "{origin}"
    );
    let note = fs::read_to_string(state.join("jobs/job-origin/reflection-titel-blau.run")).unwrap();
    assert!(
        note.contains("\nintent_from=") && note.contains("observation.run\n"),
        "{note}"
    );
    let queued = factory()
        .arg("approve")
        .arg(&state)
        .arg("titel-blau")
        .output()
        .unwrap();
    assert_eq!(queued.status.code(), Some(0), "{queued:?}");
    assert_eq!(
        fs::read(state.join("jobs/job-titel-blau/submission.seed.html")).unwrap(),
        fs::read(&page).unwrap()
    );
    // A product the Factory does not know is refused.
    answer("=== product unbekannt ===\nEtwas ändern\n");
    assert_eq!(propose("fremd").status.code(), Some(2));
    assert!(!state.join("proposals/fremd").exists());

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

#[test]
fn a_request_of_the_user_becomes_a_proposal_and_keeps_what_was_said() {
    use std::io::Write;
    use std::process::Stdio;
    let root = tempfile::Builder::new()
        .prefix("taik-request-")
        .tempdir_in("/tmp")
        .unwrap();
    let factory = || Command::new(env!("CARGO_BIN_EXE_taik-seed"));
    let state = root.path().join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
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
    let say = |arguments: &[&str], text: &str| {
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
    };
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

    assert_eq!(say(&["request", home], "  \n").status.code(), Some(2));
    let asked = say(
        &["request", home],
        "Eine Seite mit Titel\n\nSie soll blau sein.\n",
    );
    assert_eq!(asked.status.code(), Some(0), "{asked:?}");
    assert_eq!(String::from_utf8_lossy(&asked.stdout).trim(), "ticket-001");
    let record = fs::read_to_string(state.join("requests/ticket-001.run")).unwrap();
    assert!(
        record.contains("intent=Eine Seite mit Titel\n")
            && record.contains("observation=Sie soll blau sein.\n"),
        "{record}"
    );
    assert!(
        !state.join("proposals").exists(),
        "a request alone starts nothing"
    );

    // The service derives the proposal with its own worker.
    answer(
        "=== brief.md ===\nEine blaue Seite mit Titel bauen\n\n=== verify.mjs ===\nconsole.log('PASS');\n",
    );
    let served = serve();
    let proposal = state.join("proposals/ticket-001");
    assert!(proposal.join("brief.md").is_file(), "{served:?}");
    assert!(state.join("requests/reflection-ticket-001.run").is_file());
    // A second pass derives nothing again.
    answer("STOP: nothing");
    serve();
    assert!(!state.join("requests/ticket-001.failed").exists());

    // A request the worker declines is answered, not repeated.
    assert_eq!(
        String::from_utf8_lossy(&say(&["request", home], "Unklar").stdout).trim(),
        "ticket-002"
    );
    serve();
    let declined = fs::read_to_string(state.join("requests/reflection-ticket-002.run")).unwrap();
    assert!(declined.contains("outcome=stop: nothing"), "{declined}");
    assert!(!state.join("proposals/ticket-002").exists());

    // A comment on the waiting proposal stays with it and opens the conversation of the job.
    let comment = say(&["reply", home, "ticket-001"], "Bitte dunkelblau.");
    assert_eq!(comment.status.code(), Some(0), "{comment:?}");
    assert!(proposal.join("message-001.user.md").is_file());
    assert_eq!(
        say(&["reply", home, "ticket-002"], "x").status.code(),
        Some(2)
    );
    let approved = factory()
        .args(["approve", home, "ticket-001"])
        .output()
        .unwrap();
    assert_eq!(approved.status.code(), Some(0), "{approved:?}");
    assert_eq!(
        fs::read_to_string(state.join("jobs/job-ticket-001/message-001.user.md")).unwrap(),
        "round=0\n\nBitte dunkelblau.\n"
    );
    assert_eq!(
        say(&["reply", home, "ticket-001"], "zu spät").status.code(),
        Some(2)
    );
}

#[test]
fn a_rejected_round_gets_a_short_analysis_once() {
    let root = tempfile::Builder::new()
        .prefix("taik-analysis-")
        .tempdir_in("/tmp")
        .unwrap();
    let factory = || Command::new(env!("CARGO_BIN_EXE_taik-seed"));
    let job = root.path().join("jobs/job-seite");
    fs::create_dir_all(&job).unwrap();
    fs::write(job.join("submission.brief.md"), "Eine Seite mit Titel\n").unwrap();
    fs::write(job.join("submission.gate.mjs"), "// checks the title\n").unwrap();
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
    let analyse = || {
        factory()
            .arg("analyse")
            .arg(&job)
            .env("TAIK_LEAD_WORKER", &worker)
            .output()
            .unwrap()
    };
    let said =
        |output: &std::process::Output| String::from_utf8_lossy(&output.stdout).trim().to_owned();

    // An accepted round needs no analysis, and no worker is asked.
    fs::write(job.join("round-01.run"), "decision=accepted\nend\n").unwrap();
    answer("Verlangt: x");
    assert_eq!(said(&analyse()), "nothing to analyse");

    fs::write(
        job.join("round-02.run"),
        "decision=rejected\nchecks=AssertionError: the page has no title\nend\n",
    )
    .unwrap();
    // An answer without the four lines is refused and leaves nothing behind.
    answer("Der Titel fehlt, glaube ich.");
    assert_eq!(analyse().status.code(), Some(2));
    assert!(!job.join("round-02.analysis.md").exists());

    answer(
        "Hier die Analyse:\n- **Verlangt:** Die Seite braucht einen Titel.\nGescheitert an: Der Kandidat hat keinen.\nUrsache: Fehler im Kandidaten.\nNächster Schritt: Der Agent ergänzt den Titel.\nNoch ein Satz, den niemand braucht.",
    );
    let written = analyse();
    assert_eq!(written.status.code(), Some(0), "{written:?}");
    assert_eq!(
        fs::read_to_string(job.join("round-02.analysis.md")).unwrap(),
        "Verlangt: Die Seite braucht einen Titel.\nGescheitert an: Der Kandidat hat keinen.\nUrsache: Fehler im Kandidaten.\nNächster Schritt: Der Agent ergänzt den Titel.\n"
    );
    let ledger = fs::read_to_string(job.join("round-02.analysis.run")).unwrap();
    assert!(ledger.contains("worker_cost_usd=0.01"), "{ledger}");
    // Written once.
    answer("Verlangt: a\nGescheitert an: b\nUrsache: c\nNächster Schritt: d");
    assert_eq!(said(&analyse()), "nothing to analyse");
}
