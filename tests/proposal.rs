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
    // A long request is kept whole.
    let long = format!("Langer Wunsch\n\n{} Ende.", "Satz mit Inhalt. ".repeat(120));
    assert_eq!(say(&["request", home], &long).status.code(), Some(0));
    let kept = fs::read_to_string(state.join("requests/ticket-002.run")).unwrap();
    assert!(
        kept.contains(" Ende.\ncontinuation="),
        "the request was cut"
    );
    fs::remove_file(state.join("requests/ticket-002.run")).unwrap();

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

#[test]
fn the_orchestrator_plans_a_request_and_approval_follows_the_plan() {
    use std::io::Write;
    use std::process::Stdio;
    let root = tempfile::Builder::new()
        .prefix("taik-plan-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = root.path().canonicalize().unwrap();
    let factory = || Command::new(env!("CARGO_BIN_EXE_taik-seed"));
    let state = root.join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    fs::create_dir_all(state.join("repos")).unwrap();
    fs::create_dir_all(state.join("crew")).unwrap();
    // A repository the user made known, and a second agent beside the standard worker.
    let repo = root.join("demo");
    fs::create_dir_all(&repo).unwrap();
    for arguments in [
        vec!["init", "--quiet"],
        vec![
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "start",
        ],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(arguments)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(
        state.join("repos/demo.repo"),
        format!("{}\n", repo.display()),
    )
    .unwrap();
    let worker = root.join("lead");
    fs::write(
        state.join("crew/pruefer.agent"),
        "harness=claude\nmodel=m\neffort=low\npersona=Schreibt Prüfungen.\nend\n",
    )
    .unwrap();
    // One stand-in for both roles: it answers the orchestrator with a plan, everyone else with files.
    let reply = |text: &str| {
        serde_json::json!({"type": "result", "is_error": false, "result": text,
            "usage": {"output_tokens": 1}, "total_cost_usd": 0.01, "modelUsage": {"m": {}}})
        .to_string()
    };
    fs::write(
        root.join("plan.json"),
        reply("Art: sitzung\nRepo: demo\nGate: pruefer\nArbeit: standard\nGrund: Es müssen bestehende Dateien geändert werden."),
    )
    .unwrap();
    fs::write(
        root.join("files.json"),
        reply("=== brief.md ===\nEine Datei hello.txt anlegen\n\n=== gate.sh ===\ntest -f hello.txt && echo 'PASS: da'\n"),
    )
    .unwrap();
    fs::write(
        &worker,
        format!(
            "#!/bin/sh\ncase \"$*\" in *'orchestrator of a software factory'*) cat '{0}/plan.json';; *) cat '{0}/files.json';; esac\n",
            root.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    // The named agent runs on the Claude harness, which is this stand-in under the name `claude`.
    fs::copy(&worker, root.join("claude")).unwrap();
    let home = state.to_str().unwrap();
    let mut asked = factory()
        .args(["request", home])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    asked
        .stdin
        .take()
        .unwrap()
        .write_all(b"Lege hello.txt im Repo demo an")
        .unwrap();
    assert!(asked.wait().unwrap().success());
    let served = factory()
        .arg("drain")
        .arg(&state)
        .env("TAIK_LEAD_WORKER", &worker)
        .env(
            "PATH",
            format!("{}:{}", root.display(), std::env::var("PATH").unwrap()),
        )
        .env_remove("TAIK_SECOND_WORKER")
        .env_remove("TAIK_AUTO_PROPOSE")
        .output()
        .unwrap();
    let plan = fs::read_to_string(state.join("requests/ticket-001.plan"))
        .unwrap_or_else(|_| panic!("{served:?}"));
    assert!(
        plan.contains("kind=sitzung\n")
            && plan.contains("repo=demo\n")
            && plan.contains("gate=pruefer\n")
            && plan.contains("work=standard\n"),
        "{plan}"
    );
    let proposal = state.join("proposals/ticket-001");
    assert!(proposal.join("gate.sh").is_file(), "{served:?}");
    assert_eq!(fs::read_to_string(proposal.join("plan.txt")).unwrap(), plan);
    // The gate was written by the agent the plan names.
    let origin = fs::read_to_string(proposal.join("origin.txt")).unwrap();
    assert!(origin.contains("proposed_by=pruefer\n"), "{origin}");

    let approved = factory()
        .args(["approve", home, "ticket-001"])
        .output()
        .unwrap();
    assert_eq!(approved.status.code(), Some(0), "{approved:?}");
    let job = state.join("jobs/job-ticket-001");
    assert!(job.join("session.brief.md").is_file() && job.join("session.gate.sh").is_file());
    assert_eq!(
        fs::read_to_string(job.join("session.repo")).unwrap().trim(),
        repo.to_str().unwrap()
    );
    assert!(
        !job.join("agent").exists(),
        "the standard worker needs no agent file"
    );
}

#[test]
fn a_planned_product_change_gets_a_check_that_is_red_on_the_current_source() {
    use std::io::Write;
    use std::process::Stdio;
    let root = tempfile::Builder::new()
        .prefix("taik-check-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = root.path().canonicalize().unwrap();
    let factory = || Command::new(env!("CARGO_BIN_EXE_taik-seed"));
    let state = root.join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    fs::create_dir_all(state.join("products")).unwrap();
    let page = root.join("page.html");
    fs::write(&page, "<!doctype html><p>alt</p>\n").unwrap();
    let gate = root.join("product.mjs");
    fs::write(&gate, "console.log('PASS: the product as it was');\n").unwrap();
    fs::write(
        state.join("products/seite.product"),
        format!(
            "about=Eine Seite\nseed={}\ngate={}\nend\n",
            page.display(),
            gate.display()
        ),
    )
    .unwrap();
    let reply = |text: &str| {
        serde_json::json!({"type": "result", "is_error": false, "result": text,
            "usage": {"output_tokens": 1}, "total_cost_usd": 0.01, "modelUsage": {"m": {}}})
        .to_string()
    };
    fs::write(root.join("plan.json"), reply("Art: dokument\nRepo: -\nGate: standard\nArbeit: standard\nGrund: Ein Produkt wird geändert.")).unwrap();
    fs::write(
        root.join("product.json"),
        reply("=== product seite ===\nDie Seite bekommt eine Überschrift\n\nMehr.\n"),
    )
    .unwrap();
    let worker = root.join("lead");
    fs::write(
        &worker,
        format!(
            "#!/bin/sh\ncase \"$*\" in *'orchestrator of a software factory'*) cat '{0}/plan.json';; *'CURRENT SOURCE OF THE PRODUCT'*) cat '{0}/check.json';; *) cat '{0}/product.json';; esac\n",
            root.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let home = state.to_str().unwrap();
    let wish = |check: &str| {
        fs::write(root.join("check.json"), reply(check)).unwrap();
        let mut asked = factory()
            .args(["request", home])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        asked
            .stdin
            .take()
            .unwrap()
            .write_all(b"Eine Ueberschrift fuer die Seite")
            .unwrap();
        let name = String::from_utf8_lossy(&asked.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned();
        let served = factory()
            .arg("drain")
            .arg(&state)
            .env("TAIK_LEAD_WORKER", &worker)
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap();
        let proposal = state.join("proposals").join(&name);
        let gate = fs::read_to_string(proposal.join("verify.mjs"))
            .unwrap_or_else(|_| panic!("{served:?}"));
        (gate, fs::read_to_string(proposal.join("brief.md")).unwrap())
    };

    // A check that fails on the current source is appended, and the revised brief replaces the first.
    let (gate, brief) = wish(
        "=== brief.md ===\nDie Seite bekommt eine Überschrift\n\nSie steht in einem h1-Element.\n=== check.mjs ===\nassert(html.includes('<h1>'), 'die Überschrift in einem h1-Element fehlt');\n",
    );
    assert!(
        gate.starts_with("console.log('PASS: the product as it was');\n"),
        "{gate}"
    );
    assert!(
        gate.contains("// TAIK: check added by job ticket-001\n")
            && gate.contains("die Überschrift in einem h1-Element fehlt"),
        "{gate}"
    );
    assert!(brief.contains("Sie steht in einem h1-Element."), "{brief}");

    // A check that already passes proves nothing and is left out. So is one that is no JavaScript,
    // one with an import statement and a revised brief with another intent.
    for useless in [
        "=== brief.md ===\nDie Seite bekommt eine Überschrift\n=== check.mjs ===\nassert(html.includes('<p>'), 'x');\n",
        "=== brief.md ===\nDie Seite bekommt eine Überschrift\n=== check.mjs ===\nassert(((\n",
        "=== brief.md ===\nDie Seite bekommt eine Überschrift\n=== check.mjs ===\nimport fs from 'node:fs';\nassert(false, 'x');\n",
        "=== brief.md ===\nEtwas ganz anderes\n=== check.mjs ===\nassert(false, 'x');\n",
        "NONE",
    ] {
        let (gate, brief) = wish(useless);
        assert_eq!(
            gate, "console.log('PASS: the product as it was');\n",
            "{useless}"
        );
        assert_eq!(
            brief, "Die Seite bekommt eine Überschrift\n\nMehr.\n",
            "{useless}"
        );
    }
}

#[test]
fn a_request_is_derived_while_a_job_runs() {
    use std::io::Write;
    use std::process::Stdio;
    let root = tempfile::Builder::new()
        .prefix("taik-during-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = root.path().canonicalize().unwrap();
    let factory = || Command::new(env!("CARGO_BIN_EXE_taik-seed"));
    let state = root.join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();
    let reply = |text: &str| {
        serde_json::json!({"type": "result", "is_error": false, "result": text,
            "usage": {"output_tokens": 1}, "total_cost_usd": 0.0, "modelUsage": {"m": {}}})
        .to_string()
    };
    fs::write(
        root.join("plan.json"),
        reply("Art: dokument\nRepo: -\nGate: standard\nArbeit: standard\nGrund: Neu."),
    )
    .unwrap();
    fs::write(
        root.join("files.json"),
        reply("=== brief.md ===\nEine Seite\n\n=== verify.mjs ===\nconsole.log('PASS');\n"),
    )
    .unwrap();
    fs::write(root.join("page.json"), reply("<!doctype html><p>ok</p>")).unwrap();
    // The worker of the job takes four seconds. Planning and deriving answer at once.
    let worker = root.join("lead");
    fs::write(
        &worker,
        format!(
            "#!/bin/sh\ncase \"$*\" in *'orchestrator of a software factory'*) cat '{0}/plan.json';; *'You derive the next job'*) cat '{0}/files.json';; *) sleep 4; cat '{0}/page.json';; esac\n",
            root.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let brief = root.join("brief.md");
    fs::write(&brief, "Eine Seite liefern\n").unwrap();
    let gate = root.join("verify.mjs");
    fs::write(&gate, "console.log('PASS: any');\n").unwrap();
    let home = state.to_str().unwrap();
    assert!(
        factory()
            .args(["queue-source", home, "lang"])
            .arg(&brief)
            .arg(&gate)
            .status()
            .unwrap()
            .success()
    );
    let service = factory()
        .arg("drain")
        .arg(&state)
        .env("TAIK_LEAD_WORKER", &worker)
        .env_remove("TAIK_SECOND_WORKER")
        .env_remove("TAIK_AUTO_PROPOSE")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // The wish arrives while the job is at work.
    std::thread::sleep(std::time::Duration::from_millis(1200));
    let mut asked = factory()
        .args(["request", home])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    asked
        .stdin
        .take()
        .unwrap()
        .write_all(b"Noch eine Seite")
        .unwrap();
    assert!(asked.wait().unwrap().success());
    assert!(
        !state.join("jobs/job-lang/round-01.run").exists(),
        "the job must still be running when the wish arrives"
    );
    let waited = std::time::Instant::now();
    while !state.join("proposals/ticket-001/brief.md").exists() {
        assert!(
            !state.join("jobs/job-lang/round-01.run").exists(),
            "the proposal came only after the job had ended"
        );
        assert!(
            waited.elapsed().as_secs() < 3,
            "the wish was not derived during the job"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let mut service = service;
    assert_eq!(service.wait().unwrap().code(), Some(0));
    assert!(state.join("jobs/job-lang/round-01.run").exists());
}
