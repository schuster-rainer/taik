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

fn adopt(state: &Path, job: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
        .arg("adopt")
        .arg(state)
        .arg(job)
        .output()
        .unwrap()
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
fn the_accepted_commit_becomes_a_branch_of_the_origin() {
    let s = setup();
    let head = git(&s.origin, &["symbolic-ref", "HEAD"]);

    let output = adopt(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"taik/demo\n");
    assert_eq!(
        git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]),
        s.commit
    );
    // Nothing checked out, merged or changed in the working files.
    assert_eq!(git(&s.origin, &["symbolic-ref", "HEAD"]), head);
    assert_eq!(git(&s.origin, &["rev-parse", "HEAD"]), s.base);
    assert!(!s.origin.join("b.txt").exists());
    assert_eq!(git(&s.origin, &["status", "--porcelain"]), "");

    // Again on the same commit is still success.
    let again = adopt(&s.state, "job-demo");
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(again.stdout, b"taik/demo\n");
}

#[test]
fn a_branch_on_another_commit_is_refused() {
    let s = setup();
    // A commit of the origin beside the accepted one: not its ancestor.
    let side = git(
        &s.origin,
        &["commit-tree", "-p", &s.base, "-m", "side", "HEAD^{tree}"],
    );
    git(&s.origin, &["branch", "taik/demo", &side]);
    let output = adopt(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]), side);
}

#[test]
fn a_last_round_that_is_not_accepted_is_refused() {
    let s = setup();
    fs::write(
        s.job.join("round-03.run"),
        format!("decision=rejected\ncommit={}\nend\n", s.commit),
    )
    .unwrap();
    let output = adopt(&s.state, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let branches = git(&s.origin, &["branch", "--list", "taik/*"]);
    assert_eq!(branches, "");
}

#[test]
fn an_unknown_job_or_a_job_without_repository_is_refused() {
    let s = setup();
    assert_eq!(adopt(&s.state, "job-missing").status.code(), Some(2));
    fs::remove_file(s.job.join("session.repo")).unwrap();
    assert_eq!(adopt(&s.state, "job-demo").status.code(), Some(2));
    assert_eq!(git(&s.origin, &["branch", "--list", "taik/*"]), "");
}
