#![cfg(unix)]

use std::ffi::{CString, OsStr};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
}

fn invoke(args: &[&OsStr], exit: i32) -> Output {
    let output = cli().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    output
}

fn wait_for(mut ready: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(Instant::now() < end, "condition did not become true");
        thread::sleep(Duration::from_millis(10));
    }
}

struct Service(Child);

impl Service {
    fn start(state: &Path) -> Self {
        Self::start_with(state, &[])
    }

    fn start_with(state: &Path, options: &[&str]) -> Self {
        let log = state.join("test-service.log");
        let stderr = fs::File::create(&log).unwrap();
        let service = Self(
            cli()
                .arg("serve")
                .arg(state)
                .args(options)
                .stdout(Stdio::null())
                .stderr(stderr)
                .spawn()
                .unwrap(),
        );
        wait_for(|| {
            fs::read_to_string(&log)
                .unwrap()
                .contains("watching for jobs")
        });
        service
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn accepted_records_reject_a_fifo_without_blocking() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = dir.path().join("blocked.run");
    let pipe = CString::new(baseline.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(pipe.as_ptr(), 0o600) }, 0);
    let output = dir.path().join("unpublished.run");
    let mut child = Service(
        cli()
            .args(["extend"])
            .arg(baseline)
            .arg(&output)
            .arg("elm")
            .spawn()
            .unwrap(),
    );
    wait_for(|| child.0.try_wait().unwrap().is_some());
    assert_eq!(child.0.wait().unwrap().code(), Some(2));
    assert!(!output.exists());
}

#[cfg(target_os = "macos")]
#[test]
fn queued_source_repairs_an_assertion_and_resumes_from_saved_feedback() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::Builder::new()
        .prefix("taik-feedback-")
        .tempdir_in("/tmp")
        .unwrap();
    let bin = root.path().join("bin");
    let auth = root.path().join("auth");
    fs::create_dir(&bin).unwrap();
    fs::create_dir(&auth).unwrap();
    fs::write(auth.join("auth.json"), "{}").unwrap();
    let worker = bin.join("codex");
    fs::write(&worker, r#"#!/bin/sh
set -eu
if [ "$1" = login ]; then
  printf '%s\n' 'Logged in using ChatGPT'
  exit 0
fi
while [ "$1" != --output-last-message ]; do shift; done
output="$2"
shift 3
case "$1" in
  *'Add a route legend to the verified section'*'ORIGINAL WORK ORDER:'*'Publish a route and optional legend'*'ACCEPTED SOURCE TO BUILD ON:'*'<h1 data-verified>Safe route</h1>'*)
    printf '%s\n' '<!doctype html><h1 data-verified>Safe route</h1><p data-legend>Route legend</p>' > "$output" ;;
  *'Add a route legend to the verified section'*) exit 4 ;;
  *'Publish a route and optional legend'*)
    printf '%s\n' '<!doctype html><h1 data-verified>Safe route</h1>' > "$output" ;;
  *'GATE DIAGNOSIS:'*'Gate lacked verified section'*'REJECTED HTML:'*'<h1>Missing section</h1>'*)
    case "$1" in
      *'Return the same draft'*) printf '%s\n' '<!doctype html><h1>Missing section</h1>' > "$output" ;;
      *) printf '%s\n' '<!doctype html><h1 data-verified>Corrected section</h1>' > "$output" ;;
    esac ;;
  *'GATE DIAGNOSIS:'*) exit 3 ;;
  *) printf '%s\n' '<!doctype html><h1>Missing section</h1>' > "$output" ;;
esac
"#).unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let mut directories = vec![bin];
    directories.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(directories).unwrap();
    let drain = |state: &Path, expected| {
        let output = cli()
            .arg("drain")
            .arg(state)
            .env("PATH", &path)
            .env("CODEX_HOME", &auth)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected), "{output:?}");
    };
    let brief = root.path().join("brief.md");
    let gate = root.path().join("gate.mjs");
    fs::write(&gate, r#"import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
assert(readFileSync(process.argv[2], 'utf8').includes('data-verified'), 'Gate lacked verified section');
console.log('PASS: corrected section');
"#).unwrap();
    let state = root.path().join("state");
    let submit = |name: &str, goal: &str| {
        fs::write(&brief, goal).unwrap();
        invoke(
            &[
                "queue-source".as_ref(),
                state.as_os_str(),
                name.as_ref(),
                brief.as_os_str(),
                gate.as_os_str(),
            ],
            0,
        );
        state.join("jobs").join(format!("job-{name}"))
    };
    let job = submit("repair", "Publish a checked section\n");
    drain(&state, 0);
    let rejection = fs::read_to_string(job.join("round-01.run")).unwrap();
    let acceptance = fs::read_to_string(job.join("round-02.run")).unwrap();
    assert!(rejection.starts_with("decision=rejected\n"));
    assert!(rejection.contains("rejected_candidate=round-01.rejected.txt\n"));
    assert!(rejection.contains("advance=true\n"));
    assert!(!job.join("round-01.html").exists());
    assert!(acceptance.starts_with("decision=accepted\n"));
    assert!(acceptance.contains("feedback=round-01.run\n"));
    assert!(acceptance.contains("checks=PASS: corrected section\n"));
    assert!(acceptance.contains("advance=false\n"));

    let resume_state = root.path().join("resume");
    let resume = resume_state.join("jobs/job-repair");
    fs::create_dir_all(&resume).unwrap();
    for name in [
        "submission.brief.md",
        "submission.gate.mjs",
        "gate.sha256",
        "request.run",
        "round-01.run",
        "round-01.rejected.txt",
    ] {
        fs::copy(job.join(name), resume.join(name)).unwrap();
    }
    drain(&resume_state, 0);
    assert_eq!(
        fs::read_to_string(resume.join("round-01.run")).unwrap(),
        rejection
    );
    assert!(
        fs::read_to_string(resume.join("round-02.run"))
            .unwrap()
            .contains("feedback=round-01.run\n")
    );

    let unchanged = submit("unchanged", "Return the same draft\n");
    drain(&state, 1);
    let stopped = fs::read_to_string(unchanged.join("round-02.run")).unwrap();
    assert!(stopped.contains("The worker returned the same rejected source."));
    assert!(!unchanged.join("round-03.run").exists());

    fs::write(&gate, "throw new Error('Broken gate infrastructure');\n").unwrap();
    let broken = submit("broken-gate", "Check a section\n");
    drain(&state, 1);
    assert!(
        fs::read_to_string(broken.join("round-01.run"))
            .unwrap()
            .contains("advance=false\n")
    );
    assert!(!broken.join("round-02.run").exists());

    let gate_script = r#"import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const html = readFileSync(process.argv[2], 'utf8');
assert(html.includes('<h1 data-verified>Safe route</h1>'), 'Required route changed');
const hasLegend = html.includes('data-legend');
if (process.argv[3] === 'follow-up') assert(hasLegend, 'Follow-up lacks the legend');
console.log('PASS: required route');
if (!hasLegend) console.log('TAIK_FOLLOWUP: Add a route legend to the verified section');
else if (!html.includes('aria-label')) console.log('TAIK_FOLLOWUP: Add an accessible label to the legend');
"#;
    fs::write(&gate, gate_script).unwrap();
    let ordinary = submit("ordinary", "Publish a route and optional legend\n");
    drain(&state, 0);
    assert!(
        fs::read_to_string(ordinary.join("round-01.run"))
            .unwrap()
            .contains("follow_up=\n")
    );
    assert!(!state.join("jobs/job-ordinary-folgeauftrag").exists());

    fs::write(&gate, format!("// TAIK: allow-follow-up\n{gate_script}")).unwrap();
    let parent = submit("route", "Publish a route and optional legend\n");
    let child = state.join("jobs/job-route-folgeauftrag");
    drain(&state, 0);
    let parent_report = fs::read_to_string(parent.join("round-01.run")).unwrap();
    let child_report = fs::read_to_string(child.join("round-01.run")).unwrap();
    assert!(parent_report.contains("follow_up=Add a route legend to the verified section\n"));
    assert!(child_report.starts_with("decision=accepted\n"));
    assert!(child_report.contains("follow_up=\n"));
    assert_eq!(
        fs::read_to_string(child.join("origin.txt")).unwrap(),
        "source=\"../job-route/round-01.run\"\n"
    );
    assert_eq!(
        fs::read(parent.join("gate.sha256")).unwrap(),
        fs::read(child.join("gate.sha256")).unwrap()
    );
    let original = fs::read_to_string(parent.join("round-01.html")).unwrap();
    assert!(!original.contains("data-legend"));
    assert_eq!(
        fs::read_to_string(child.join("submission.seed.html")).unwrap(),
        original
    );
    assert!(
        fs::read_to_string(child.join("round-01.html"))
            .unwrap()
            .contains("data-legend")
    );
    assert!(
        !state
            .join("jobs/job-route-folgeauftrag-folgeauftrag")
            .exists()
    );
    drain(&state, 0);
    assert_eq!(
        fs::read_to_string(child.join("round-01.run")).unwrap(),
        child_report
    );
    assert!(!child.join("round-02.run").exists());

    let retained = resume_state.join("jobs/job-route");
    fs::create_dir(&retained).unwrap();
    for name in [
        "submission.brief.md",
        "submission.gate.mjs",
        "gate.sha256",
        "request.run",
        "round-01.run",
        "round-01.html",
    ] {
        fs::copy(parent.join(name), retained.join(name)).unwrap();
    }
    drain(&resume_state, 0);
    assert!(
        resume_state
            .join("jobs/job-route-folgeauftrag/round-01.html")
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(retained.join("round-01.run")).unwrap(),
        parent_report
    );
}

#[test]
fn invalid_proposal_and_staging_cannot_replace_or_seed_accepted_work() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = dir.path().join("seed.run");
    invoke(&["seed".as_ref(), baseline.as_os_str()], 0);
    let before = fs::read(&baseline).unwrap();
    let rejected = dir.path().join("rejected.run");
    invoke(
        &[
            "evaluate".as_ref(),
            baseline.as_os_str(),
            rejected.as_os_str(),
            "lower,sort,trim,dedup,drop-blank".as_ref(),
        ],
        1,
    );
    assert!(
        fs::read_to_string(&rejected)
            .unwrap()
            .contains("order_preserves_contract=false")
    );
    let unused = dir.path().join("unused.run");
    invoke(
        &[
            "extend".as_ref(),
            rejected.as_os_str(),
            unused.as_os_str(),
            "elm".as_ref(),
        ],
        2,
    );
    let pending = dir.path().join("interrupted.pending");
    fs::write(&pending, &before).unwrap();
    invoke(
        &[
            "extend".as_ref(),
            pending.as_os_str(),
            unused.as_os_str(),
            "elm".as_ref(),
        ],
        2,
    );
    invoke(
        &[
            "extend".as_ref(),
            baseline.as_os_str(),
            baseline.as_os_str(),
            "elm".as_ref(),
        ],
        2,
    );
    assert_eq!(fs::read(&baseline).unwrap(), before);
    assert!(!unused.exists());
}

#[test]
fn concurrent_publication_has_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("one.run");
    let mut first = cli()
        .arg("seed")
        .arg(&target)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut second = cli()
        .arg("seed")
        .arg(&target)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut exits = [first.wait().unwrap().code(), second.wait().unwrap().code()];
    exits.sort_unstable();
    assert_eq!(exits, [Some(0), Some(2)]);
    assert!(fs::read_to_string(target).unwrap().ends_with("end\n"));
}

#[test]
fn service_resumes_from_an_accepted_round_and_wakes_for_new_work() {
    let dir = tempfile::Builder::new()
        .prefix("taik-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = dir.path().join("state");
    let baseline = dir.path().join("seed.run");
    invoke(&["seed".as_ref(), baseline.as_os_str()], 0);
    let job = invoke(
        &["queue".as_ref(), state.as_os_str(), baseline.as_os_str()],
        0,
    );
    let job = Path::new(std::str::from_utf8(&job.stdout).unwrap().trim());
    let first = job.join("round-01.run");
    invoke(
        &[
            "evolve".as_ref(),
            job.join("baseline.run").as_os_str(),
            first.as_os_str(),
        ],
        0,
    );
    let known_good = fs::read(&first).unwrap();
    fs::write(job.join(".taik-interrupted.pending"), &known_good).unwrap();

    let service = Service::start(&state);
    let terminal = job.join("round-03.run");
    wait_for(|| terminal.is_file());
    let final_record = fs::read_to_string(&terminal).unwrap();
    assert!(final_record.contains("probe_work=lowercase_bytes:9, sort_items:3"));
    assert!(final_record.contains("advance=false"));
    let progress = job.join("round-03.progress");
    wait_for(|| fs::read_to_string(&progress).is_ok_and(|trace| trace.contains("\tretained\t")));
    let trace = fs::read_to_string(progress).unwrap();
    for stage in [
        "start", "baseline", "probe", "trial", "evaluate", "checks", "retained",
    ] {
        assert!(trace.contains(&format!("\t{stage}\t")), "missing {stage}");
    }
    assert_eq!(fs::read(&first).unwrap(), known_good);
    drop(service); // Abruptly stop the daemon. The staged and accepted records remain.

    let service = Service::start(&state);
    let queued_search = invoke(&["queue-self".as_ref(), state.as_os_str()], 0);
    let search_job = Path::new(std::str::from_utf8(&queued_search.stdout).unwrap().trim());
    let final_search = search_job.join("round-03.run");
    wait_for(|| final_search.is_file());
    let search_record = fs::read_to_string(&final_search).unwrap();
    assert!(search_record.contains("advance=false"));
    assert_eq!(
        fs::read_to_string(search_job.join("round-01.policy")).unwrap(),
        "unique\n"
    );
    assert_eq!(
        fs::read_to_string(search_job.join("round-02.policy")).unwrap(),
        "valid-unique\n"
    );
    let research = state.join("jobs/job-suchentscheidungen-wiederverwenden");
    let research_output = research.join("round-01.run");
    wait_for(|| research_output.is_file());
    let research_record = fs::read_to_string(&research_output).unwrap();
    assert!(research_record.starts_with("decision=accepted\n"));
    assert!(research_record.contains("discovery_candidate_probes=16\n"));
    assert!(research_record.contains("policy=precomputed\n"));
    let request = fs::read_to_string(research.join("request.run")).unwrap();
    assert!(request.contains("evidence=../job-selbstverbesserung/round-02.run\n"));
    assert!(request.contains("observed_candidate_probes=16\n"));
    drop(service);

    #[cfg(target_os = "macos")]
    {
        let activation = state.join("jobs/job-suchtabelle-bei-bedarf");
        fs::create_dir(&activation).unwrap();
        fs::write(
            activation.join("round-02.run"),
            "decision=accepted\nkind=search\npolicy=first-use\nend\n",
        )
        .unwrap();
        let withheld = dir.path().join("table.run");
        fs::rename(&research_output, &withheld).unwrap();
        invoke(&["drain".as_ref(), state.as_os_str()], 0);
        assert!(
            !research_output.exists(),
            "idle startup must not load or recreate the table"
        );
        fs::rename(withheld, &research_output).unwrap();
    }

    let service = Service::start(&state);
    let simultaneous = invoke(&["drain".as_ref(), state.as_os_str()], 2);
    assert!(String::from_utf8_lossy(&simultaneous.stderr).contains("another Factory worker"));
    let inputs = state.join("jobs/_inputs");
    fs::create_dir(&inputs).unwrap();
    let reused = inputs.join("reused.run");
    invoke(
        &[
            "extend".as_ref(),
            terminal.as_os_str(),
            reused.as_os_str(),
            "  PINE ".as_ref(),
        ],
        0,
    );
    assert!(
        fs::read_to_string(&reused)
            .unwrap()
            .contains("tags=ash,cedar,pine")
    );
    let new_job = invoke(
        &["queue".as_ref(), state.as_os_str(), reused.as_os_str()],
        0,
    );
    let new_job = Path::new(std::str::from_utf8(&new_job.stdout).unwrap().trim());
    wait_for(|| new_job.join("round-01.run").is_file());
    assert_eq!(
        fs::read_to_string(new_job.join("origin.txt")).unwrap(),
        "source=\"../_inputs/reused.run\"\n"
    );
    let reused_record = fs::read_to_string(new_job.join("round-01.run")).unwrap();
    assert!(reused_record.contains("advance=false"));
    assert!(reused_record.contains("search_policy=precomputed\n"));
    assert!(reused_record.contains("search_probe_executions=0\n"));
    assert!(reused_record.contains("tags=ash,cedar,pine\n"));
    #[cfg(target_os = "macos")]
    assert!(
        fs::read_to_string(new_job.join("round-01.progress"))
            .unwrap()
            .contains("Suchtabelle vor der ersten Nutzung geprüft")
    );
    assert_eq!(
        fs::read_to_string(&research_output).unwrap(),
        research_record
    );
    assert_eq!(fs::read_to_string(&final_search).unwrap(), search_record);
    assert_eq!(fs::read_to_string(&terminal).unwrap(), final_record);
    drop(service);

    #[cfg(target_os = "macos")]
    {
        let service = Service::start_with(&state, &["--reflect"]);
        let job = state.join("jobs/job-suchtabelle-kaltstart");
        let output = job.join("round-01.run");
        wait_for(|| output.is_file());
        let result = fs::read_to_string(output).unwrap();
        assert!(result.starts_with("decision=accepted\n"), "{result}");
        assert!(result.contains("varied decision rejected before lookup"));
        assert_eq!(
            fs::read_to_string(&research_output).unwrap(),
            research_record
        );
        drop(service);
    }
}
