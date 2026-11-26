//! `adopt` and `settle` move an existing branch `taik/NAME` forward to the newly accepted commit
//! when it points at an ancestor, and never backwards, sideways or while it is checked out.

use std::fs;
use std::path::{Path, PathBuf};
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

fn run(command: &str, state: &Path, job: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_taik-seed"))
        .arg(command)
        .arg(state)
        .arg(job)
        .output()
        .unwrap()
}

struct Setup {
    _root: tempfile::TempDir,
    state: PathBuf,
    origin: PathBuf,
    work: PathBuf,
    job: PathBuf,
    base: String,
    commit: String,
}

const BRANCH: &str = "refs/heads/taik/demo";

// An origin with one commit, its copy in STATE/work/demo with one more, and a job whose last
// round accepts that commit.
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
    let job = state.join("jobs").join("job-demo");
    fs::create_dir_all(&job).unwrap();
    fs::write(job.join("session.repo"), format!("{}\n", origin.display())).unwrap();
    let mut s = Setup {
        _root: root,
        state,
        origin,
        work,
        job,
        base,
        commit: String::new(),
    };
    s.commit = accept(&s, 1, "b.txt");
    s
}

// One more commit in the work copy, accepted as round NUMBER.
fn accept(s: &Setup, number: u8, file: &str) -> String {
    fs::write(s.work.join(file), format!("{file}\n")).unwrap();
    git(&s.work, &["add", file]);
    git(&s.work, &["commit", "--quiet", "-m", file]);
    let commit = git(&s.work, &["rev-parse", "HEAD"]);
    fs::write(
        s.job.join(format!("round-{number:02}.run")),
        format!("decision=accepted\nkind=software\ncommit={commit}\nend\n"),
    )
    .unwrap();
    commit
}

// HEAD, checked-out commit and working files of the origin are as at the start.
fn assert_origin_untouched(s: &Setup, head: &str) {
    assert_eq!(git(&s.origin, &["symbolic-ref", "HEAD"]), head);
    assert_eq!(git(&s.origin, &["rev-parse", "HEAD"]), s.base);
    assert!(!s.origin.join("b.txt").exists());
    assert!(!s.origin.join("c.txt").exists());
    assert_eq!(git(&s.origin, &["status", "--porcelain"]), "");
}

#[test]
fn without_a_branch_the_branch_is_created() {
    let s = setup();
    let head = git(&s.origin, &["symbolic-ref", "HEAD"]);
    let output = run("adopt", &s.state, "job-demo");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"taik/demo\n");
    assert_eq!(git(&s.origin, &["rev-parse", BRANCH]), s.commit);
    assert_origin_untouched(&s, &head);
}

#[test]
fn a_branch_on_an_ancestor_moves_forward() {
    let s = setup();
    let head = git(&s.origin, &["symbolic-ref", "HEAD"]);
    git(&s.origin, &["branch", "taik/demo", &s.base]);
    let output = run("adopt", &s.state, "job-demo");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"taik/demo\n");
    assert_eq!(git(&s.origin, &["rev-parse", BRANCH]), s.commit);
    assert_origin_untouched(&s, &head);
}

#[test]
fn a_further_accepted_round_moves_the_settled_branch_forward() {
    let s = setup();
    let head = git(&s.origin, &["symbolic-ref", "HEAD"]);
    let first = run("settle", &s.state, "job-demo");
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    assert_eq!(first.stdout, b"taik/demo\n");
    assert_eq!(git(&s.origin, &["rev-parse", BRANCH]), s.commit);
    let adopted = fs::read(s.job.join("adopted.txt")).unwrap();
    assert_eq!(adopted, b"taik/demo\n");

    let next = accept(&s, 2, "c.txt");
    let second = run("settle", &s.state, "job-demo");
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    assert_eq!(second.stdout, b"taik/demo\n");
    assert_eq!(git(&s.origin, &["rev-parse", BRANCH]), next);
    assert_eq!(fs::read(s.job.join("adopted.txt")).unwrap(), adopted);
    assert!(!s.job.join("adopt-failed.txt").exists());
    assert_origin_untouched(&s, &head);
}

#[test]
fn a_branch_on_a_commit_that_is_no_ancestor_is_refused() {
    let s = setup();
    let head = git(&s.origin, &["symbolic-ref", "HEAD"]);
    let side = git(
        &s.origin,
        &["commit-tree", "-p", &s.base, "-m", "side", "HEAD^{tree}"],
    );
    git(&s.origin, &["branch", "taik/demo", &side]);
    let output = run("adopt", &s.state, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(git(&s.origin, &["rev-parse", BRANCH]), side);
    assert_origin_untouched(&s, &head);

    // A newer branch is never moved back to an older accepted commit either.
    let newer = git(
        &s.origin,
        &["commit-tree", "-p", &side, "-m", "newer", "HEAD^{tree}"],
    );
    git(&s.origin, &["branch", "--force", "taik/demo", &newer]);
    let output = run("settle", &s.state, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(git(&s.origin, &["rev-parse", BRANCH]), newer);
    assert!(!s.job.join("adopted.txt").exists());
    assert!(s.job.join("adopt-failed.txt").exists());
}

#[test]
fn a_checked_out_branch_is_refused_even_on_an_ancestor() {
    let s = setup();
    git(&s.origin, &["checkout", "--quiet", "-b", "taik/demo"]);
    let output = run("adopt", &s.state, "job-demo");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(git(&s.origin, &["rev-parse", BRANCH]), s.base);
    assert_eq!(git(&s.origin, &["symbolic-ref", "HEAD"]), BRANCH);
    assert!(!s.origin.join("b.txt").exists());
    assert_eq!(git(&s.origin, &["status", "--porcelain"]), "");
}
