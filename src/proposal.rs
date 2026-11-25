//! The Factory proposes its next job. A person releases it.
//!
//! One worker call writes a brief and a gate from the continuation of a decision record. Both wait
//! in STATE/proposals until `approve` queues them. The worker that later writes the candidate
//! sees the brief and never the gate.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::progress::Progress;
use crate::{Result, record, service, source};

const CHECK_TIMEOUT: Duration = Duration::from_secs(10);
const CONTRACT: &str = "You derive the next job of the TAIK Factory from a decision record. Write a brief and an independent gate. Another worker will later see only the brief and write the candidate. It never sees the gate.

Reply with exactly two files in this form and nothing else:

=== brief.md ===
<brief>
=== verify.mjs ===
<gate>

Brief: Markdown in German, at most 6000 bytes. Its first line is one sentence that names the intent, without Markdown markup. State everything the candidate author needs: the exact output format, every name, value and rule that the gate checks. Do not mention the gate's code.

Candidate format: one text document of at most 96 KiB. Either a standalone HTML document that starts with <!doctype html>, or a bundle of source files, each introduced by a line '=== path ===', when the job is a program.

Gate: a Node.js ES module of at most 12000 bytes. It reads the candidate from process.argv[2], uses node:assert/strict, and exits with a failed assertion that carries a clear message when a requirement is not met. On success it prints one line that starts with 'PASS: '. It runs in a sandbox without network. It may write only inside its working directory and has 15 seconds. It may unpack a bundle there and compile and run it. rustc is at process.env.HOME + '/.cargo/bin/rustc'. No other compiler is guaranteed.

Rules for the gate: check behaviour and results, not the presence of words. Every requirement must be satisfiable together with all others. Never require what the brief does not state. Keep the scope of the job as small as the record's continuation justifies.";

pub fn propose(state: &Path, name: &str, record_path: &Path) -> Result<PathBuf> {
    let name = crate::worker_name(name).ok_or("invalid proposal name")?;
    let state = state.canonicalize()?;
    let target = state.join("proposals").join(name);
    if target.try_exists()?
        || state
            .join("jobs")
            .join(format!("job-{name}"))
            .try_exists()?
    {
        return Err("a proposal or job with this name exists".into());
    }
    let record_path = record_path.canonicalize()?;
    let decision = record::read_regular(&record_path, record::MAX_RECORD_BYTES)?;
    if !decision.ends_with("end\n")
        || !decision
            .lines()
            .any(|line| line.starts_with("continuation="))
    {
        return Err("the record has no continuation to derive a job from".into());
    }
    let job = record_path.parent().ok_or("record without a job folder")?;
    let origin_brief =
        record::read_regular(&job.join("submission.brief.md"), record::MAX_RECORD_BYTES)
            .unwrap_or_default();
    let prompt = format!(
        "{CONTRACT}\n\nDECISION RECORD ({}):\n{decision}\nBRIEF OF THAT JOB:\n{}",
        record_path.display(),
        if origin_brief.is_empty() {
            "(none)"
        } else {
            &origin_brief
        },
    );
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let workspace = scratch.join("worker");
    fs::create_dir(&workspace)?;
    let (answer, worker) = source::ask(&workspace, job, &prompt, &mut Progress::silent());
    let answer = answer?;
    let (brief, gate) =
        split(&answer).ok_or("the worker did not return brief.md and verify.mjs")?;
    source::intent(&brief)?;
    if brief.len() > record::MAX_RECORD_BYTES
        || gate.trim().is_empty()
        || gate.len() > source::LIMIT
    {
        return Err("the proposed brief or gate exceeds its size limit".into());
    }
    let check = scratch.join("check");
    fs::create_dir(&check)?;
    fs::write(check.join("verify.mjs"), &gate)?;
    let syntax = source::run_child(
        source::sandbox(&check, job, "node", false)?.args(["--check", "verify.mjs"]),
        CHECK_TIMEOUT,
        |_| {},
    )?;
    if !syntax.status.success() {
        return Err("the proposed gate is not valid JavaScript".into());
    }
    fs::create_dir_all(&target)?;
    record::publish_text(&target.join("brief.md"), &brief, record::MAX_RECORD_BYTES)?;
    record::publish_text(&target.join("verify.mjs"), &gate, source::LIMIT)?;
    record::publish_text(
        &target.join("origin.txt"),
        &format!(
            "derived_from={}\nproposed_by={worker}\nstate=waiting for approval\n",
            record_path.display()
        ),
        record::MAX_RECORD_BYTES,
    )?;
    Ok(target)
}

pub fn approve(state: &Path, name: &str) -> Result<PathBuf> {
    let name = crate::worker_name(name).ok_or("invalid proposal name")?;
    let proposal = state.canonicalize()?.join("proposals").join(name);
    let job = service::queue_source(
        state,
        name,
        &proposal.join("brief.md"),
        &proposal.join("verify.mjs"),
    )?;
    record::publish_text(
        &proposal.join("approved.txt"),
        &format!("job={}\n", job.display()),
        record::MAX_RECORD_BYTES,
    )?;
    Ok(job)
}

fn split(answer: &str) -> Option<(String, String)> {
    let rest = answer.trim_start().strip_prefix("=== brief.md ===\n")?;
    let (brief, gate) = rest.split_once("\n=== verify.mjs ===\n")?;
    Some((format!("{}\n", brief.trim()), format!("{}\n", gate.trim())))
}
