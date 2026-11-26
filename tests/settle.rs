use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn git(folder: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn settle(state: &Path, job: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
        .arg("settle")
        .arg(state)
        .arg(job)
        .output()
        .unwrap()
}

// The names in a folder, sorted, without descending.
fn entries(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

struct Setup {
    _root: tempfile::TempDir,
    state: std::path::PathBuf,
    origin: std::path::PathBuf,
    job: std::path::PathBuf,
    base: String,
    commit: String,
}

// An origin with one commit, its copy in STATE/work/demo with one more, and a job naming both.
fn setup() -> Setup {
    let root = tempfile::tempdir().unwrap();
    let origin = root.path().join("origin");
    fs::create_dir(&origin).unwrap();
    git(&origin, &["init", "--quiet"]);
    fs::write(origin.join("a.txt"), "a\n").unwrap();
    git(&origin, &["add", "a.txt"]);
    git(&origin, &["commit", "--quiet", "-m", "start"]);
    let base = git(&origin, &["rev-parse", "HEAD"]);

    let state = root.path().join("state");
    let work = state.join("work").join("demo");
    fs::create_dir_all(state.join("work")).unwrap();
    git(
        root.path(),
        &[
            "clone",
            "--quiet",
            "--local",
            "--no-hardlinks",
            origin.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    git(&work, &["remote", "remove", "origin"]);
    fs::write(work.join("b.txt"), "b\n").unwrap();
    git(&work, &["add", "b.txt"]);
    git(&work, &["commit", "--quiet", "-m", "agent"]);
    let commit = git(&work, &["rev-parse", "HEAD"]);

    let job = state.join("jobs").join("job-demo");
    fs::create_dir_all(&job).unwrap();
    fs::write(job.join("session.repo"), format!("{}\n", origin.display())).unwrap();
    fs::write(
        job.join("round-01.run"),
        format!("decision=rejected\ncommit={base}\nend\n"),
    )
    .unwrap();
    fs::write(
        job.join("round-02.run"),
        format!("decision=accepted\nkind=software\ncommit={commit}\nend\n"),
    )
    .unwrap();
    Setup {
        _root: root,
        state,
        origin,
        job,
        base,
        commit,
    }
}

#[test]
fn an_adopted_job_records_its_branch() {
    let s = setup();
    let output = settle(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"taik/demo\n");
    assert_eq!(
        fs::read_to_string(s.job.join("adopted.txt")).unwrap(),
        "taik/demo\n"
    );
    assert!(!s.job.join("adopt-failed.txt").exists());
    assert_eq!(
        git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]),
        s.commit
    );
    assert_eq!(git(&s.origin, &["rev-parse", "HEAD"]), s.base);
    // No temporary file is left behind.
    assert!(
        entries(&s.job).iter().all(|name| !name.starts_with('.')),
        "{:?}",
        entries(&s.job)
    );

    // Again is still success, and the record stays as it is.
    let again = settle(&s.state, "job-demo");
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(again.stdout, b"taik/demo\n");
    assert_eq!(
        fs::read_to_string(s.job.join("adopted.txt")).unwrap(),
        "taik/demo\n"
    );
}

#[test]
fn an_existing_record_is_left_unchanged() {
    let s = setup();
    fs::write(s.job.join("adopted.txt"), "written before\n").unwrap();
    let output = settle(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"taik/demo\n");
    assert_eq!(
        fs::read_to_string(s.job.join("adopted.txt")).unwrap(),
        "written before\n"
    );
}

#[test]
fn a_failed_adoption_is_recorded_in_one_line() {
    let s = setup();
    fs::write(
        s.job.join("round-03.run"),
        format!("decision=rejected\ncommit={}\nend\n", s.commit),
    )
    .unwrap();
    fs::write(
        s.job.join("adopt-failed.txt"),
        "an older failure\nin two lines\n",
    )
    .unwrap();
    let output = settle(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let failed = fs::read_to_string(s.job.join("adopt-failed.txt")).unwrap();
    let line = failed.strip_suffix('\n').expect("one line with its end");
    assert!(
        !line.trim().is_empty() && !line.contains('\n'),
        "{failed:?}"
    );
    assert!(line.contains("not accepted"), "{failed:?}");
    assert!(!s.job.join("adopted.txt").exists());
    assert_eq!(git(&s.origin, &["branch", "--list", "taik/*"]), "");
}

#[test]
fn a_branch_on_another_commit_is_a_failure() {
    let s = setup();
    // A commit of the origin beside the accepted one: not its ancestor.
    let side = git(
        &s.origin,
        &["commit-tree", "-p", &s.base, "-m", "side", "HEAD^{tree}"],
    );
    git(&s.origin, &["branch", "taik/demo", &side]);
    let output = settle(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let failed = fs::read_to_string(s.job.join("adopt-failed.txt")).unwrap();
    assert_eq!(failed.lines().count(), 1, "{failed:?}");
    assert!(failed.ends_with('\n') && !failed.trim().is_empty());
    assert!(!s.job.join("adopted.txt").exists());
    assert_eq!(
        git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]),
        side
    );
}

#[test]
fn a_job_without_repository_is_left_alone() {
    let s = setup();
    fs::remove_file(s.job.join("session.repo")).unwrap();
    let before = entries(&s.job);
    let output = settle(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_eq!(entries(&s.job), before);
    assert_eq!(git(&s.origin, &["branch", "--list", "taik/*"]), "");
}

#[test]
fn an_unknown_job_is_refused_and_nothing_is_created() {
    let s = setup();
    let jobs = s.state.join("jobs");
    let before = entries(&jobs);
    let output = settle(&s.state, "job-missing");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(entries(&jobs), before);
    assert!(!jobs.join("job-missing").exists());

    // Nor in a state folder that does not exist.
    let nowhere = s.state.join("nowhere");
    let output = settle(&nowhere, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(!nowhere.exists());
}
