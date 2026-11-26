#![cfg(target_os = "macos")]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

fn git(folder: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn an_agent_session_works_in_its_copy_and_is_judged_by_commit() {
    let root = tempfile::Builder::new()
        .prefix("taik-session-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = root.path().canonicalize().unwrap();
    let factory = || Command::new(env!("CARGO_BIN_EXE_taik-seed"));
    let state = root.join("state");
    fs::create_dir_all(state.join("jobs")).unwrap();

    // A repository with one protected folder.
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("locked")).unwrap();
    fs::create_dir_all(repo.join(".taik")).unwrap();
    git(&repo, &["init", "--quiet"]);
    fs::write(repo.join("README"), "start\n").unwrap();
    fs::write(repo.join("locked/keep.txt"), "do not touch\n").unwrap();
    fs::write(
        repo.join(".taik/protected"),
        "# kept by the gate\nlocked/\n",
    )
    .unwrap();
    git(&repo, &["add", "--all"]);
    git(&repo, &["commit", "--quiet", "-m", "start"]);

    // The harness stand-in does what the file `mode` says and reports like the real stream.
    let mode = root.join("mode");
    let agent = root.join("agent");
    fs::write(
        &agent,
        format!(
            r#"#!/bin/sh
mode=$(cat '{mode}')
resumed=no
case "$*" in *--resume*) resumed=yes;; esac
say() {{ printf '%s\n' "$1"; }}
case "$mode" in
  write) echo hi > hello.txt
    say '{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Ich lege die Datei an."}},{{"type":"tool_use","name":"Write","input":{{"file_path":"'"$PWD"'/hello.txt"}}}}]}}}}'
    text="Datei hello.txt angelegt. resumed=$resumed";;
  protected) echo changed > locked/keep.txt
    say '{{"type":"assistant","message":{{"content":[{{"type":"tool_use","name":"Edit","input":{{"file_path":"'"$PWD"'/locked/keep.txt"}}}}]}}}}'
    text="Geschützte Datei geändert. resumed=$resumed";;
  ask) text="=== FRAGE ===\\nWelche Farbe?";;
  outside) if echo x > "$HOME/.taik-session-test" 2>/dev/null; then wrote=yes; rm -f "$HOME/.taik-session-test"; else wrote=no; fi
    if cat "$HOME/.zshrc" >/dev/null 2>&1 || ls '{jobs}' >/dev/null 2>&1; then read=yes; else read=no; fi
    echo ok > hello.txt
    text="wrote=$wrote read=$read";;
esac
say '{{"type":"result","is_error":false,"result":"'"$text"'","usage":{{"output_tokens":3,"cache_read_input_tokens":7}},"total_cost_usd":0.02,"modelUsage":{{"m":{{}}}}}}'
"#,
            mode = mode.display(),
            jobs = state.join("jobs").display(),
        ),
    )
    .unwrap();
    fs::set_permissions(&agent, fs::Permissions::from_mode(0o700)).unwrap();
    let lead = root.join("lead");
    fs::write(
        &lead,
        "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"result\",\"is_error\":false,\"result\":\"Verlangt: a\\nGescheitert an: b\\nUrsache: c\\nNächster Schritt: d\",\"usage\":{\"output_tokens\":1},\"total_cost_usd\":0,\"modelUsage\":{\"m\":{}}}'\n",
    )
    .unwrap();
    fs::set_permissions(&lead, fs::Permissions::from_mode(0o700)).unwrap();

    let brief = root.join("brief.md");
    fs::write(&brief, "Eine Datei hello.txt anlegen\n\nMit einem Gruß.\n").unwrap();
    let gate = root.join("gate.sh");
    fs::write(&gate, "test -f hello.txt || { echo 'FAIL: hello.txt fehlt'; exit 1; }\n: > /dev/null || { echo 'FAIL: null device'; exit 1; }\nmkdir /tmp/taik-gate-$$ && rmdir /tmp/taik-gate-$$ || { echo 'FAIL: no fixtures in /tmp'; exit 1; }\nmade=$(mktemp -d) && rmdir \"$made\" || { echo 'FAIL: mktemp'; exit 1; }\nif touch \"$HOME/.taik-gate-probe\" 2>/dev/null; then rm -f \"$HOME/.taik-gate-probe\"; echo 'FAIL: the gate can write into the home'; exit 1; fi\necho 'PASS: hello.txt ist da'\n").unwrap();
    let queue = |name: &str| {
        let queued = factory()
            .args(["queue-session", state.to_str().unwrap(), name])
            .arg(&brief)
            .arg(&gate)
            .arg(&repo)
            .output()
            .unwrap();
        assert_eq!(queued.status.code(), Some(0), "{queued:?}");
        state.join(format!("jobs/job-{name}"))
    };
    let serve = || {
        factory()
            .arg("drain")
            .arg(&state)
            .env("TAIK_SESSION_AGENT", &agent)
            .env("TAIK_LEAD_WORKER", &lead)
            .env_remove("TAIK_SECOND_WORKER")
            .env_remove("TAIK_AUTO_PROPOSE")
            .output()
            .unwrap()
    };
    let field = |record: &str, key: &str| {
        record
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or("")
            .to_owned()
    };

    // 1. The agent writes a file. The Factory commits it, the gate judges that commit.
    fs::write(&mode, "write").unwrap();
    let job = queue("gruss");
    let served = serve();
    let record =
        fs::read_to_string(job.join("round-01.run")).unwrap_or_else(|_| panic!("{served:?}"));
    assert!(record.starts_with("decision=accepted\n"), "{record}");
    assert_eq!(field(&record, "checks"), "PASS: hello.txt ist da");
    assert_eq!(field(&record, "files"), "hello.txt +1 -0");
    assert!(field(&record, "summary").contains("resumed=no"), "{record}");
    let copy = state.join("work/gruss");
    let commit = field(&record, "commit");
    assert_eq!(git(&copy, &["rev-parse", "HEAD"]).trim(), commit);
    assert_eq!(field(&record, "artifact"), commit);
    assert!(
        fs::read_to_string(job.join("round-01.diff"))
            .unwrap()
            .contains("+hi")
    );
    let dialog = fs::read_to_string(job.join("round-01.dialog.jsonl")).unwrap();
    assert!(
        dialog.contains(r#""text":"Ich lege die Datei an.""#)
            && dialog.contains(r#""target":"hello.txt""#),
        "{dialog}"
    );
    let steps = fs::read_to_string(job.join("round-01.progress")).unwrap();
    assert!(steps.contains("\ttool\tWrite hello.txt\n"), "{steps}");
    // The user's repository is untouched, and the copy cannot push back.
    assert!(!repo.join("hello.txt").exists());
    assert_eq!(git(&copy, &["remote"]).trim(), "");
    // The accepted commit is settled: it is a branch of the repository and recorded in the job.
    assert_eq!(
        git(&repo, &["rev-parse", "refs/heads/taik/gruss"]).trim(),
        commit
    );
    assert_eq!(
        fs::read_to_string(job.join("adopted.txt")).unwrap(),
        "taik/gruss\n"
    );
    assert!(!job.join("adopt-failed.txt").exists());
    assert_eq!(git(&repo, &["status", "--porcelain"]).trim(), "");
    // A finished job is left alone.
    serve();
    assert!(!job.join("round-02.run").exists());

    // 2. A reply continues the same session.
    let mut reply = factory()
        .args(["reply", state.to_str().unwrap(), "job-gruss"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    reply
        .stdin
        .take()
        .unwrap()
        .write_all(b"Bitte noch einmal.")
        .unwrap();
    assert!(reply.wait().unwrap().success());
    serve();
    let second = fs::read_to_string(job.join("round-02.run")).unwrap();
    assert!(
        field(&second, "summary").contains("resumed=yes"),
        "{second}"
    );
    assert_eq!(field(&second, "session"), field(&record, "session"));
    assert!(
        fs::read_to_string(job.join("round-02.dialog.jsonl"))
            .unwrap()
            .contains("Bitte noch einmal.")
    );

    // 3. A change to a protected path is rejected without running the gate, and explained.
    fs::write(&mode, "protected").unwrap();
    let guarded = queue("gesperrt");
    let guarded_run = serve();
    assert_eq!(guarded_run.status.code(), Some(1));
    let refused = fs::read_to_string(guarded.join("round-01.run")).unwrap();
    assert!(refused.starts_with("decision=rejected\n"), "{refused}");
    assert_eq!(
        field(&refused, "checks"),
        "the change touches protected paths: locked/keep.txt"
    );
    assert!(!guarded.join("round-01.gate.log").exists());
    // The next rounds continue the session with the finding, up to the budget.
    let next = fs::read_to_string(guarded.join("round-02.run")).unwrap();
    assert!(field(&next, "summary").contains("resumed=yes"), "{next}");
    assert!(guarded.join("round-06.run").exists() && !guarded.join("round-07.run").exists());
    assert!(
        guarded.join("round-06.analysis.md").exists(),
        "{}",
        String::from_utf8_lossy(&guarded_run.stderr)
    );

    // 4. The agent asks. Nothing is judged until the user answered.
    fs::write(&mode, "ask").unwrap();
    let asking = queue("frage");
    serve();
    assert_eq!(
        fs::read_to_string(asking.join("message-001.agent.md")).unwrap(),
        "round=0\n\nWelche Farbe?\n"
    );
    assert!(!asking.join("round-01.run").exists());
    serve();
    assert!(!asking.join("round-01.run").exists());

    // 5. The sandbox: nothing outside the copy can be written, the home and the jobs cannot be read.
    fs::write(&mode, "outside").unwrap();
    let fenced = queue("zaun");
    serve();
    let seen = fs::read_to_string(fenced.join("round-01.run")).unwrap();
    assert!(
        field(&seen, "summary").contains("wrote=no read=no"),
        "{seen}"
    );

    // 6. A failed settling is recorded and changes neither the decision nor the rounds.
    fs::write(&mode, "write").unwrap();
    // A commit beside HEAD, from which the copy does not descend: no ancestor of the session commit.
    let side = git(
        &repo,
        &["commit-tree", "-p", "HEAD", "-m", "besetzt", "HEAD^{tree}"],
    );
    git(&repo, &["branch", "taik/besetzt", side.trim()]);
    let occupied = queue("besetzt");
    let occupied_run = serve();
    let accepted = fs::read_to_string(occupied.join("round-01.run"))
        .unwrap_or_else(|_| panic!("{occupied_run:?}"));
    assert!(accepted.starts_with("decision=accepted\n"), "{accepted}");
    let failed = fs::read_to_string(occupied.join("adopt-failed.txt"))
        .unwrap_or_else(|_| panic!("{occupied_run:?}"));
    assert_eq!(failed.lines().count(), 1, "{failed:?}");
    assert!(failed.contains("taik/besetzt"), "{failed:?}");
    assert!(!occupied.join("adopted.txt").exists());
    assert!(!occupied.join("round-02.run").exists());
    serve();
    assert!(!occupied.join("round-02.run").exists());

    // 7. A session job without a repository has nothing to settle.
    let bare_brief = root.join("bare.md");
    fs::write(
        &bare_brief,
        "Eine Datei hello.txt anlegen\n\nOhne Repository.\n",
    )
    .unwrap();
    let bare = factory()
        .args(["queue-session", state.to_str().unwrap(), "leer"])
        .arg(&bare_brief)
        .arg(&gate)
        .output()
        .unwrap();
    assert_eq!(bare.status.code(), Some(0), "{bare:?}");
    let bare = state.join("jobs/job-leer");
    serve();
    let alone = fs::read_to_string(bare.join("round-01.run")).unwrap();
    assert!(alone.starts_with("decision=accepted\n"), "{alone}");
    assert!(!bare.join("adopted.txt").exists() && !bare.join("adopt-failed.txt").exists());
}
