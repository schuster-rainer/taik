#!/bin/sh
# Gate: Zweig rückt bei adopt vor (nur Fast-Forward).
fail() { echo "FAIL: $1"; exit 1; }

test -f tests/adopt_forward.rs || fail "tests/adopt_forward.rs fehlt"
test -f tests/adopt.rs || fail "tests/adopt.rs fehlt"
for t in a_branch_on_another_commit_is_refused the_accepted_commit_becomes_a_branch_of_the_origin a_last_round_that_is_not_accepted_is_refused an_unknown_job_or_a_job_without_repository_is_refused; do
  grep -q "fn $t" tests/adopt.rs || fail "Test $t fehlt in tests/adopt.rs"
done

out=$(cargo clippy --offline --locked --all-targets -- -D warnings 2>&1) || { echo "$out" | tail -40; fail "clippy hat Befunde"; }

out=$(cargo test --offline --locked --test adopt 2>&1) || { echo "$out" | tail -40; fail "tests/adopt.rs schlägt fehl"; }
out=$(cargo test --offline --locked --test adopt_forward 2>&1) || { echo "$out" | tail -40; fail "tests/adopt_forward.rs schlägt fehl"; }
echo "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed' || fail "adopt_forward führt keinen Test aus"

cat > tests/zz_gate_forward.rs <<'EOF'
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn git(folder: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git").arg("-C").arg(folder)
        .args(["-c","user.name=T","-c","user.email=t@example.invalid","-c","commit.gpgsign=false","-c","core.hooksPath=/dev/null"])
        .args(arguments).output().unwrap();
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
fn run(state: &Path, verb: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_taik-seed")).arg(verb).arg(state).arg("job-demo").output().unwrap()
}
struct S { _r: tempfile::TempDir, state: std::path::PathBuf, origin: std::path::PathBuf, job: std::path::PathBuf, base: String, commit: String }
fn setup() -> S {
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
    git(root.path(), &["clone","--quiet","--local","--no-hardlinks",origin.to_str().unwrap(),work.to_str().unwrap()]);
    git(&work, &["remote", "remove", "origin"]);
    fs::write(work.join("b.txt"), "b\n").unwrap();
    git(&work, &["add", "b.txt"]);
    git(&work, &["commit", "--quiet", "-m", "agent"]);
    let commit = git(&work, &["rev-parse", "HEAD"]);
    let job = state.join("jobs").join("job-demo");
    fs::create_dir_all(&job).unwrap();
    fs::write(job.join("session.repo"), format!("{}\n", origin.display())).unwrap();
    fs::write(job.join("round-01.run"), format!("decision=accepted\nkind=software\ncommit={commit}\nend\n")).unwrap();
    S { _r: root, state, origin, job, base, commit }
}

#[test]
fn forward_adopt() {
    let s = setup();
    git(&s.origin, &["branch", "taik/demo", &s.base]);
    let o = run(&s.state, "adopt");
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(o.stdout, b"taik/demo\n");
    assert_eq!(git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]), s.commit);
    assert_eq!(git(&s.origin, &["rev-parse", "HEAD"]), s.base);
    assert_eq!(git(&s.origin, &["status", "--porcelain"]), "");
    assert!(!s.origin.join("b.txt").exists());
}

#[test]
fn forward_settle_writes_once() {
    let s = setup();
    git(&s.origin, &["branch", "taik/demo", &s.base]);
    let o = run(&s.state, "settle");
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]), s.commit);
    assert_eq!(fs::read_to_string(s.job.join("adopted.txt")).unwrap(), "taik/demo\n");
    let o = run(&s.state, "settle");
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(fs::read_to_string(s.job.join("adopted.txt")).unwrap(), "taik/demo\n");
}

#[test]
fn sideways_refused() {
    let s = setup();
    git(&s.origin, &["checkout", "--quiet", "-b", "side"]);
    fs::write(s.origin.join("x.txt"), "x\n").unwrap();
    git(&s.origin, &["add", "x.txt"]);
    git(&s.origin, &["commit", "--quiet", "-m", "side"]);
    let side = git(&s.origin, &["rev-parse", "HEAD"]);
    git(&s.origin, &["branch", "taik/demo", &side]);
    let o = run(&s.state, "adopt");
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert_eq!(git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]), side);
}

#[test]
fn checked_out_refused() {
    let s = setup();
    git(&s.origin, &["checkout", "--quiet", "-b", "taik/demo"]);
    let o = run(&s.state, "adopt");
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert_eq!(git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]), s.base);
    assert_eq!(git(&s.origin, &["symbolic-ref", "HEAD"]), "refs/heads/taik/demo");
    assert_eq!(git(&s.origin, &["status", "--porcelain"]), "");
    let o = run(&s.state, "settle");
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(!s.job.join("adopted.txt").exists());
}

#[test]
fn no_branch_creates() {
    let s = setup();
    let o = run(&s.state, "adopt");
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(git(&s.origin, &["rev-parse", "refs/heads/taik/demo"]), s.commit);
}
EOF

out=$(cargo test --offline --locked --test zz_gate_forward 2>&1); rc=$?
rm -f tests/zz_gate_forward.rs
if [ $rc -ne 0 ]; then echo "$out" | tail -50; fail "Verhalten von adopt/settle stimmt nicht"; fi

echo "PASS: Zweig rückt nur vorwärts nach, Abweisungen bleiben, Tests und clippy sauber"
