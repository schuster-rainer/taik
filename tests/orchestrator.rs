#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

fn files(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in fs::read_dir(folder).unwrap().flatten() {
        let path = entry.path();
        found.push(path.display().to_string());
        if path.is_dir() && !path.is_symlink() {
            found.extend(files(&path));
        }
    }
    found.sort();
    found
}

#[test]
fn the_orchestrator_shows_products_protected_paths_and_failed_agents() {
    let root = tempfile::Builder::new()
        .prefix("taik-orchestrator-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = root.path();
    let state = root.join("state");
    for folder in ["crew", "products", "repos", "requests"] {
        fs::create_dir_all(state.join(folder)).unwrap();
    }
    for name in ["anna", "bert"] {
        fs::write(
            state.join(format!("crew/{name}.agent")),
            "harness=claude\nmodel=m\neffort=low\npersona=\nend\n",
        )
        .unwrap();
    }
    // anna failed a moment ago, bert seven hours ago.
    fs::write(state.join("crew/anna.failed"), "timeout\n").unwrap();
    fs::write(state.join("crew/bert.failed"), "timeout\n").unwrap();
    fs::File::options()
        .write(true)
        .open(state.join("crew/bert.failed"))
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(7 * 60 * 60))
        .unwrap();

    // One repository with protected paths, one without the file.
    let guarded = root.join("guarded");
    fs::create_dir_all(guarded.join(".git")).unwrap();
    fs::create_dir_all(guarded.join(".taik")).unwrap();
    fs::write(guarded.join("README.md"), "Guarded repository.\n").unwrap();
    fs::write(
        guarded.join(".taik/protected"),
        "# a comment\nSPEC.md\n\n  tools/gate.sh  \n# another\n.taik/\n",
    )
    .unwrap();
    let open = root.join("open");
    fs::create_dir_all(open.join(".git")).unwrap();
    fs::write(open.join("README.md"), "Open repository.\n").unwrap();
    fs::write(
        state.join("repos/guarded.repo"),
        format!("{}\n", guarded.display()),
    )
    .unwrap();
    fs::write(
        state.join("repos/open.repo"),
        format!("{}\n", open.display()),
    )
    .unwrap();

    let request = state.join("requests/ticket-001.run");
    fs::write(
        &request,
        "decision=observed\nidentity=ticket-001\nintent=Die Uhr bekommt Sekunden\nend\n",
    )
    .unwrap();

    // A worker that leaves a trace when it is called.
    let trace = root.join("called");
    let worker = root.join("worker");
    fs::write(
        &worker,
        format!("#!/bin/sh\ntouch '{}'\necho '{{}}'\n", trace.display()),
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();

    let show = || {
        let output = Command::new(env!("CARGO_BIN_EXE_taik-seed"))
            .arg("orchestrator-prompt")
            .arg(&state)
            .arg(&request)
            .env("TAIK_LEAD_WORKER", &worker)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        String::from_utf8(output.stdout).unwrap()
    };

    // Without products.
    let before = files(&state);
    let text = show();
    assert_eq!(files(&state), before, "the command wrote into STATE");
    assert!(!trace.exists(), "the command called the worker");
    let lines: Vec<&str> = text.lines().collect();
    let products = lines.iter().position(|line| *line == "PRODUCTS:").unwrap();
    assert_eq!(lines[products + 1], "(none)", "{text}");

    // Failed agents.
    let agent = |name: &str| {
        *lines
            .iter()
            .find(|line| line.starts_with(&format!("- {name}: ")))
            .unwrap_or_else(|| panic!("no line for {name}:\n{text}"))
    };
    assert!(agent("anna").ends_with(" (failed recently)"), "{text}");
    assert!(!agent("bert").contains("(failed recently)"), "{text}");
    assert!(!agent("standard").contains("(failed recently)"), "{text}");

    // Protected paths under each repository.
    let repository = |name: &str| {
        lines
            .iter()
            .position(|line| line.starts_with(&format!("- {name}: ")))
            .unwrap_or_else(|| panic!("no line for {name}:\n{text}"))
    };
    assert_eq!(
        lines[repository("guarded")],
        "- guarded: Guarded repository. "
    );
    assert_eq!(
        lines[repository("guarded") + 1],
        "  protected: SPEC.md, tools/gate.sh, .taik/"
    );
    assert_eq!(lines[repository("open") + 1], "  protected: (none)");
    assert!(
        text.contains("A change to a listed product is of the kind 'dokument'")
            && text.contains("A session may not change the protected paths"),
        "{text}"
    );
    assert!(
        text.ends_with("intent=Die Uhr bekommt Sekunden\nend\n"),
        "{text}"
    );

    // Products: valid ones are listed, invalid ones not.
    fs::write(
        state.join("products/uhr.product"),
        "about=Die Uhr im Browser\nseed=/tmp/seed.html\ngate=/tmp/gate.mjs\nend\n",
    )
    .unwrap();
    fs::write(
        state.join("products/alt.product"),
        "about=Ein altes Spiel\nseed=/tmp/seed.html\ngate=/tmp/gate.mjs\n",
    )
    .unwrap();
    fs::write(
        state.join("products/ohne-gate.product"),
        "about=Ohne Gate\nseed=/tmp/seed.html\nend\n",
    )
    .unwrap();
    fs::write(
        state.join("products/Gross.product"),
        "about=Falscher Name\nseed=/tmp/seed.html\ngate=/tmp/gate.mjs\nend\n",
    )
    .unwrap();
    let before = files(&state);
    let text = show();
    assert_eq!(files(&state), before, "the command wrote into STATE");
    assert!(!trace.exists(), "the command called the worker");
    let lines: Vec<&str> = text.lines().collect();
    let products = lines.iter().position(|line| *line == "PRODUCTS:").unwrap();
    assert_eq!(lines[products + 1], "- uhr: Die Uhr im Browser", "{text}");
    assert!(
        lines[products + 2].is_empty() || !lines[products + 2].starts_with("- "),
        "{text}"
    );
}
