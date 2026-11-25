//! The installed Factory judges its successor and installs it.
//!
//! The author of a backend change supplies a brief and a gate. The gate is pinned for the life of
//! the job and builds the candidate. The installed binary then exercises the candidate with checks
//! of its own, writes the decision and replaces itself, keeping the previous binary.

use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::progress::Progress;
use crate::{Result, record, source};

const GATE_TIMEOUT: Duration = Duration::from_secs(900);
const SMOKE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ROUNDS: usize = 99;
const MAX_BINARY: u64 = 64 * 1024 * 1024;
const MAX_LOG_TAIL: u64 = 4096;

pub fn run(job: &Path, brief: &Path, gate: &Path, target: &Path, worker: &str) -> Result<bool> {
    let target = target.canonicalize()?;
    let judge = env::current_exe()?.canonicalize()?;
    if judge != target {
        return Err("a promotion is judged by the installed binary itself".into());
    }
    let brief = record::read_regular(brief, record::MAX_RECORD_BYTES)?;
    let intent = source::intent(&brief)?;
    let gate = record::read_regular(gate, source::LIMIT)?;
    let digest = format!("{:x}", Sha256::digest(gate.as_bytes()));
    fs::create_dir_all(job)?;
    let job = job.canonicalize()?;
    let _lock = source::lock_job(&job)?;
    source::pin(&job.join("gate.sha256"), &format!("{digest}\n"))?;
    source::pin(&job.join("request.run"), source::REQUEST)?;
    let number = (1..=MAX_ROUNDS)
        .find(|number| !job.join(format!("round-{number:02}.run")).exists())
        .ok_or("promotion round budget exhausted")?;
    let stem = format!("round-{number:02}");
    source::pin(&job.join(format!("{stem}.brief.md")), &brief)?;
    let output = job.join(format!("{stem}.run"));
    let log_name = format!("{stem}.checks.log");
    let artifact_name = format!("{stem}-taik-seed");
    let rollback_name = format!("{stem}.rollback-taik-seed");
    let installed = digest_of(&target)?;

    let mut progress = Progress::for_round(&output);
    progress.event(
        "start",
        format_args!("Die installierte Factory beurteilt ihren Nachfolger"),
    );
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let candidate = scratch.join("candidate");
    let mut gate_ms = String::new();
    let mut candidate_digest = String::new();
    let judgement = (|| -> Result<String> {
        progress.event("evaluate", format_args!("Fixiertes Gate ausführen"));
        let script = scratch.join("gate.sh");
        fs::write(&script, &gate)?;
        let started = Instant::now();
        let verdict = run_gate(&script, &candidate, &job.join(&log_name))?;
        gate_ms = started.elapsed().as_millis().to_string();
        progress.event("checks", format_args!("{verdict}"));
        let size = candidate
            .symlink_metadata()
            .map_err(|_| "the gate passed without producing a candidate")?;
        if !size.is_file() || size.len() == 0 || size.len() > MAX_BINARY {
            return Err("the candidate is not a regular binary within the size limit".into());
        }
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755))?;
        candidate_digest = digest_of(&candidate)?;
        let smoke = smoke(&candidate, &scratch)?;
        progress.event("compare", format_args!("{smoke}"));
        // Keep both binaries before the installed one changes.
        copy_new(&candidate, &job.join(&artifact_name))?;
        copy_new(&target, &job.join(&rollback_name))?;
        let next = target.with_extension("next");
        fs::copy(&candidate, &next)?;
        fs::set_permissions(&next, fs::Permissions::from_mode(0o755))?;
        fs::rename(&next, &target)?;
        if digest_of(&target)? != candidate_digest {
            return Err("the installed binary differs from the judged candidate".into());
        }
        Ok(format!("{verdict}; {smoke}"))
    })();
    let accepted = judgement.is_ok();
    let checks = judgement.unwrap_or_else(|error| {
        let reason = source::single_line(&error.to_string());
        progress.event("error", format_args!("{reason}"));
        reason
    });
    let (decision, artifact, rollback, continuation) = if accepted {
        (
            "accepted",
            artifact_name.as_str(),
            rollback_name.as_str(),
            "Restart the service so that it runs the accepted binary.",
        )
    } else {
        ("rejected", "", "", "The installed binary stays in place.")
    };
    let report = format!(
        "decision={decision}\nkind=software\nprocedure=\ntags=\nidentity={stem}\nintent={intent}\nbaseline=taik-seed {installed}\nworker={worker}\n\
         decided_by=the installed taik-seed {installed} at {}\nlimits={MAX_ROUNDS} rounds; gate 900s; candidate checks 10s each\n\
         gate_sha256={digest}\ngate_ms={gate_ms}\nchecks={checks}\nevidence={log_name}\nartifact={artifact}\nartifact_sha256={}\nrollback={rollback}\n\
         continuation={continuation}\nadvance=false\nend\n",
        judge.display(),
        if accepted { candidate_digest.as_str() } else { "" },
    );
    crate::emit(&output, &report)?;
    progress.event(decision, format_args!("{continuation}"));
    Ok(accepted)
}

fn digest_of(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file.by_ref().take(MAX_BINARY + 1), &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn copy_new(from: &Path, to: &Path) -> Result<()> {
    let mut source = File::open(from)?;
    let mut target = File::options().write(true).create_new(true).open(to)?;
    std::io::copy(&mut source, &mut target)?;
    target.set_permissions(fs::Permissions::from_mode(0o755))?;
    target.sync_all()?;
    Ok(())
}

// The gate builds and tests in the checkout. Its whole output goes to the log beside the record.
fn run_gate(script: &Path, candidate: &Path, log: &Path) -> Result<String> {
    let mut output = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(log)?;
    let mut child = Command::new("/bin/sh")
        .arg(script)
        .env("TAIK_CANDIDATE_OUT", candidate)
        .env_remove("TAIK_LEAD_WORKER")
        .env_remove("TAIK_SECOND_WORKER")
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output.try_clone()?)
        .process_group(0)
        .spawn()?;
    let deadline = Instant::now() + GATE_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            return Err("the gate exceeded its deadline".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let length = output.metadata()?.len();
    output.seek(std::io::SeekFrom::Start(length.saturating_sub(MAX_LOG_TAIL)))?;
    let mut tail = Vec::new();
    output.read_to_end(&mut tail)?;
    let tail = String::from_utf8_lossy(&tail);
    let last = tail
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("the gate printed nothing");
    let last = source::single_line(last);
    if status.success() {
        Ok(last)
    } else {
        Err(format!("gate failed ({status}): {last}").into())
    }
}

// Checks owned by the installed binary. They do not depend on the gate the author supplied.
fn smoke(candidate: &Path, scratch: &Path) -> Result<String> {
    let run = |arguments: &[&Path]| -> Result<Option<i32>> {
        let mut command = Command::new(candidate);
        command.args(arguments).current_dir(scratch);
        Ok(source::run_child(&mut command, SMOKE_TIMEOUT, |_| {})?
            .status
            .code())
    };
    let seed = scratch.join("smoke-seed.run");
    let rejected = scratch.join("smoke-rejected.run");
    let extended = scratch.join("smoke-extended.run");
    if run(&[Path::new("seed"), &seed])? != Some(0) {
        return Err("the candidate could not produce a seed record".into());
    }
    let accepted = record::read_accepted(&seed)
        .map_err(|_| "the installed binary cannot read the candidate's seed record")?;
    let before = fs::read(&seed)?;
    if run(&[Path::new("reject"), &seed, &rejected])? != Some(1) {
        return Err("the candidate did not reject a deliberately invalid procedure".into());
    }
    if fs::read(&seed)? != before {
        return Err("the candidate changed an accepted record while rejecting".into());
    }
    if run(&[Path::new("extend"), &seed, &extended, Path::new(" ELM ")])? != Some(0) {
        return Err("the candidate could not extend the accepted record".into());
    }
    let next = record::read_accepted(&extended)
        .map_err(|_| "the installed binary cannot read the candidate's extended record")?;
    if next.tags.len() != accepted.tags.len() + 1 || !next.tags.iter().any(|tag| tag == "elm") {
        return Err("the candidate did not retain and extend the accepted tags".into());
    }
    Ok("candidate seeded, rejected an invalid procedure without touching the accepted record, and extended it".into())
}
