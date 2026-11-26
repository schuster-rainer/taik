#![cfg(target_os = "macos")]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, SystemTime};

struct Factory {
    _root: tempfile::TempDir,
    root: PathBuf,
    state: PathBuf,
}

impl Factory {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("taik-reflect-")
            .tempdir_in("/tmp")
            .unwrap();
        let path = root.path().canonicalize().unwrap();
        let state = path.join("state");
        fs::create_dir_all(state.join("jobs/job-origin")).unwrap();
        // A worker that leaves a mark when anyone calls it.
        let worker = path.join("worker");
        fs::write(
            &worker,
            format!(
                "#!/bin/sh\ntouch '{}'\nexit 1\n",
                path.join("called").display()
            ),
        )
        .unwrap();
        fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
        Factory {
            _root: root,
            root: path,
            state,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_taik-seed"));
        let worker = self.root.join("worker");
        command
            .env("TAIK_LEAD_WORKER", &worker)
            .env("TAIK_WORKER", &worker)
            .env("TAIK_ORCHESTRATOR_WORKER", &worker)
            .env_remove("TAIK_AUTO_PROPOSE");
        command
    }

    fn reflect(&self) -> Output {
        let output = self
            .command()
            .arg("reflect")
            .arg(&self.state)
            .arg("job-origin")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(
            !self.root.join("called").exists(),
            "reflect called a worker"
        );
        output
    }

    fn request(&self, text: &str) -> String {
        let mut child = self
            .command()
            .arg("request")
            .arg(&self.state)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    /// An observation record with an open intent, changed `age` ago.
    fn observation(&self, job: &str, file: &str, intent: &str, age: Duration) -> PathBuf {
        let folder = self.state.join("jobs").join(job);
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(file);
        fs::write(
            &path,
            format!("decision=observed\nidentity={job}\ncontinuation={intent}\nend\n"),
        )
        .unwrap();
        age_file(&path, age);
        path
    }

    fn requests(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.state.join("requests"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn read_request(&self, name: &str) -> String {
        fs::read_to_string(self.state.join("requests").join(format!("{name}.run"))).unwrap()
    }
}

fn age_file(path: &Path, age: Duration) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
}

const HOUR: Duration = Duration::from_secs(60 * 60);

#[test]
fn the_oldest_open_intent_becomes_a_request_of_the_factory() {
    let factory = Factory::new();
    let intent: String = format!("{} und dann noch mehr Text am Ende", "ä".repeat(78));
    let old = factory.observation("job-alt", "observation.run", &intent, HOUR * 5);
    factory.observation("job-neu", "summary.run", "Ein neueres Vorhaben", HOUR);
    let output = factory.reflect();
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ticket-001");
    assert_eq!(factory.requests(), ["ticket-001.run"]);
    let title: String = intent.chars().take(80).collect();
    assert_eq!(
        factory.read_request("ticket-001"),
        format!(
            "decision=observed\nidentity=ticket-001\nintent=Vorhaben der Factory: {title}\nobservation={intent}\ncontinuation=Derive one job that fulfils this request of the user. Stop if the request is unclear or cannot be checked by a gate.\norigin=reflection\nintent_from={}\nend\n",
            old.display()
        )
    );
    assert!(!factory.state.join("proposals").exists());
}

#[test]
fn a_request_of_the_user_keeps_its_format_and_the_next_free_name_is_taken() {
    let factory = Factory::new();
    assert_eq!(factory.request("Ein Titel\nMehr dazu.\n"), "ticket-001");
    assert_eq!(
        factory.read_request("ticket-001"),
        "decision=observed\nidentity=ticket-001\nintent=Ein Titel\nobservation=Mehr dazu.\ncontinuation=Derive one job that fulfils this request of the user. Stop if the request is unclear or cannot be checked by a gate.\nend\n"
    );
    factory.observation("job-a", "observation.run", "Etwas verbessern", HOUR);
    factory.reflect();
    let reflected = factory.read_request("ticket-002");
    assert!(reflected.contains("\norigin=reflection\n"), "{reflected}");
    assert!(!factory.read_request("ticket-001").contains("origin="));
}

#[test]
fn an_intent_taken_up_by_a_request_or_a_reflection_is_not_taken_again() {
    let factory = Factory::new();
    let first = factory.observation("job-a", "observation.run", "Erstes Vorhaben", HOUR * 3);
    let second = factory.observation("job-b", "observation.run", "Zweites Vorhaben", HOUR * 2);
    let third = factory.observation("job-c", "summary.run", "Drittes Vorhaben", HOUR);
    // A reflection beside a record has taken up the second intent, long ago.
    let note = factory.state.join("jobs/job-c/reflection-alt.run");
    fs::write(
        &note,
        format!(
            "decision=observed\nidentity=reflection-alt\nintent_from={}\nend\n",
            second.display()
        ),
    )
    .unwrap();
    age_file(&note, HOUR * 48);
    factory.reflect();
    assert!(
        factory
            .read_request("ticket-001")
            .contains(&format!("\nintent_from={}\n", first.display()))
    );
    factory.reflect();
    assert!(
        factory
            .read_request("ticket-002")
            .contains(&format!("\nintent_from={}\n", third.display()))
    );
    let output = factory.reflect();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "nothing to reflect"
    );
    assert_eq!(factory.requests(), ["ticket-001.run", "ticket-002.run"]);
}

#[test]
fn without_an_open_intent_nothing_is_written() {
    let factory = Factory::new();
    // Not an observation, and an observation without a continuation.
    fs::write(
        factory.state.join("jobs/job-origin/round-01.run"),
        "decision=accepted\ncontinuation=Weiter.\nend\n",
    )
    .unwrap();
    fs::write(
        factory.state.join("jobs/job-origin/observation.run"),
        "decision=observed\nidentity=x\nend\n",
    )
    .unwrap();
    let output = factory.reflect();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "nothing to reflect"
    );
    assert!(factory.requests().is_empty(), "{:?}", factory.requests());
    assert!(!factory.state.join("proposals").exists());
}

#[test]
fn three_reflections_in_a_day_are_the_limit() {
    let factory = Factory::new();
    factory.observation("job-a", "observation.run", "Ein Vorhaben", HOUR);
    // Old reflections do not count.
    for number in 1..=3 {
        let old = factory
            .state
            .join(format!("jobs/job-origin/reflection-old-{number}.run"));
        fs::write(&old, "decision=observed\nend\n").unwrap();
        age_file(&old, HOUR * 30);
    }
    // Two young reflections beside records and one young request of the Factory make three.
    for number in 1..=2 {
        fs::write(
            factory
                .state
                .join(format!("jobs/job-origin/reflection-young-{number}.run")),
            "decision=observed\nend\n",
        )
        .unwrap();
    }
    fs::create_dir_all(factory.state.join("requests")).unwrap();
    let young = factory.state.join("requests/ticket-001.run");
    fs::write(
        &young,
        "decision=observed\nidentity=ticket-001\norigin=reflection\nintent_from=/nirgends\nend\n",
    )
    .unwrap();
    let output = factory.reflect();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "nothing to reflect"
    );
    assert_eq!(factory.requests(), ["ticket-001.run"]);
    // Once the request is older than a day, there is room for one more.
    age_file(&young, HOUR * 25);
    factory.reflect();
    assert_eq!(factory.requests(), ["ticket-001.run", "ticket-002.run"]);
}
