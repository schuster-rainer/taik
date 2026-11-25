#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn factory() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
}

#[test]
fn worker_failure_records_distinguish_phases_without_persisting_stderr() {
    let root = tempfile::Builder::new()
        .prefix("taik-worker-observation-")
        .tempdir_in("/tmp")
        .unwrap();
    let bin = root.path().join("bin");
    let auth = root.path().join("auth");
    fs::create_dir(&bin).unwrap();
    fs::create_dir(&auth).unwrap();
    fs::write(auth.join("auth.json"), "{}").unwrap();
    let mut path = vec![bin.clone()];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(path).unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Check the worker boundary\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(&gate, "throw new Error('unexpected gate invocation');\n").unwrap();
    let worker = bin.join("codex");

    let cases = [
        (
            "login-exit",
            "printf 'synthetic-secret: Connection refused\n' >&2; exit 2",
            "exit 3",
            ("login", "exit", "2", "unreachable"),
        ),
        (
            "execution-limit",
            "printf 'Logged in using ChatGPT\n'; exit 0",
            "printf 'synthetic-secret: rate_limit_exceeded\n' >&2; exit 2",
            ("execute", "exit", "2", "rate-limit"),
        ),
        (
            "execution-unreachable",
            "printf 'Logged in using ChatGPT\n'; exit 0",
            "printf 'synthetic-secret: Connection refused\n' >&2; exit 2",
            ("execute", "exit", "2", "unreachable"),
        ),
        (
            "missing-response",
            "printf 'Logged in using ChatGPT\n'; exit 0",
            "exit 0",
            ("output", "missing-response", "", ""),
        ),
        (
            "login-timeout",
            "sleep 20",
            "exit 3",
            ("login", "timeout", "", ""),
        ),
    ];
    for (name, login, execute, (expected_stage, expected_failure, expected_exit, expected_hint)) in
        cases
    {
        fs::write(
            &worker,
            format!("#!/bin/sh\nif [ \"$1\" = login ]; then\n  {login}\nfi\n{execute}\n"),
        )
        .unwrap();
        fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
        let state = root.path().join(name);
        let queued = factory()
            .arg("queue-source")
            .arg(&state)
            .arg(name)
            .arg(&brief)
            .arg(&gate)
            .output()
            .unwrap();
        assert!(queued.status.success(), "{queued:?}");
        let result = factory()
            .arg("drain")
            .arg(&state)
            .env("PATH", &path)
            .env("CODEX_HOME", &auth)
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_LEAD_WORKER")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1), "{name}: {result:?}");
        let job = state.join("jobs").join(format!("job-{name}"));
        let record = fs::read_to_string(job.join("round-01.run")).unwrap();
        let trace = fs::read_to_string(job.join("round-01.progress")).unwrap();
        let field = |key| {
            record
                .lines()
                .filter_map(|line| line.split_once('='))
                .find_map(|(name, value)| (name == key).then_some(value))
                .unwrap()
        };
        assert_eq!(field("worker_stage"), expected_stage);
        assert_eq!(field("worker_failure"), expected_failure);
        assert_eq!(field("worker_exit_code"), expected_exit);
        assert_eq!(field("worker_provider_hint"), expected_hint);
        let elapsed = field("worker_elapsed_ms").parse::<u128>().unwrap();
        if name == "login-timeout" {
            assert!(elapsed >= 4_900, "{record}");
        }
        assert_eq!(field("decision"), "rejected");
        assert_eq!(field("advance"), "false");
        assert_eq!(field("artifact"), "");
        assert_eq!(field("follow_up"), "");
        assert!(!job.join("round-01.html").exists());
        assert!(!job.join("round-02.run").exists());
        assert!(!record.contains("synthetic-secret"));
        assert!(!trace.contains("synthetic-secret"));
        assert!(!trace.contains("\tevaluate\t"));
        eprintln!(
            "{name}: stage={expected_stage} failure={expected_failure} exit={expected_exit} hint={expected_hint} elapsed={elapsed}ms"
        );
    }
}

#[test]
fn a_configured_second_worker_takes_over_and_is_named_in_the_record() {
    let root = tempfile::Builder::new()
        .prefix("taik-second-worker-")
        .tempdir_in("/tmp")
        .unwrap();
    let bin = root.path().join("bin");
    let auth = root.path().join("auth");
    fs::create_dir(&bin).unwrap();
    fs::create_dir(&auth).unwrap();
    fs::write(auth.join("auth.json"), "{}").unwrap();
    let mut path = vec![bin.clone()];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(path).unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a marked document\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(
        &gate,
        "import { readFileSync } from 'node:fs';\nif (!readFileSync(process.argv[2], 'utf8').includes('<p>second</p>')) throw new Error('missing mark');\nconsole.log('PASS: marked');\n",
    )
    .unwrap();
    let script = |name: &str, body: &str| {
        let file = bin.join(name);
        fs::write(&file, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
        file
    };
    script("codex", "printf 'synthetic-secret: rate_limit_exceeded\n' >&2; exit 2");
    let answers = [
        (
            "takes-over",
            r#"printf '%s\n' '{"type":"stream_event","event":{"delta":{"type":"text_delta","text":"<!doctype"}}}' 'not json' '{"type":"result","is_error":false,"result":"<!doctype html><p>second</p>","usage":{"output_tokens":7},"total_cost_usd":0.25,"modelUsage":{"model-x":{}}}'"#,
            Some(0),
        ),
        ("also-fails", "printf 'synthetic-secret\n' >&2; exit 3", Some(1)),
        ("reports-an-error", r#"printf '%s' '{"type":"result","is_error":true,"result":"<p>second</p>"}'"#, Some(1)),
    ];
    script("lead", r#"printf '%s' '{"type":"result","is_error":false,"result":"<!doctype html><p>second</p>","usage":{"output_tokens":3},"total_cost_usd":0.5,"modelUsage":{"model-y":{}}}'"#);
    let state = root.path().join("lead");
    let queued = factory()
        .arg("queue-source")
        .arg(&state)
        .arg("lead")
        .arg(&brief)
        .arg(&gate)
        .output()
        .unwrap();
    assert!(queued.status.success(), "{queued:?}");
    let result = factory()
        .arg("drain")
        .arg(&state)
        .env("PATH", &path)
        .env("CODEX_HOME", &auth)
        .env("TAIK_LEAD_WORKER", bin.join("lead"))
        .env_remove("TAIK_SECOND_WORKER")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0), "{result:?}");
    let job = state.join("jobs/job-lead");
    let record = fs::read_to_string(job.join("round-01.run")).unwrap();
    let trace = fs::read_to_string(job.join("round-01.progress")).unwrap();
    assert!(record.contains("\nworker=claude-cli\nworker_model=model-y\n"), "{record}");
    assert!(!record.contains("worker_fallback") && !trace.contains("Codex"), "{record}{trace}");

    assert!(record.lines().any(|line| line.starts_with("gate_ms=") && line.len() > 8), "{record}");

    script("lead", "exit 4");
    let state = root.path().join("lead-fails");
    let queued = factory()
        .arg("queue-source")
        .arg(&state)
        .arg("lead-fails")
        .arg(&brief)
        .arg(&gate)
        .output()
        .unwrap();
    assert!(queued.status.success(), "{queued:?}");
    let result = factory()
        .arg("drain")
        .arg(&state)
        .env("PATH", &path)
        .env("CODEX_HOME", &auth)
        .env("TAIK_LEAD_WORKER", bin.join("lead"))
        .env_remove("TAIK_SECOND_WORKER")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    let record = fs::read_to_string(state.join("jobs/job-lead-fails/round-01.run")).unwrap();
    assert!(record.contains("\nworker=claude-cli\nworker_stage=execute\n"), "{record}");
    assert!(record.contains("\ngate_ms=\n"), "{record}");

    for (name, answer, expected) in answers {
        let second = script("second", answer);
        let state = root.path().join(name);
        let queued = factory()
            .arg("queue-source")
            .arg(&state)
            .arg(name)
            .arg(&brief)
            .arg(&gate)
            .output()
            .unwrap();
        assert!(queued.status.success(), "{queued:?}");
        let result = factory()
            .arg("drain")
            .arg(&state)
            .env("PATH", &path)
            .env("CODEX_HOME", &auth)
            .env("TAIK_SECOND_WORKER", &second)
            .env_remove("TAIK_LEAD_WORKER")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), expected, "{name}: {result:?}");
        let job = state.join("jobs").join(format!("job-{name}"));
        let record = fs::read_to_string(job.join("round-01.run")).unwrap();
        let trace = fs::read_to_string(job.join("round-01.progress")).unwrap();
        assert!(!record.contains("synthetic-secret") && !trace.contains("synthetic-secret"));
        if name == "takes-over" {
            for line in [
                "decision=accepted",
                "worker=claude-cli",
                "worker_fallback=Codex login exit (exit status: 2) [rate-limit] after",
                "worker_model=model-x",
                "worker_output_tokens=7",
                "worker_cost_usd=0.25",
                "artifact=round-01.html",
            ] {
                assert!(record.lines().any(|found| found.starts_with(line)), "{line}\n{record}");
            }
            assert_eq!(
                fs::read_to_string(job.join("round-01.html")).unwrap(),
                "<!doctype html><p>second</p>\n"
            );
        } else {
            assert!(record.starts_with("decision=rejected\n"), "{record}");
            assert!(record.contains("\nworker_failure="), "{record}");
            assert!(record.contains("\nchecks=Claude "), "{record}");
            assert!(record.contains("\nworker=claude-cli\nworker_fallback=Codex "), "{record}");
            assert!(!job.join("round-01.html").exists());
        }
    }
}
