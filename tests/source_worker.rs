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
    script(
        "codex",
        "printf 'synthetic-secret: rate_limit_exceeded\n' >&2; exit 2",
    );
    let answers = [
        (
            "takes-over",
            r#"printf '%s\n' '{"type":"stream_event","event":{"delta":{"type":"text_delta","text":"<!doctype"}}}' 'not json' '{"type":"result","is_error":false,"result":"<!doctype html><p>second</p>","usage":{"output_tokens":7},"total_cost_usd":0.25,"modelUsage":{"model-x":{}}}'"#,
            Some(0),
        ),
        (
            "also-fails",
            "printf 'synthetic-secret\n' >&2; exit 3",
            Some(1),
        ),
        (
            "reports-an-error",
            r#"printf '%s' '{"type":"result","is_error":true,"result":"<p>second</p>"}'"#,
            Some(1),
        ),
    ];
    script(
        "lead",
        r#"printf '%s' '{"type":"result","is_error":false,"result":"<!doctype html><p>second</p>","usage":{"output_tokens":3},"total_cost_usd":0.5,"modelUsage":{"model-y":{}}}'"#,
    );
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
    assert!(
        record.contains("\nworker=claude-cli\nworker_model=model-y\n"),
        "{record}"
    );
    assert!(
        !record.contains("worker_fallback") && !trace.contains("Codex"),
        "{record}{trace}"
    );

    assert!(
        record
            .lines()
            .any(|line| line.starts_with("gate_ms=") && line.len() > 8),
        "{record}"
    );

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
    assert!(
        record.contains("\nworker=claude-cli\nworker_stage=execute\n"),
        "{record}"
    );
    assert!(record.contains("\ngate_ms=\n"), "{record}");

    // The day's bound: an earlier round in the same queue already cost more than the limit.
    script(
        "lead",
        r#"printf '%s\n' '{"type":"rate_limit_event","rate_limit_info":{"rateLimitType":"seven_day","status":"allowed","utilization":0.5}}' '{"type":"result","is_error":false,"result":"<!doctype html><p>second</p>","usage":{"output_tokens":3},"total_cost_usd":0.75,"modelUsage":{"model-y":{}}}'"#,
    );
    let state = root.path().join("bounded");
    for (name, expected) in [("first", Some(0)), ("second", Some(1))] {
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
            .env("TAIK_LEAD_WORKER", bin.join("lead"))
            .env("TAIK_DAILY_BUDGET_USD", "0.5")
            .env_remove("TAIK_SECOND_WORKER")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), expected, "{name}: {result:?}");
    }
    let first = fs::read_to_string(state.join("jobs/job-first/round-01.run")).unwrap();
    assert!(first.contains("\nworker_cost_usd=0.75\n"), "{first}");
    assert!(
        first.contains("\nworker_account_usage=seven_day allowed 0.5\n"),
        "{first}"
    );
    let second = fs::read_to_string(state.join("jobs/job-second/round-01.run")).unwrap();
    assert!(
        second.contains("\nworker=claude-cli\nworker_stage=setup\nworker_failure=budget\n"),
        "{second}"
    );

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
                assert!(
                    record.lines().any(|found| found.starts_with(line)),
                    "{line}\n{record}"
                );
            }
            assert_eq!(
                fs::read_to_string(job.join("round-01.html")).unwrap(),
                "<!doctype html><p>second</p>\n"
            );
        } else {
            assert!(record.starts_with("decision=rejected\n"), "{record}");
            assert!(record.contains("\nworker_failure="), "{record}");
            assert!(record.contains("\nchecks=Claude "), "{record}");
            assert!(
                record.contains("\nworker=claude-cli\nworker_fallback=Codex "),
                "{record}"
            );
            assert!(!job.join("round-01.html").exists());
        }
    }
}

#[test]
fn savant_is_a_streamed_worker_with_its_own_cost_record() {
    let root = tempfile::Builder::new()
        .prefix("taik-savant-")
        .tempdir_in("/tmp")
        .unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a marked document\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(
        &gate,
        "import { readFileSync } from 'node:fs';\nif (!readFileSync(process.argv[2], 'utf8').includes('<p>savant</p>')) throw new Error('missing mark');\nconsole.log('PASS: marked');\n",
    )
    .unwrap();
    let stream = root.path().join("stream.txt");
    let chunk = |delta: &str| format!("data: {{\"choices\":[{{\"delta\":{delta}}}]}}\n\n");
    let complete = [
        chunk(r#"{"role":"assistant","content":""}"#),
        chunk(r#"{"reasoning":null}"#),
        chunk(r#"{"content":"<!doctype html>"}"#),
        chunk(r#"{"content":"<p>savant</p>"}"#),
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":1000,\"completion_tokens\":2000}}\n\n"
            .to_owned(),
        "data: [DONE]\n".to_owned(),
    ]
    .concat();
    let cases = [
        ("complete", complete.clone(), "test-key", Some(0)),
        (
            "cut-off",
            complete.replace("data: [DONE]\n", ""),
            "test-key",
            Some(1),
        ),
        ("no-key", complete, "", Some(1)),
    ];
    for (name, body, key, expected) in cases {
        fs::write(&stream, body).unwrap();
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
            .env("TAIK_LEAD_WORKER", "savant")
            .env("SUBQ_API_KEY", key)
            .env("TAIK_SUBQ_ENDPOINT", format!("file://{}", stream.display()))
            .env_remove("TAIK_SUBQ_KEYCHAIN")
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), expected, "{name}: {result:?}");
        let job = state.join("jobs").join(format!("job-{name}"));
        let record = fs::read_to_string(job.join("round-01.run")).unwrap();
        let trace = fs::read_to_string(job.join("round-01.progress")).unwrap();
        assert!(!record.contains("test-key") && !trace.contains("test-key"));
        assert!(record.contains("\nworker=savant\n"), "{record}");
        match name {
            "complete" => {
                for line in [
                    "decision=accepted",
                    "worker_model=subq-savant-1.4",
                    "worker_input_tokens=1000",
                    "worker_output_tokens=2000",
                    "worker_cost_usd=0.001100",
                ] {
                    assert!(
                        record.lines().any(|found| found == line),
                        "{line}\n{record}"
                    );
                }
                assert_eq!(
                    fs::read_to_string(job.join("round-01.html")).unwrap(),
                    "<!doctype html><p>savant</p>\n"
                );
            }
            "cut-off" => assert!(
                record.contains("worker_failure=invalid-response"),
                "{record}"
            ),
            _ => assert!(record.contains("worker_failure=configuration"), "{record}"),
        }
    }
}

#[test]
fn a_queued_job_builds_on_an_accepted_source_through_a_patch() {
    let root = tempfile::Builder::new()
        .prefix("taik-patch-")
        .tempdir_in("/tmp")
        .unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Change the mark\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(
        &gate,
        "import { readFileSync } from 'node:fs';\nconst text = readFileSync(process.argv[2], 'utf8');\nif (text !== '<!doctype html>\\n<p>new</p>\\n<p>kept</p>\\n') throw new Error('unexpected document');\nconsole.log('PASS: patched');\n",
    )
    .unwrap();
    let seed = root.path().join("accepted.html");
    fs::write(&seed, "<!doctype html>\n<p>old</p>\n<p>kept</p>\n").unwrap();
    let stream = root.path().join("stream.txt");
    let answer = |text: &str| {
        let chunk = serde_json::json!({"choices": [{"delta": {"content": text}}]});
        fs::write(
            &stream,
            format!("data: {chunk}\n\ndata: {{\"choices\":[],\"usage\":{{\"prompt_tokens\":10,\"completion_tokens\":10}}}}\n\ndata: [DONE]\n"),
        )
        .unwrap();
    };
    let run = |name: &str| {
        let state = root.path().join(name);
        let queued = factory()
            .arg("queue-source")
            .arg(&state)
            .arg(name)
            .arg(&brief)
            .arg(&gate)
            .arg(&seed)
            .output()
            .unwrap();
        assert!(queued.status.success(), "{queued:?}");
        let result = factory()
            .arg("drain")
            .arg(&state)
            .env("TAIK_LEAD_WORKER", "savant")
            .env("SUBQ_API_KEY", "test-key")
            .env("TAIK_SUBQ_ENDPOINT", format!("file://{}", stream.display()))
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap();
        let job = state.join("jobs").join(format!("job-{name}"));
        (result.status.code(), job)
    };

    answer("=== PATCH ===\n<<<<<<< SEARCH\n<p>old</p>\n=======\n<p>new</p>\n>>>>>>> REPLACE\n");
    let (code, job) = run("patched");
    let record = fs::read_to_string(job.join("round-01.run")).unwrap();
    assert_eq!(code, Some(0), "{record}");
    assert!(
        record.contains("\npatch_blocks=1\n") && record.starts_with("decision=accepted\n"),
        "{record}"
    );
    assert_eq!(
        fs::read_to_string(job.join("round-01.html")).unwrap(),
        "<!doctype html>\n<p>new</p>\n<p>kept</p>\n"
    );

    // The same worker answer every round: the diagnosis names the block, then the loop stops.
    answer("=== PATCH ===\n<<<<<<< SEARCH\n<p>absent</p>\n=======\n<p>new</p>\n>>>>>>> REPLACE\n");
    let (code, job) = run("unmatched");
    let record = fs::read_to_string(job.join("round-01.run")).unwrap();
    assert_eq!(code, Some(1), "{record}");
    assert!(
        record.contains("checks=patch not applicable: the SEARCH text of block 1 does not occur"),
        "{record}"
    );
    assert!(record.contains("\nadvance=true\n"), "{record}");
    assert!(!job.join("round-01.html").exists());
    assert!(job.join("round-02.run").exists() && !job.join("round-03.run").exists());
}

#[test]
fn a_worker_keeps_its_lease_by_output_and_loses_it_by_silence() {
    let root = tempfile::Builder::new()
        .prefix("taik-lease-")
        .tempdir_in("/tmp")
        .unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a marked document\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(&gate, "console.log('PASS: any');\n").unwrap();
    let worker = root.path().join("lead");
    let delta = r#"{"type":"stream_event","event":{"delta":{"type":"text_delta","text":"x"}}}"#;
    let result = r#"{"type":"result","is_error":false,"result":"<!doctype html><p>ok</p>","usage":{"output_tokens":1},"total_cost_usd":0,"modelUsage":{"m":{}}}"#;
    let cases = [
        // Output every 0.3 s for 1.8 s: longer than the lease of 1 s, shorter than the ceiling.
        (
            "working",
            format!("for i in 1 2 3 4 5 6; do echo '{delta}'; sleep 0.3; done; echo '{result}'"),
            Some(0),
            "",
        ),
        // Silent for longer than the lease.
        (
            "silent",
            format!("sleep 3; echo '{result}'"),
            Some(1),
            "stalled",
        ),
        // Output without end: the ceiling of 5 s ends it.
        (
            "endless",
            format!("while true; do echo '{delta}'; sleep 0.2; done"),
            Some(1),
            "timeout",
        ),
    ];
    for (name, body, expected, failure) in cases {
        fs::write(&worker, format!("#!/bin/sh\n{body}\n")).unwrap();
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
        let started = std::time::Instant::now();
        let outcome = factory()
            .arg("drain")
            .arg(&state)
            .env("TAIK_LEAD_WORKER", &worker)
            .env("TAIK_WORKER_LEASE_MS", "1000")
            .env("TAIK_WORKER_CEILING_MS", "5000")
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap();
        let elapsed = started.elapsed();
        let record =
            fs::read_to_string(state.join(format!("jobs/job-{name}/round-01.run"))).unwrap();
        assert_eq!(outcome.status.code(), expected, "{name}: {record}");
        if failure.is_empty() {
            assert!(record.starts_with("decision=accepted\n"), "{record}");
            assert!(
                elapsed.as_millis() >= 1700,
                "{name} ended early: {elapsed:?}"
            );
        } else {
            assert!(
                record.contains(&format!("\nworker_failure={failure}\n")),
                "{name}: {record}"
            );
            assert!(
                elapsed.as_secs() < 9,
                "{name} was not stopped in time: {elapsed:?}"
            );
        }
        // A finished round leaves the job free: a second drain neither repeats nor overtakes it.
        let again = factory().arg("drain").arg(&state).output().unwrap();
        assert_eq!(again.status.code(), Some(0), "{name}: {again:?}");
        assert!(!state.join(format!("jobs/job-{name}/round-02.run")).exists());
    }
}

#[test]
fn queued_software_jobs_run_side_by_side() {
    let root = tempfile::Builder::new()
        .prefix("taik-parallel-")
        .tempdir_in("/tmp")
        .unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a marked document\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(&gate, "console.log('PASS: any');\n").unwrap();
    let worker = root.path().join("lead");
    fs::write(
        &worker,
        "#!/bin/sh\nsleep 1.5\necho '{\"type\":\"result\",\"is_error\":false,\"result\":\"<!doctype html><p>ok</p>\",\"usage\":{\"output_tokens\":1},\"total_cost_usd\":0,\"modelUsage\":{\"m\":{}}}'\n",
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let mut durations = Vec::new();
    // The file STATE/parallel, which the viewer writes, wins over the environment.
    for (width, file, state) in [
        ("1", None, "one-at-a-time"),
        ("3", None, "three-at-a-time"),
        ("3", Some("1\n"), "set-to-one-in-the-viewer"),
    ] {
        let state = root.path().join(state);
        if let Some(slots) = file {
            fs::create_dir_all(&state).unwrap();
            fs::write(state.join("parallel"), slots).unwrap();
        }
        for name in ["a", "b", "c"] {
            let queued = factory()
                .arg("queue-source")
                .arg(&state)
                .arg(name)
                .arg(&brief)
                .arg(&gate)
                .output()
                .unwrap();
            assert!(queued.status.success(), "{queued:?}");
        }
        let started = std::time::Instant::now();
        let outcome = factory()
            .arg("drain")
            .arg(&state)
            .env("TAIK_LEAD_WORKER", &worker)
            .env("TAIK_PARALLEL", width)
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap();
        durations.push(started.elapsed());
        assert_eq!(outcome.status.code(), Some(0), "{outcome:?}");
        for name in ["a", "b", "c"] {
            let record =
                fs::read_to_string(state.join(format!("jobs/job-{name}/round-01.run"))).unwrap();
            assert!(record.starts_with("decision=accepted\n"), "{record}");
            assert!(!state.join(format!("jobs/job-{name}/round-02.run")).exists());
        }
    }
    eprintln!(
        "one at a time {:?}, three at a time {:?}",
        durations[0], durations[1]
    );
    assert!(durations[0].as_millis() >= 4500, "{durations:?}");
    assert!(durations[1].as_millis() < 3500, "{durations:?}");
    assert!(durations[2].as_millis() >= 4500, "{durations:?}");
}

#[test]
fn a_job_that_arrives_during_work_takes_a_free_slot() {
    let root = tempfile::Builder::new()
        .prefix("taik-arrival-")
        .tempdir_in("/tmp")
        .unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a marked document\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(&gate, "console.log('PASS: any');\n").unwrap();
    let worker = root.path().join("lead");
    fs::write(
        &worker,
        "#!/bin/sh\nsleep 1.5\necho '{\"type\":\"result\",\"is_error\":false,\"result\":\"<!doctype html><p>ok</p>\",\"usage\":{\"output_tokens\":1},\"total_cost_usd\":0,\"modelUsage\":{\"m\":{}}}'\n",
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let state = root.path().join("state");
    let queue = |name: &str| {
        let queued = factory()
            .arg("queue-source")
            .arg(&state)
            .arg(name)
            .arg(&brief)
            .arg(&gate)
            .output()
            .unwrap();
        assert!(queued.status.success(), "{queued:?}");
    };
    queue("first");
    let started = std::time::Instant::now();
    let mut drain = factory()
        .arg("drain")
        .arg(&state)
        .env("TAIK_LEAD_WORKER", &worker)
        .env("TAIK_PARALLEL", "3")
        .env_remove("TAIK_SECOND_WORKER")
        .env_remove("TAIK_AUTO_PROPOSE")
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));
    queue("late");
    assert!(drain.wait().unwrap().success());
    let elapsed = started.elapsed();
    for name in ["first", "late"] {
        let record =
            fs::read_to_string(state.join(format!("jobs/job-{name}/round-01.run"))).unwrap();
        assert!(record.starts_with("decision=accepted\n"), "{record}");
    }
    // One after the other would take at least 3 s. Side by side the late job ends about 0.4 s later.
    assert!(
        elapsed.as_millis() < 2900,
        "the late job waited: {elapsed:?}"
    );
}

#[test]
fn a_gate_can_name_a_release_step_for_its_accepted_artifact() {
    let root = tempfile::Builder::new()
        .prefix("taik-release-")
        .tempdir_in("/tmp")
        .unwrap();
    // The checkout is the working directory. The sandbox hides it from worker and gate.
    let checkout = root.path().join("checkout");
    fs::create_dir_all(checkout.join("tools")).unwrap();
    let script = checkout.join("tools/mark.sh");
    fs::write(&script, "#!/bin/sh\necho released > \"$1/released.txt\"\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a marked document\n").unwrap();
    let worker = root.path().join("lead");
    fs::write(
        &worker,
        "#!/bin/sh\necho '{\"type\":\"result\",\"is_error\":false,\"result\":\"<!doctype html><p>ok</p>\",\"usage\":{\"output_tokens\":1},\"total_cost_usd\":0,\"modelUsage\":{\"m\":{}}}'\n",
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let cases = [
        (
            "released",
            "// TAIK: release tools/mark.sh\nconsole.log('PASS: any');\n",
            true,
        ),
        (
            "rejected",
            "// TAIK: release tools/mark.sh\nthrow new Error('no');\n",
            false,
        ),
        (
            "outside",
            "// TAIK: release ../tools/mark.sh\nconsole.log('PASS: any');\n",
            false,
        ),
        ("unnamed", "console.log('PASS: any');\n", false),
    ];
    for (name, gate_text, released) in cases {
        let gate = root.path().join(format!("{name}.mjs"));
        fs::write(&gate, gate_text).unwrap();
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
        factory()
            .arg("drain")
            .arg(&state)
            .current_dir(&checkout)
            .env("TAIK_LEAD_WORKER", &worker)
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap();
        let job = state.join(format!("jobs/job-{name}"));
        assert_eq!(job.join("released.txt").exists(), released, "{name}");
        // A second pass over the finished job does not release again.
        if released {
            fs::remove_file(job.join("released.txt")).unwrap();
            factory()
                .arg("drain")
                .arg(&state)
                .current_dir(&checkout)
                .output()
                .unwrap();
            assert!(
                !job.join("released.txt").exists(),
                "{name} was released twice"
            );
        }
    }
}

#[test]
fn a_job_is_worked_by_the_agent_it_names() {
    let root = tempfile::Builder::new()
        .prefix("taik-crew-")
        .tempdir_in("/tmp")
        .unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let script = |name: &str, body: &str| {
        let file = bin.join(name);
        fs::write(&file, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
    };
    // Each stand-in checks that it was asked for the configured model and effort, without tools,
    // and that the persona stands before the Factory's own sentence.
    script(
        "grok",
        r#"case "$*" in *"-m grok-x"*"--reasoning-effort low"*) ;; *) echo wrong arguments >&2; exit 3 ;; esac
case "$*" in *"--tools  --no-subagents"*) ;; *) echo tools not switched off >&2; exit 4 ;; esac
head -1 prompt.txt | grep -q '^Du malst sparsam\.$' || exit 5
grep -q 'You are the code worker embedded in TAIK' prompt.txt || exit 6
printf '%s\n' '{"type":"text","data":"<!doctype html>"}' '{"type":"text","data":"<p>grok</p>"}' '{"type":"end","stopReason":"end_turn","usage":{"input_tokens":11,"output_tokens":7},"total_cost_usd":0.5,"modelUsage":{"grok-reported":{}}}'"#,
    );
    script(
        "pi",
        r#"case "$*" in *"--no-tools"*"--model xai/grok-y --thinking high"*) ;; *) echo wrong arguments >&2; exit 3 ;; esac
printf '%s\n' '{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"<!doctype html><p>pi</p>"}}' '{"type":"message_end","message":{"role":"assistant","provider":"xai","model":"grok-y-reported","stopReason":"stop","usage":{"input":5,"output":9,"cost":{"total":0.125}}}}'"#,
    );
    let mut path = vec![bin.clone()];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(path).unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a marked document\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(&gate, "console.log('PASS: any');\n").unwrap();
    let state = root.path().join("state");
    fs::create_dir_all(state.join("crew")).unwrap();
    // As in the real setup, the job folder is a link into another directory. Agents and settings
    // still come from the state directory.
    fs::create_dir(root.path().join("records")).unwrap();
    std::os::unix::fs::symlink(root.path().join("records"), state.join("jobs")).unwrap();
    fs::write(
        state.join("crew/maler.agent"),
        "harness=grok\nmodel=grok-x\neffort=low\npersona=Du malst sparsam.\nend\n",
    )
    .unwrap();
    fs::write(
        state.join("crew/pruefer.agent"),
        "harness=pi\nmodel=xai/grok-y\neffort=high\nend\n",
    )
    .unwrap();
    let run = |name: &str, agent: &str| {
        let queued = factory()
            .arg("queue-source")
            .arg(&state)
            .arg(name)
            .arg(&brief)
            .arg(&gate)
            .arg(format!("--agent={agent}"))
            .output()
            .unwrap();
        assert!(queued.status.success(), "{queued:?}");
        let job = state.join(format!("jobs/job-{name}"));
        assert_eq!(
            fs::read_to_string(job.join("agent")).unwrap(),
            format!("{agent}\n")
        );
        let drained = factory()
            .arg("drain")
            .arg(&state)
            .env("PATH", &path)
            // The environment names another worker. The agent of the job wins.
            .env("TAIK_LEAD_WORKER", bin.join("missing-claude"))
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap();
        (drained, job)
    };

    let (drained, job) = run("bild", "maler");
    let record = fs::read_to_string(job.join("round-01.run")).unwrap();
    assert_eq!(drained.status.code(), Some(0), "{record}");
    for line in [
        "decision=accepted",
        "worker=grok-cli",
        "agent=maler",
        "worker_model=grok-reported",
        "worker_input_tokens=11",
        "worker_output_tokens=7",
        "worker_cost_usd=0.500000",
    ] {
        assert!(
            record.lines().any(|found| found == line),
            "{line}\n{record}"
        );
    }
    assert_eq!(
        fs::read_to_string(job.join("round-01.html")).unwrap(),
        "<!doctype html><p>grok</p>\n"
    );

    let (drained, job) = run("pruefung", "pruefer");
    let record = fs::read_to_string(job.join("round-01.run")).unwrap();
    assert_eq!(drained.status.code(), Some(0), "{record}");
    for line in [
        "decision=accepted",
        "worker=pi-cli",
        "agent=pruefer",
        "worker_model=xai/grok-y-reported",
        "worker_cost_usd=0.125000",
    ] {
        assert!(
            record.lines().any(|found| found == line),
            "{line}\n{record}"
        );
    }

    // An agent that is not defined stops the job. Nothing runs on another account.
    let (drained, job) = run("verwaist", "niemand");
    assert_eq!(drained.status.code(), Some(1), "{drained:?}");
    assert!(!job.join("round-01.run").exists());
    assert!(String::from_utf8_lossy(&drained.stderr).contains("agent niemand is not defined"));

    // The overview lists the defined agents and never a credential.
    let overview = factory()
        .arg("crew")
        .arg(&state)
        .env("PATH", &path)
        .output()
        .unwrap();
    assert!(overview.status.success(), "{overview:?}");
    let listed: serde_json::Value = serde_json::from_slice(&overview.stdout).unwrap();
    let names: Vec<&str> = listed["agents"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|agent| agent["name"].as_str())
        .collect();
    assert_eq!(names, ["maler", "pruefer"]);
    assert!(
        listed["harnesses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|harness| harness["name"] == "grok")
    );
    assert!(state.join("crew/overview.json").is_file());
}

#[test]
fn a_job_holds_its_conversation_and_a_reply_reopens_it() {
    let root = tempfile::Builder::new()
        .prefix("taik-talk-")
        .tempdir_in("/tmp")
        .unwrap();
    let brief = root.path().join("brief.md");
    fs::write(&brief, "Deliver a coloured paragraph\n").unwrap();
    let gate = root.path().join("verify.mjs");
    fs::write(&gate, "console.log('PASS: any');\n").unwrap();
    // The stand-in worker asks first, delivers once the user has answered, and follows a later reply.
    let worker = root.path().join("lead");
    let result = |text: &str| {
        format!(
            r#"printf '%s\n' '{{"type":"result","is_error":false,"result":"{text}","usage":{{"output_tokens":1}},"total_cost_usd":0.125,"modelUsage":{{"m":{{}}}}}}'"#
        )
    };
    fs::write(
        &worker,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *'[user] Jetzt rot.'*'Follow the last message'*) {} ;;\n  *'[you] Welche Farbe?'*'[user] Blau.'*) {} ;;\n  *'=== FRAGE ==='*) {} ;;\n  *) exit 9 ;;\nesac\n",
            result("<!doctype html><p>rot</p>"),
            result("<!doctype html><p>blau</p>"),
            result(r"=== FRAGE ===\nWelche Farbe?"),
        ),
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let state = root.path().join("state");
    let queued = factory()
        .arg("queue-source")
        .arg(&state)
        .arg("farbe")
        .arg(&brief)
        .arg(&gate)
        .output()
        .unwrap();
    assert!(queued.status.success(), "{queued:?}");
    let job = state.join("jobs/job-farbe");
    let drain = || {
        factory()
            .arg("drain")
            .arg(&state)
            .env("TAIK_LEAD_WORKER", &worker)
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap()
    };
    let reply = |text: &str| {
        use std::io::Write;
        let mut child = factory()
            .arg("reply")
            .arg(&state)
            .arg("job-farbe")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success());
    };

    // The worker asks. Nothing is decided, the question and its cost are in the job folder.
    assert!(drain().status.success());
    assert!(!job.join("round-01.run").exists());
    assert_eq!(
        fs::read_to_string(job.join("message-001.agent.md")).unwrap(),
        "round=0\n\nWelche Farbe?\n"
    );
    let ledger = fs::read_to_string(job.join("message-001.run")).unwrap();
    assert!(
        ledger.contains("\nworker_cost_usd=0.125\n") && ledger.contains("kind=message"),
        "{ledger}"
    );
    // Without an answer the job waits. The worker is not asked again.
    assert!(drain().status.success());
    assert!(!job.join("round-01.run").exists() && !job.join("message-002.agent.md").exists());

    // The answer goes into the job. The next round reads the conversation.
    reply("Blau.\n");
    assert_eq!(
        fs::read_to_string(job.join("message-002.user.md")).unwrap(),
        "round=0\n\nBlau.\n"
    );
    assert!(drain().status.success());
    let record = fs::read_to_string(job.join("round-01.run")).unwrap();
    assert!(record.starts_with("decision=accepted\n"), "{record}");
    assert_eq!(
        fs::read_to_string(job.join("round-01.html")).unwrap(),
        "<!doctype html><p>blau</p>\n"
    );

    // The job is finished. Another pass does nothing, a reply grants exactly one further round.
    assert!(drain().status.success());
    assert!(!job.join("round-02.run").exists());
    reply("Jetzt rot.");
    assert!(drain().status.success());
    assert_eq!(
        fs::read_to_string(job.join("round-02.html")).unwrap(),
        "<!doctype html><p>rot</p>\n"
    );
    assert!(drain().status.success());
    assert!(!job.join("round-03.run").exists());

    // A reply names a queued job and nothing else.
    for name in ["job-unbekannt", "../job-farbe", "farbe"] {
        let refused = factory()
            .arg("reply")
            .arg(&state)
            .arg(name)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(refused.status.code(), Some(2), "{name}");
    }
}
