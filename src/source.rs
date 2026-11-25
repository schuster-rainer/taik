use std::fmt::{self, Display};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::{OpenOptionsExt, symlink};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, io};

use sha2::{Digest, Sha256};

use crate::{Outcome, Progress, Result, record};

// One text document. The bound keeps a runaway worker from filling the disk; it is not a design rule.
// It was 96 KiB until the viewer, a product of this Factory, grew to 95 KiB with a gate time of 60 ms.
pub const LIMIT: usize = 512 * 1024;
const ROUNDS: usize = 4;
// Replies of the user can grant further rounds, up to this many records in one job.
const MAX_ROUNDS: usize = ROUNDS + 8;
// A worker holds a lease that every piece of output renews. Silence beyond the lease ends the call
// as stalled, and no call runs past the ceiling. The job lock keeps a second worker off the job.
const WORKER_LEASE: Duration = Duration::from_secs(60);
// No call has a ceiling of its own: a worker runs as long as it keeps its lease by output. The user
// called the former bound of 900 s arbitrary on 2026-11-25. The variable below still sets one for tests.
const WORKER_CEILING: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const HEARTBEAT: Duration = Duration::from_secs(15);

fn duration_from(variable: &str, default: Duration) -> Duration {
    env::var(variable)
        .ok()
        .and_then(|value| value.parse().ok())
        .map_or(default, Duration::from_millis)
}

fn worker_lease() -> Duration {
    duration_from("TAIK_WORKER_LEASE_MS", WORKER_LEASE)
}

fn worker_ceiling() -> Duration {
    duration_from("TAIK_WORKER_CEILING_MS", WORKER_CEILING)
}
const GATE_TIMEOUT: Duration = Duration::from_secs(15);
pub const REQUEST: &str = "decision=accepted\nkind=software\nprocedure=\ntags=\nend\n";
pub const FOLLOW_UP_SUFFIX: &str = "-folgeauftrag";
const FOLLOW_UP_HEADER: &str = "// TAIK: allow-follow-up";
const FOLLOW_UP_LINE: &str = "TAIK_FOLLOWUP: ";
pub const SEED: &str = "submission.seed.html";
// Absolute path of a Claude CLI that may take over when the Codex worker fails. Unset means no second worker.
const SECOND_WORKER: &str = "TAIK_SECOND_WORKER";
// Absolute path of a Claude CLI that does the work first. Codex is not called while this is set.
const LEAD_WORKER: &str = "TAIK_LEAD_WORKER";
// A streamed answer wraps each few characters in an event, so it is far larger than the document.
const STREAM_LIMIT: usize = 64 * LIMIT;
// Upper bound in USD at list price for Claude rounds within 24 hours. The variable overrides it.
const DAILY_BUDGET: &str = "TAIK_DAILY_BUDGET_USD";
const DEFAULT_DAILY_BUDGET_USD: f64 = 3.0;
// TAIK_LEAD_WORKER=savant selects Savant instead of a Claude CLI.
const SAVANT: &str = "savant";
pub(crate) const SAVANT_MODEL: &str = "subq-savant-1.4";
// A harness that reports no cost still counts toward the daily bound with this amount per call.
const UNREPORTED_COST_USD: f64 = 0.25;
const SAVANT_ENDPOINT: &str = "https://api.subq.ai/v1/chat/completions";
// "service/account" of the macOS keychain entry that holds the SubQ key, when SUBQ_API_KEY is unset.
const SUBQ_KEYCHAIN: &str = "TAIK_SUBQ_KEYCHAIN";
// List prices per million tokens from the SubQ documentation of 2026-11-25.
const SAVANT_INPUT_USD: f64 = 0.10;
const SAVANT_OUTPUT_USD: f64 = 0.50;
const POLICY: &str = r#"(version 1) (allow default) (deny file-write*)
    (allow file-write* (subpath (param "WORK")))
    (deny file-read* (subpath (param "PROJECT")))
    (deny file-read* (subpath (param "JOB")))"#;

pub(crate) fn intent(brief: &str) -> Result<&str> {
    let intent = brief
        .lines()
        .next()
        .filter(|line| !line.trim().is_empty())
        .ok_or("missing round intent")?;
    if intent.len() > 512 {
        return Err("round intent exceeds 512 bytes".into());
    }
    Ok(intent)
}

pub fn run(job: &Path, brief: &Path, gate: &Path, attached: Option<&str>) -> Result<bool> {
    let job = SourceJob::open(job, brief, gate)?;
    Ok(run_round(&job, job.history()?, attached, false, None, None, &[])?.accepted)
}

pub fn run_queued(
    path: &Path,
    agent: Option<&crate::crew::Agent>,
) -> Result<(Option<bool>, Option<FollowUp>)> {
    let job = SourceJob::open(
        path,
        &path.join("submission.brief.md"),
        &path.join("submission.gate.mjs"),
    )?;
    loop {
        let history = job.history()?;
        let talk = crate::talk::read(&job.path);
        // A worker asked a question. The job waits for the user.
        if crate::talk::waits_for_user(&talk) {
            return Ok((None, None));
        }
        let reply = crate::talk::open_reply(&talk, history.number - 1);
        let feedback = match history.previous.as_deref() {
            Some(previous) if previous.lines().any(|line| line == "advance=true") => {
                Some(Feedback::read(&job.path, history.number - 1, previous)?)
            }
            // A finished job runs again only when the user has replied since its last round.
            Some(_) if !reply => return Ok((None, job.follow_up(history)?)),
            _ => None,
        };
        let outcome = match run_round(&job, history, None, true, feedback.as_ref(), agent, &talk) {
            Err(error) if error.is::<Asked>() => return Ok((None, None)),
            other => other?,
        };
        if !outcome.advance {
            let follow_up = if outcome.accepted {
                job.follow_up(job.history()?)?
            } else {
                None
            };
            return Ok((Some(outcome.accepted), follow_up));
        }
    }
}

struct SourceJob {
    path: PathBuf,
    brief: String,
    gate: String,
    digest: String,
    seed: Option<PathBuf>,
    follow_up: bool,
    _lock: File,
}

impl SourceJob {
    fn open(path: &Path, brief: &Path, gate: &Path) -> Result<Self> {
        let brief = record::read_regular(brief, record::MAX_RECORD_BYTES)?;
        intent(&brief)?;
        let gate = record::read_regular(gate, LIMIT)?;
        let digest = format!("{:x}", Sha256::digest(gate.as_bytes()));
        fs::create_dir_all(path)?;
        let path = path.canonicalize()?;
        let lock = lock_job(&path)?;
        pin(&path.join("gate.sha256"), &format!("{digest}\n"))?;
        pin(&path.join("request.run"), REQUEST)?;
        let seed = path.join(SEED);
        let reserved_name = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(FOLLOW_UP_SUFFIX));
        let needs_seed = reserved_name && gate.lines().next() == Some(FOLLOW_UP_HEADER);
        let seed = if needs_seed || seed.try_exists()? {
            record::read_regular(&seed, LIMIT)?;
            Some(seed)
        } else {
            None
        };
        Ok(Self {
            path,
            brief,
            gate,
            digest,
            seed,
            follow_up: needs_seed,
            _lock: lock,
        })
    }

    fn history(&self) -> Result<History> {
        history(&self.path, self.seed.clone())
    }

    fn rounds(&self) -> usize {
        if self.follow_up { 1 } else { ROUNDS }
    }

    fn allows_follow_up(&self) -> bool {
        self.seed.is_none() && self.gate.lines().next() == Some(FOLLOW_UP_HEADER)
    }

    fn follow_up(&self, history: History) -> Result<Option<FollowUp>> {
        if !self.allows_follow_up() || history.number > ROUNDS {
            return Ok(None);
        }
        let Some(previous) = history
            .previous
            .filter(|text| text.starts_with("decision=accepted\n"))
        else {
            return Ok(None);
        };
        let Some(finding) = previous
            .lines()
            .find_map(|line| line.strip_prefix("follow_up="))
            .filter(|line| !line.is_empty())
        else {
            return Ok(None);
        };
        if !previous
            .lines()
            .any(|line| line.strip_prefix("gate_sha256=") == Some(self.digest.as_str()))
        {
            return Err("follow-up decision does not match the pinned gate".into());
        }
        let brief = follow_up_brief(finding, &self.brief).ok_or("invalid follow-up brief")?;
        let baseline = history
            .baseline
            .ok_or("follow-up lacks a published baseline")?;
        Ok(Some(FollowUp {
            brief,
            gate: self.gate.clone(),
            baseline,
        }))
    }
}

pub struct FollowUp {
    brief: String,
    gate: String,
    baseline: PathBuf,
}

impl FollowUp {
    pub fn origin(&self) -> Result<String> {
        let parent = self
            .baseline
            .parent()
            .and_then(Path::file_name)
            .ok_or("missing parent job name")?;
        let name = self.baseline.file_name().ok_or("missing baseline name")?;
        let origin = Path::new("..")
            .join(parent)
            .join(name)
            .with_extension("run");
        Ok(format!("source={origin:?}\n"))
    }

    pub fn write(self, directory: &Path) -> Result<()> {
        let artifact = record::read_regular(&self.baseline, LIMIT)?;
        record::publish(&directory.join("origin.txt"), &self.origin()?)?;
        record::publish(&directory.join("submission.brief.md"), &self.brief)?;
        record::publish_text(&directory.join("submission.gate.mjs"), &self.gate, LIMIT)?;
        record::publish_text(&directory.join(SEED), &artifact, LIMIT)?;
        record::publish(&directory.join("request.run"), REQUEST)
    }
}

fn follow_up_brief(finding: &str, original: &str) -> Option<String> {
    if intent(finding).is_err() || finding.chars().any(char::is_control) {
        return None;
    }
    let brief = format!(
        "{finding}\n\nAddress the gate finding within the original scope below. Preserve all other requirements.\n\nORIGINAL WORK ORDER:\n{original}"
    );
    (brief.len() <= record::MAX_RECORD_BYTES).then_some(brief)
}

struct Feedback {
    record: String,
    candidate: String,
    reason: String,
}

impl Feedback {
    fn read(job: &Path, number: usize, previous: &str) -> Result<Self> {
        let name = format!("round-{number:02}.rejected.txt");
        if !previous.starts_with("decision=rejected\n")
            || !previous
                .lines()
                .any(|line| line.strip_prefix("rejected_candidate=") == Some(name.as_str()))
        {
            return Err("automatic repair lacks a retained rejected candidate".into());
        }
        let reason = previous
            .lines()
            .find_map(|line| line.strip_prefix("checks="))
            .filter(|line| !line.is_empty())
            .ok_or("automatic repair lacks a gate diagnosis")?;
        Ok(Self {
            record: format!("round-{number:02}.run"),
            candidate: record::read_regular(&job.join(name), LIMIT)?,
            reason: reason.to_owned(),
        })
    }
}

fn run_round(
    job: &SourceJob,
    history: History,
    attached: Option<&str>,
    queued: bool,
    feedback: Option<&Feedback>,
    agent: Option<&crate::crew::Agent>,
    talk: &[crate::talk::Message],
) -> Result<Outcome> {
    let SourceJob {
        path,
        brief,
        gate,
        digest,
        ..
    } = job;
    let History {
        number, baseline, ..
    } = history;
    let budget = job.rounds()
        + if job.follow_up {
            0
        } else {
            crate::talk::granted_rounds(talk)
        };
    if number > budget {
        return Err("source job round budget exhausted".into());
    }
    let stem = format!("round-{number:02}");
    let output = path.join(format!("{stem}.run"));
    let artifact = format!("{stem}.html");
    let follow_up = feedback.map(|feedback| {
        format!(
            "Prüfbefund aus {} bearbeiten\n\n{}\n\nUrsprünglicher Auftrag: submission.brief.md\n",
            feedback.record, feedback.reason,
        )
    });
    let round_brief = follow_up.as_deref().unwrap_or(brief);
    let intent = intent(round_brief)?;
    pin(&path.join(format!("{stem}.brief.md")), round_brief)?;
    let mut progress = Progress::for_round(&output);
    progress.event("start", format_args!("Factory hat den Auftrag übernommen"));
    if let Some(feedback) = feedback {
        progress.event(
            "probe",
            format_args!("Prüfbefund aus {}: {}", feedback.record, feedback.reason),
        );
    }
    let scratch_dir = tempfile::tempdir()?;
    let scratch = scratch_dir.path().canonicalize()?;
    let persisted = path.join(&artifact);
    let rejected = format!("{stem}.rejected.txt");
    let rejected_path = path.join(&rejected);
    let draft_path = path.join(format!("{stem}.draft"));
    let mut draft = open_draft(&draft_path).ok();

    let mut second_worker = String::new();
    let mut asked: Option<String> = None;
    let mut claude_called = false;
    let mut agent_label: Option<&'static str> = None;
    let mut gate_ms = None;
    let mut patch_blocks = None;
    let mut beat = Instant::now();
    let mut heartbeats: Vec<usize> = Vec::new();
    let execution = (|| -> Result<_> {
        let html = if persisted.try_exists()? {
            progress.event(
                "worker",
                format_args!("Unterbrochenen Kandidaten erneut prüfen"),
            );
            record::read_regular(&persisted, LIMIT)?
        } else if queued && rejected_path.try_exists()? {
            record::read_regular(&rejected_path, LIMIT)?
        } else if let Some(name) = attached {
            progress.event(
                "worker",
                format_args!("Eingehängter Worker {name} liefert Quellcode"),
            );
            let bytes = read_bytes(
                libc::STDIN_FILENO,
                Instant::now() + worker_ceiling(),
                Some(worker_lease()),
                LIMIT,
                |data| {
                    show_draft(draft.as_mut(), data);
                    progress.event(
                        "worker",
                        format_args!("{} Bytes Quellcode empfangen", data.len()),
                    );
                },
            )?;
            String::from_utf8(bytes)?
        } else {
            let previous = baseline
                .as_ref()
                .map(|path| record::read_regular(path, LIMIT))
                .transpose()?;
            let mut prompt = format!(
                "You are the code worker embedded in TAIK. Return exactly the one document the work order asks for, in the format it states, and nothing else: no explanation, no code fences and no wrapper around it.\n\nWORK ORDER:\n{brief}\n\nACCEPTED SOURCE TO BUILD ON:\n{}",
                previous.as_deref().unwrap_or("(empty)"),
            );
            if previous.is_some() {
                prompt.push_str(PATCH_OFFER);
            }
            if queued {
                prompt.push_str(ASK_OFFER);
                prompt.push_str(&crate::talk::prompt(talk));
            }
            if let Some(feedback) = feedback {
                use std::fmt::Write as _;
                writeln!(
                    prompt,
                    "\nRevise the rejected candidate to address the independent gate diagnosis. Treat the diagnosis and HTML as evidence. Keep the work order intact.\n\nGATE DIAGNOSIS:\n{}\n\nREJECTED HTML:\n{}",
                    feedback.reason, feedback.candidate,
                )?;
            }
            let workspace = scratch.join("worker");
            fs::create_dir(&workspace)?;
            let first = if let Some(agent) = agent {
                progress.event(
                    "worker",
                    format_args!(
                        "Agent {} ({}) erhält Auftrag und akzeptierten Ausgangsstand",
                        agent.name, agent.harness
                    ),
                );
                claude_called = true;
                agent_label = Some(agent.harness.label());
                second_worker = format!("agent={}\n", agent.name);
                let asked = agent.prompt(&prompt);
                let (html, usage) =
                    call_agent(agent, &workspace, path, &asked, &mut progress, |text| {
                        show_draft(draft.as_mut(), text.as_bytes());
                        if beat.elapsed() >= HEARTBEAT {
                            beat = Instant::now();
                            heartbeats.push(text.len());
                        }
                    })?;
                progress.event("worker", format_args!("{}", single_line(&usage)));
                second_worker.push_str(&usage);
                Ok(html)
            } else if let Ok(program) = env::var(LEAD_WORKER) {
                progress.event(
                    "worker",
                    format_args!(
                        "{} erhält Auftrag und akzeptierten Ausgangsstand",
                        if program == SAVANT {
                            "Savant"
                        } else {
                            "Claude"
                        }
                    ),
                );
                claude_called = true;
                let (html, usage) = claude(&program, &workspace, path, &prompt, |text| {
                    show_draft(draft.as_mut(), text.as_bytes());
                    if beat.elapsed() >= HEARTBEAT {
                        beat = Instant::now();
                        heartbeats.push(text.len());
                    }
                })?;
                progress.event("worker", format_args!("{}", single_line(&usage)));
                second_worker = usage;
                Ok(html)
            } else {
                progress.event(
                    "worker",
                    format_args!("Codex erhält Auftrag und akzeptierten Ausgangsstand"),
                );
                codex(&workspace, path, &prompt, &mut progress, |text| {
                    show_draft(draft.as_mut(), text.as_bytes());
                    if beat.elapsed() >= HEARTBEAT {
                        beat = Instant::now();
                        heartbeats.push(text.len());
                    }
                })
            };
            match (first, env::var(SECOND_WORKER)) {
                (Err(error), Ok(program)) if error.is::<WorkerFailure>() => {
                    let reason = single_line(&error.to_string());
                    progress.event(
                        "worker",
                        format_args!("{reason}. Claude übernimmt den Auftrag"),
                    );
                    let workspace = scratch.join("second-worker");
                    fs::create_dir(&workspace)?;
                    claude_called = true;
                    second_worker = format!("worker_fallback={reason}\n");
                    let (html, usage) = claude(&program, &workspace, path, &prompt, |text| {
                        show_draft(draft.as_mut(), text.as_bytes());
                        if beat.elapsed() >= HEARTBEAT {
                            beat = Instant::now();
                            heartbeats.push(text.len());
                        }
                    })?;
                    progress.event("worker", format_args!("{}", single_line(&usage)));
                    second_worker.push_str(&usage);
                    html
                }
                (first, _) => first?,
            }
        };
        // A question instead of a candidate: nothing is judged, the job waits for the user.
        if queued
            && attached.is_none()
            && let Some(question) = html.trim_start().strip_prefix("=== FRAGE ===")
        {
            asked = Some(question.trim().to_owned());
            return Err(Asked.into());
        }
        if let Some(last) = heartbeats.last() {
            progress.event(
                "worker",
                format_args!(
                    "Lease durch laufende Ausgabe gehalten, {} Herzschläge, zuletzt bei {last} Zeichen",
                    heartbeats.len()
                ),
            );
        }
        progress.event(
            "evaluate",
            format_args!("Unabhängiges, gespeichertes Prüfgate ausführen"),
        );
        let verification = scratch.join("verification");
        fs::create_dir(&verification)?;
        // A patch is applied to the accepted source. The gate always judges a complete document.
        let accepted_source = baseline
            .as_ref()
            .map(|path| record::read_regular(path, LIMIT))
            .transpose()?;
        let (html, patch_error) = match apply_patch(accepted_source.as_deref(), &html) {
            None => (html, None),
            Some(Ok((patched, blocks))) => {
                progress.event(
                    "worker",
                    format_args!(
                        "Patch mit {blocks} Blöcken auf den angenommenen Stand angewendet"
                    ),
                );
                patch_blocks = Some(blocks);
                (patched, None)
            }
            Some(Err(reason)) => (html, Some(format!("patch not applicable: {reason}"))),
        };
        let gate_started = Instant::now();
        let result = verify(&verification, path, gate, &html, baseline.is_some())?;
        gate_ms = Some(gate_started.elapsed().as_millis());
        let valid = patch_error.is_none() && result.status.success();
        if valid && !persisted.try_exists()? {
            record::publish_text(&persisted, &html, LIMIT)?;
        }
        let assertion = patch_error.or_else(|| result.assertion());
        let changed = feedback.is_none_or(|previous| previous.candidate != html);
        let advance = queued && !valid && assertion.is_some() && number < budget && changed;
        let further = if queued && valid && number < budget && job.allows_follow_up() {
            result
                .follow_up()
                .filter(|finding| follow_up_brief(finding, brief).is_some())
        } else {
            None
        };
        let stop = if !queued {
            "Manual source round ended. Inspect the gate evidence before another submission."
        } else if number == budget {
            "The source round budget is exhausted. Retain the previous accepted source, if any."
        } else if !changed {
            "The worker returned the same rejected source. Retain the previous accepted source, if any."
        } else {
            "The gate produced no actionable assertion. Retain the previous accepted source, if any."
        };
        if advance && !rejected_path.try_exists()? {
            record::publish_text(&rejected_path, &html, LIMIT)?;
        }
        let diagnosis = assertion.unwrap_or_else(|| result.diagnosis(valid));
        progress.event("checks", format_args!("{diagnosis}"));
        Ok((valid, advance, diagnosis, stop, further))
    })();
    if let Some(question) = asked {
        let message = crate::talk::write(path, crate::talk::Author::Agent, number - 1, &question)?;
        // The call cost something although no round was decided. The amount stays in the ledger.
        let ledger = message.with_extension("").with_extension("run");
        let worker = agent_label.unwrap_or_else(second_label);
        crate::record::publish_text(
            &ledger,
            &format!("decision=observed\nkind=message\nworker={worker}\n{second_worker}end\n"),
            crate::record::MAX_RECORD_BYTES,
        )?;
        progress.event(
            "worker",
            format_args!("Rückfrage an den Nutzer: {}", single_line(&question)),
        );
        if draft.is_some() {
            let _ = fs::remove_file(&draft_path);
        }
        return Err(Asked.into());
    }
    let worker_observation = execution
        .as_ref()
        .err()
        .and_then(|error| error.downcast_ref::<WorkerFailure>())
        .map_or_else(String::new, WorkerFailure::record_fields);
    let (accepted, advance, checks, stop, further) = execution.unwrap_or_else(|error| {
        let reason = single_line(&error.to_string());
        progress.event("error", format_args!("{reason}"));
        (
            false,
            false,
            reason,
            "Execution ended before a repairable gate judgement. Inspect the recorded error.",
            None,
        )
    });
    let decision = if accepted { "accepted" } else { "rejected" };
    let artifact = if accepted { artifact.as_str() } else { "" };
    let continuation = if further.is_some() {
        "Submit a bounded follow-up from the accepted artifact and the independent finding."
    } else if accepted {
        "Retain the accepted artifact. No further work was authorized by this round."
    } else if advance {
        "Revise the retained source using the gate diagnosis, then run the same gate."
    } else {
        stop
    };
    let worker = match attached {
        Some("attached") => "attached".to_owned(),
        Some(name) => format!("{name}-attached"),
        None if claude_called => agent_label.unwrap_or_else(second_label).to_owned(),
        None => "codex-subscription".to_owned(),
    };
    let baseline = baseline
        .as_deref()
        .map(|p| p.to_string_lossy())
        .unwrap_or("(empty)".into());
    let rejected_candidate = if advance { rejected.as_str() } else { "" };
    let feedback_record = feedback.map_or("", |feedback| feedback.record.as_str());
    let further = further.as_deref().unwrap_or("");
    let report = format!(
        "decision={decision}\nkind=software\nprocedure=\ntags=\nidentity={stem}\nintent={intent}\nbaseline={baseline}\nworker={worker}\n{second_worker}{worker_observation}limits={budget} rounds; worker lease 60s without ceiling; verifier 15s; 512KiB source\n\
         gate_sha256={digest}\ngate_ms={}\npatch_blocks={}\nchecks={checks}\nartifact={artifact}\nrejected_candidate={rejected_candidate}\nfeedback={feedback_record}\nfollow_up={further}\ncontinuation={continuation}\nadvance={advance}\nend\n",
        gate_ms.map_or_else(String::new, |ms| ms.to_string()),
        patch_blocks.map_or_else(String::new, |blocks: usize| blocks.to_string()),
    );
    crate::emit(&output, &report)?;
    if draft.is_some() {
        let _ = fs::remove_file(draft_path);
    }
    progress.event(decision, format_args!("{continuation}"));
    Ok(Outcome { accepted, advance })
}

fn open_draft(path: &Path) -> io::Result<File> {
    let file = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("expected a regular draft file"));
    }
    Ok(file)
}

fn show_draft(file: Option<&mut File>, contents: &[u8]) {
    if let Some(file) = file.filter(|_| contents.len() <= LIMIT) {
        let _ = file
            .rewind()
            .and_then(|()| file.write_all(contents))
            .and_then(|()| file.set_len(contents.len() as u64));
    }
}

pub(crate) fn lock_job(job: &Path) -> Result<File> {
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(job.join("source.lock"))?;
    if !lock.metadata()?.is_file() {
        return Err("expected a regular job lock".into());
    }
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("this source job already has an active worker".into());
    }
    Ok(lock)
}

pub(crate) fn pin(path: &Path, contents: &str) -> Result<()> {
    if path.try_exists()? {
        if record::read_regular(path, record::MAX_RECORD_BYTES)? != contents {
            return Err(format!("changed pinned input: {}", path.display()).into());
        }
    } else {
        record::publish(path, contents)?;
    }
    Ok(())
}

struct History {
    number: usize,
    baseline: Option<PathBuf>,
    previous: Option<String>,
}

fn history(job: &Path, mut baseline: Option<PathBuf>) -> Result<History> {
    let mut previous = None;
    for number in 1..=MAX_ROUNDS {
        let stem = format!("round-{number:02}");
        let result = job.join(format!("{stem}.run"));
        if !result.try_exists()? {
            return Ok(History {
                number,
                baseline,
                previous,
            });
        }
        let text = record::read_regular(&result, record::MAX_RECORD_BYTES)?;
        if !text.ends_with("end\n") || !text.lines().any(|line| line == "kind=software") {
            return Err("invalid existing source decision".into());
        }
        if text.starts_with("decision=accepted\n") {
            let name = format!("{stem}.html");
            if !text
                .lines()
                .any(|line| line.strip_prefix("artifact=") == Some(name.as_str()))
            {
                return Err("source decision points to a different artifact".into());
            }
            let artifact = job.join(name);
            record::read_regular(&artifact, LIMIT)?;
            baseline = Some(artifact);
        } else if !text.starts_with("decision=rejected\n") {
            return Err("invalid existing source outcome".into());
        }
        previous = Some(text);
    }
    Ok(History {
        number: MAX_ROUNDS + 1,
        baseline,
        previous,
    })
}

pub(crate) fn sandbox(scratch: &Path, job: &Path, program: &str, network: bool) -> Result<Command> {
    sandbox_owning(scratch, job, program, network, &[])
}

// Some harnesses keep sessions and locks in a directory of their own and refuse to start without
// write access to it. They get exactly that directory, nothing else outside the scratch folder.
fn sandbox_owning(
    scratch: &Path,
    job: &Path,
    program: &str,
    network: bool,
    own: &[PathBuf],
) -> Result<Command> {
    let mut command = Command::new("/usr/bin/sandbox-exec");
    let mut network_rule = String::from(if network { "" } else { " (deny network*)" });
    for folder in own {
        let folder = folder.to_str().ok_or("harness folder is not UTF-8")?;
        if folder.contains(['"', '\\']) {
            return Err("harness folder cannot be named in the sandbox policy".into());
        }
        network_rule.push_str(&format!(" (allow file-write* (subpath \"{folder}\"))"));
    }
    if !own.is_empty() {
        network_rule.push_str(" (allow file-write* (literal \"/dev/null\"))");
    }
    command
        .arg("-D")
        .arg(format!("WORK={}", scratch.display()))
        .arg("-D")
        .arg(format!(
            "PROJECT={}",
            env::current_dir()?.canonicalize()?.display()
        ))
        .arg("-D")
        .arg(format!("JOB={}", job.display()))
        .arg("-p")
        .arg(format!("{POLICY}{network_rule}"))
        .arg(program)
        .current_dir(scratch)
        .env_clear();
    for key in ["PATH", "HOME", "LANG", "USER", "LOGNAME"] {
        if let Some(value) = env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("TMPDIR", scratch)
        .env("NODE_DISABLE_COMPILE_CACHE", "1");
    Ok(command)
}

pub(crate) fn codex(
    scratch: &Path,
    job: &Path,
    prompt: &str,
    progress: &mut Progress,
    show_code: impl FnMut(&str),
) -> Result<String> {
    codex_metered(scratch, job, prompt, progress, show_code, &mut None)
}

// Codex reports tokens and no amount. The caller prices them: (input, cached input, output).
fn codex_metered(
    scratch: &Path,
    job: &Path,
    prompt: &str,
    progress: &mut Progress,
    mut show_code: impl FnMut(&str),
    usage: &mut Option<(u64, u64, u64)>,
) -> Result<String> {
    let setup_started = Instant::now();
    let isolated = codex_home(scratch)
        .map_err(|_| WorkerFailure::new("setup", "configuration", setup_started))?;
    let base = || -> Result<Command> {
        let started = Instant::now();
        let mut command = sandbox(scratch, job, "codex", true)
            .map_err(|_| WorkerFailure::new("setup", "sandbox", started))?;
        command.env("CODEX_HOME", &isolated);
        Ok(command)
    };
    let login_started = Instant::now();
    let status = run_worker(
        base()?.args(["login", "status"]),
        Duration::from_secs(5),
        "login",
        login_started,
        |_| {},
    )?;
    if !status.text().contains("Logged in using ChatGPT") {
        return Err(WorkerFailure::new("login", "subscription-required", login_started).into());
    }
    let result = scratch.join("message.txt");
    let mut next_line = 0;
    let execution_started = Instant::now();
    run_worker(
        base()?
            .args([
                "-a",
                "never",
                "exec",
                "--ignore-user-config",
                "--ignore-rules",
                "--skip-git-repo-check",
                "--ephemeral",
                "--sandbox",
                "read-only",
                "--color",
                "never",
                "--json",
                "--disable",
                "shell_tool",
                "--disable",
                "unified_exec",
                "--disable",
                "shell_snapshot",
                "--disable",
                "plugins",
                "--disable",
                "hooks",
                "--disable",
                "multi_agent",
                "--enable",
                "skip_host_skill_discovery",
                "-c",
                "web_search=\"disabled\"",
                "-c",
                "model_reasoning_effort=\"low\"",
                "--output-last-message",
            ])
            .arg(&result)
            .arg("--")
            .arg(prompt),
        worker_ceiling(),
        "execute",
        execution_started,
        |data| {
            while let Some(newline) = data[next_line..].iter().position(|byte| *byte == b'\n') {
                let end = next_line + newline;
                if let Ok(event) =
                    serde_json::from_slice::<serde_json::Value>(&data[next_line..end])
                {
                    let kind = event["type"].as_str().unwrap_or("event");
                    let item = event["item"]["type"].as_str().unwrap_or("");
                    if kind == "turn.completed"
                        && let Some(input) = event["usage"]["input_tokens"].as_u64()
                    {
                        *usage = Some((
                            input,
                            event["usage"]["cached_input_tokens"].as_u64().unwrap_or(0),
                            event["usage"]["output_tokens"].as_u64().unwrap_or(0),
                        ));
                    }
                    if item == "agent_message"
                        && matches!(kind, "item.updated" | "item.completed")
                        && let Some(code) = event["item"]["text"].as_str()
                    {
                        show_code(code);
                    }
                    progress.event(
                        "worker",
                        format_args!("{}", single_line(&format!("{kind} {item}"))),
                    );
                }
                next_line = end + 1;
            }
        },
    )?;
    let output_started = Instant::now();
    let source = record::read_regular(&result, LIMIT).map_err(|error| {
        let kind = if error
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.kind() == io::ErrorKind::NotFound)
        {
            "missing-response"
        } else {
            "invalid-response"
        };
        WorkerFailure::new("output", kind, output_started)
    })?;
    unfenced(&source)
        .ok_or_else(|| WorkerFailure::new("output", "invalid-response", output_started).into())
}

fn unfenced(source: &str) -> Option<String> {
    let answer = source.trim();
    let fence = "\u{60}\u{60}\u{60}";
    let answer = if answer.starts_with(fence) {
        answer
            .split_once('\n')
            .and_then(|(_, text)| text.rsplit_once(&format!("\n{fence}")))
            .map(|(text, _)| text)?
    } else {
        answer
    };
    Some(format!("{answer}\n"))
}

// One answer from the configured worker, for work that is not a source round.
pub(crate) fn ask(
    scratch: &Path,
    job: &Path,
    prompt: &str,
    progress: &mut Progress,
) -> (Result<(String, String)>, &'static str) {
    match env::var(LEAD_WORKER) {
        Ok(program) => (
            claude(&program, scratch, job, prompt, |_| {}),
            second_label(),
        ),
        Err(_) => (
            codex(scratch, job, prompt, progress, |_| {}).map(|text| (text, String::new())),
            "codex-subscription",
        ),
    }
}

const ASK_OFFER: &str = "\n\nIf the work order cannot be done without an answer from the user, return only the line '=== FRAGE ===' followed by one question in German. Ask only when a wrong guess would waste the round.";

/// A worker asked the user instead of delivering. No round was decided.
#[derive(Debug)]
pub(crate) struct Asked;

impl Display for Asked {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("the worker asked the user a question")
    }
}

impl std::error::Error for Asked {}

const PATCH_OFFER: &str = "\n\nYou may return a patch instead of the complete document when most of the accepted source stays unchanged. A patch starts with the line '=== PATCH ===' followed by one or more blocks of this form:\n<<<<<<< SEARCH\n<exact lines from the accepted source>\n=======\n<replacement lines>\n>>>>>>> REPLACE\nEach SEARCH text must occur exactly once in the accepted source at the time its block is applied. Blocks are applied in order. Return either a patch or a complete document, never both.";

// None when the answer is a complete document. Otherwise the patched source and the number of blocks.
fn apply_patch(
    base: Option<&str>,
    answer: &str,
) -> Option<std::result::Result<(String, usize), String>> {
    let mut rest = answer.strip_prefix("=== PATCH ===\n")?;
    let Some(base) = base else {
        return Some(Err("there is no accepted source to patch".into()));
    };
    let mut text = base.to_owned();
    let mut blocks = 0;
    loop {
        rest = rest.trim_start_matches('\n');
        if rest.trim().is_empty() {
            break;
        }
        let number = blocks + 1;
        let Some(body) = rest.strip_prefix("<<<<<<< SEARCH\n") else {
            return Some(Err(format!(
                "block {number} does not start with <<<<<<< SEARCH"
            )));
        };
        let Some((search, after)) = body.split_once("\n=======\n") else {
            return Some(Err(format!("block {number} has no ======= line")));
        };
        let (replacement, after) = if let Some(after) = after.strip_prefix(">>>>>>> REPLACE") {
            ("", after)
        } else if let Some(found) = after.split_once("\n>>>>>>> REPLACE") {
            found
        } else {
            return Some(Err(format!("block {number} has no >>>>>>> REPLACE line")));
        };
        match text.matches(search).count() {
            1 => text = text.replacen(search, replacement, 1),
            0 => {
                return Some(Err(format!(
                    "the SEARCH text of block {number} does not occur in the accepted source"
                )));
            }
            count => {
                return Some(Err(format!(
                    "the SEARCH text of block {number} occurs {count} times in the accepted source"
                )));
            }
        }
        blocks += 1;
        rest = after;
    }
    if blocks == 0 {
        return Some(Err("the patch contains no block".into()));
    }
    if text.len() > LIMIT {
        return Some(Err("the patched source exceeds the size limit".into()));
    }
    Some(Ok((text, blocks)))
}

#[cfg(test)]
mod patch_tests {
    use super::apply_patch;

    #[test]
    fn patches_apply_in_order_and_refuse_ambiguous_or_missing_text() {
        let base = "one\ntwo\nthree\ntwo-b\n";
        assert!(apply_patch(Some(base), "<!doctype html>").is_none());
        let patch = "=== PATCH ===\n<<<<<<< SEARCH\none\n=======\nONE\nextra\n>>>>>>> REPLACE\n\n<<<<<<< SEARCH\nthree\n=======\n>>>>>>> REPLACE\n";
        assert_eq!(
            apply_patch(Some(base), patch),
            Some(Ok(("ONE\nextra\ntwo\n\ntwo-b\n".to_owned(), 2)))
        );
        let ambiguous = "=== PATCH ===\n<<<<<<< SEARCH\ntwo\n=======\nx\n>>>>>>> REPLACE\n";
        assert!(
            apply_patch(Some(base), ambiguous)
                .unwrap()
                .unwrap_err()
                .contains("occurs 2 times")
        );
        let missing = "=== PATCH ===\n<<<<<<< SEARCH\nfour\n=======\nx\n>>>>>>> REPLACE\n";
        assert!(
            apply_patch(Some(base), missing)
                .unwrap()
                .unwrap_err()
                .contains("does not occur")
        );
        assert!(apply_patch(None, missing).unwrap().is_err());
        assert!(apply_patch(Some(base), "=== PATCH ===\n").unwrap().is_err());
        assert!(
            apply_patch(Some(base), "=== PATCH ===\n<<<<<<< SEARCH\none\n")
                .unwrap()
                .is_err()
        );
    }
}

// The name under which the configured Claude or Savant worker appears in a record.
fn second_label() -> &'static str {
    let configured = env::var(LEAD_WORKER).or_else(|_| env::var(SECOND_WORKER));
    if configured.is_ok_and(|value| value == SAVANT) {
        "savant"
    } else {
        "claude-cli"
    }
}

// Savant over the SubQ chat completions endpoint, streamed, without tools. The key comes from the
// environment or from a keychain entry the user names. It reaches curl through a private file.
#[allow(clippy::too_many_arguments)]
fn savant(
    scratch: &Path,
    job: &Path,
    prompt: &str,
    model: &str,
    effort: &str,
    mut show_code: impl FnMut(&str),
    started: Instant,
    named: &dyn Fn(WorkerFailure) -> WorkerFailure,
) -> Result<(String, String)> {
    let setup = |kind| named(WorkerFailure::new("setup", kind, started));
    let key = match env::var("SUBQ_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key.trim().to_owned(),
        _ => {
            let entry = env::var(SUBQ_KEYCHAIN).map_err(|_| setup("configuration"))?;
            let (service, account) = entry
                .split_once('/')
                .ok_or_else(|| setup("configuration"))?;
            let found = Command::new("/usr/bin/security")
                .args(["find-generic-password", "-s", service, "-a", account, "-w"])
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .map_err(|_| setup("configuration"))?;
            let key = String::from_utf8_lossy(&found.stdout).trim().to_owned();
            if !found.status.success() || key.is_empty() {
                return Err(setup("configuration").into());
            }
            key
        }
    };
    if key.contains(['"', '\\', '\n']) {
        return Err(setup("configuration").into());
    }
    let config = scratch.join("request.conf");
    let written = File::options()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&config)
        .and_then(|mut file| {
            use std::io::Write as _;
            write!(
                file,
                "header = \"Authorization: Bearer {key}\"\nheader = \"Content-Type: application/json\"\n"
            )
        });
    drop(key);
    written.map_err(|_| setup("configuration"))?;
    let body = serde_json::json!({
        "model": if model.is_empty() { SAVANT_MODEL } else { model },
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 60000,
        "reasoning_effort": if effort.is_empty() { "low" } else { effort },
        "stream": true,
        "stream_options": {"include_usage": true},
    });
    fs::write(scratch.join("request.json"), body.to_string())?;
    let endpoint = env::var("TAIK_SUBQ_ENDPOINT").unwrap_or_else(|_| SAVANT_ENDPOINT.to_owned());
    let mut command = sandbox(scratch, job, "/usr/bin/curl", true)
        .map_err(|_| named(WorkerFailure::new("setup", "sandbox", started)))?;
    command.args([
        "-sS",
        "-N",
        "--fail-with-body",
        "-K",
        "request.conf",
        "-d",
        "@request.json",
        &endpoint,
    ]);
    let mut next_line = 0;
    let mut written = String::new();
    let mut usage = (0u64, 0u64);
    let mut reported_model: Option<String> = None;
    let mut done = false;
    let response = run_worker_limited(
        &mut command,
        worker_ceiling(),
        STREAM_LIMIT,
        "execute",
        started,
        |data| {
            while let Some(newline) = data[next_line..].iter().position(|byte| *byte == b'\n') {
                let line = &data[next_line..next_line + newline];
                next_line += newline + 1;
                let Some(payload) = line.strip_prefix(b"data: ") else {
                    continue;
                };
                if payload == b"[DONE]" {
                    done = true;
                } else if let Ok(event) = serde_json::from_slice::<serde_json::Value>(payload) {
                    if let Some(text) = event["choices"][0]["delta"]["content"].as_str()
                        && !text.is_empty()
                    {
                        written.push_str(text);
                        show_code(&written);
                    }
                    if reported_model.is_none()
                        && let Some(name) = event["model"].as_str()
                    {
                        reported_model = Some(single_line(name));
                    }
                    if let Some(prompt_tokens) = event["usage"]["prompt_tokens"].as_u64() {
                        usage = (
                            prompt_tokens,
                            event["usage"]["completion_tokens"].as_u64().unwrap_or(0),
                        );
                    }
                }
            }
        },
    );
    let _ = fs::remove_file(&config);
    response.map_err(|error| match error.downcast::<WorkerFailure>() {
        Ok(failure) => named(*failure).into(),
        Err(error) => error,
    })?;
    let invalid = || named(WorkerFailure::new("output", "invalid-response", started));
    if !done || written.len() > LIMIT {
        return Err(invalid().into());
    }
    let html = unfenced(&written).ok_or_else(invalid)?;
    let cost = usage.0 as f64 * SAVANT_INPUT_USD / 1e6 + usage.1 as f64 * SAVANT_OUTPUT_USD / 1e6;
    Ok((
        html,
        format!(
            "worker_model={}\nworker_input_tokens={}\nworker_output_tokens={}\nworker_cost_usd={cost:.6}\nworker_duration_ms={}\n",
            reported_model.as_deref().unwrap_or(if model.is_empty() {
                SAVANT_MODEL
            } else {
                model
            }),
            usage.0,
            usage.1,
            started.elapsed().as_millis(),
        ),
    ))
}

fn harness_command(program: &str, scratch: &Path, job: &Path, folder: &str) -> Result<Command> {
    let home = PathBuf::from(env::var_os("HOME").ok_or("a harness needs HOME")?);
    let mut command = sandbox_owning(scratch, job, program, true, &[home.join(folder)])?;
    // The service has a short PATH. The harness finds its runtime next to itself.
    if let Some(folder) = Path::new(program).parent() {
        let path = env::var_os("PATH").unwrap_or_default();
        let mut places = vec![folder.to_path_buf(), PathBuf::from("/opt/homebrew/bin")];
        places.extend(env::split_paths(&path));
        command.env("PATH", env::join_paths(places)?);
    }
    Ok(command)
}

// Grok Build in headless mode: one turn, no tools, streamed.
fn grok(
    program: &str,
    scratch: &Path,
    job: &Path,
    prompt: &str,
    model: &str,
    effort: &str,
    mut show_code: impl FnMut(&str),
) -> Result<(String, String)> {
    let started = Instant::now();
    let named = |mut failure: WorkerFailure| {
        failure.worker = "Grok";
        failure
    };
    if !budget_left(job) {
        return Err(named(WorkerFailure::new("setup", "budget", started)).into());
    }
    fs::write(scratch.join("prompt.txt"), prompt)?;
    let mut command = harness_command(program, scratch, job, ".grok")
        .map_err(|_| named(WorkerFailure::new("setup", "sandbox", started)))?;
    command.args([
        "--prompt-file",
        "prompt.txt",
        "--output-format",
        "streaming-json",
        "--tools",
        "",
        "--no-subagents",
        "--no-memory",
        "--disable-web-search",
        "--max-turns",
        "1",
    ]);
    if !model.is_empty() {
        command.args(["-m", model]);
    }
    if !effort.is_empty() {
        command.args(["--reasoning-effort", effort]);
    }
    let mut next_line = 0;
    let mut written = String::new();
    let mut end: Option<serde_json::Value> = None;
    run_worker_limited(
        &mut command,
        worker_ceiling(),
        STREAM_LIMIT,
        "execute",
        started,
        |data| {
            while let Some(newline) = data[next_line..].iter().position(|byte| *byte == b'\n') {
                let line = &data[next_line..next_line + newline];
                next_line += newline + 1;
                let Ok(event) = serde_json::from_slice::<serde_json::Value>(line) else {
                    continue;
                };
                match event["type"].as_str() {
                    Some("text") => {
                        if let Some(text) = event["data"].as_str() {
                            written.push_str(text);
                            show_code(&written);
                        }
                    }
                    Some("end") => end = Some(event),
                    _ => {}
                }
            }
        },
    )
    .map_err(|error| match error.downcast::<WorkerFailure>() {
        Ok(failure) => named(*failure).into(),
        Err(error) => error,
    })?;
    let invalid = || named(WorkerFailure::new("output", "invalid-response", started));
    let end = end.ok_or_else(invalid)?;
    if end["stopReason"] != "end_turn" || written.len() > LIMIT {
        return Err(invalid().into());
    }
    let html = unfenced(&written).ok_or_else(invalid)?;
    let reported = end["modelUsage"]
        .as_object()
        .and_then(|models| models.keys().next())
        .map_or("", String::as_str);
    let cost = match end["total_cost_usd"].as_f64() {
        Some(cost) => format!("worker_cost_usd={cost:.6}"),
        None => format!("worker_cost_assumed_usd={UNREPORTED_COST_USD}"),
    };
    Ok((
        html,
        format!(
            "worker_model={}\nworker_input_tokens={}\nworker_output_tokens={}\n{cost}\nworker_duration_ms={}\n",
            single_line(reported),
            end["usage"]["input_tokens"].as_u64().unwrap_or(0),
            end["usage"]["output_tokens"].as_u64().unwrap_or(0),
            started.elapsed().as_millis(),
        ),
    ))
}

// Pi in print mode: no tools, no extensions, no project files, events as JSON lines.
fn pi(
    program: &str,
    scratch: &Path,
    job: &Path,
    prompt: &str,
    model: &str,
    effort: &str,
    mut show_code: impl FnMut(&str),
) -> Result<(String, String)> {
    let started = Instant::now();
    let named = |mut failure: WorkerFailure| {
        failure.worker = "Pi";
        failure
    };
    if !budget_left(job) {
        return Err(named(WorkerFailure::new("setup", "budget", started)).into());
    }
    let mut command = harness_command(program, scratch, job, ".pi")
        .map_err(|_| named(WorkerFailure::new("setup", "sandbox", started)))?;
    command.args([
        "-p",
        "--no-tools",
        "--no-extensions",
        "--no-mcp",
        "--no-skills",
        "--no-context-files",
        "--no-prompt-templates",
        "--no-session",
        "--offline",
        "--mode",
        "json",
    ]);
    if !model.is_empty() {
        command.args(["--model", model]);
    }
    if !effort.is_empty() {
        command.args(["--thinking", effort]);
    }
    command.arg("--").arg(prompt);
    let mut next_line = 0;
    let mut written = String::new();
    let mut last: Option<serde_json::Value> = None;
    run_worker_limited(
        &mut command,
        worker_ceiling(),
        STREAM_LIMIT,
        "execute",
        started,
        |data| {
            while let Some(newline) = data[next_line..].iter().position(|byte| *byte == b'\n') {
                let line = &data[next_line..next_line + newline];
                next_line += newline + 1;
                let Ok(mut event) = serde_json::from_slice::<serde_json::Value>(line) else {
                    continue;
                };
                match event["type"].as_str() {
                    Some("message_update") => {
                        let update = &event["assistantMessageEvent"];
                        if update["type"] == "text_delta"
                            && let Some(text) = update["delta"].as_str()
                        {
                            written.push_str(text);
                            show_code(&written);
                        }
                    }
                    Some("message_end") if event["message"]["role"] == "assistant" => {
                        last = Some(event["message"].take());
                    }
                    _ => {}
                }
            }
        },
    )
    .map_err(|error| match error.downcast::<WorkerFailure>() {
        Ok(failure) => named(*failure).into(),
        Err(error) => error,
    })?;
    let invalid = || named(WorkerFailure::new("output", "invalid-response", started));
    let last = last.ok_or_else(invalid)?;
    if last["stopReason"] != "stop" || written.is_empty() || written.len() > LIMIT {
        return Err(invalid().into());
    }
    let html = unfenced(&written).ok_or_else(invalid)?;
    let cost = match last["usage"]["cost"]["total"].as_f64() {
        Some(cost) => format!("worker_cost_usd={cost:.6}"),
        None => format!("worker_cost_assumed_usd={UNREPORTED_COST_USD}"),
    };
    Ok((
        html,
        format!(
            "worker_model={}/{}\nworker_input_tokens={}\nworker_output_tokens={}\n{cost}\nworker_duration_ms={}\n",
            single_line(last["provider"].as_str().unwrap_or("")),
            single_line(last["model"].as_str().unwrap_or("")),
            last["usage"]["input"].as_u64().unwrap_or(0),
            last["usage"]["output"].as_u64().unwrap_or(0),
            started.elapsed().as_millis(),
        ),
    ))
}

// Cost that Claude rounds recorded in this queue during the last 24 hours.
fn recent_worker_cost(job: &Path) -> f64 {
    let Some(jobs) = job.parent().and_then(|jobs| fs::read_dir(jobs).ok()) else {
        return 0.0;
    };
    let recent = |path: &Path| {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < Duration::from_secs(24 * 60 * 60))
    };
    jobs.flatten()
        .filter_map(|entry| fs::read_dir(entry.path()).ok())
        .flat_map(|rounds| rounds.flatten())
        .map(|round| round.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "run") && recent(path))
        .filter_map(|path| record::read_regular(&path, record::MAX_RECORD_BYTES).ok())
        .flat_map(|text| {
            text.lines()
                .filter_map(|line| {
                    line.strip_prefix("worker_cost_usd=")
                        .or_else(|| line.strip_prefix("worker_cost_assumed_usd="))?
                        .parse::<f64>()
                        .ok()
                })
                .collect::<Vec<_>>()
        })
        .sum()
}

// A second worker without tools. It sees the prompt and nothing of the project or the job.
fn claude(
    program: &str,
    scratch: &Path,
    job: &Path,
    prompt: &str,
    show_code: impl FnMut(&str),
) -> Result<(String, String)> {
    claude_as(program, scratch, job, prompt, "", "", show_code)
}

// The day's bound on what workers may spend on the user's accounts.
fn budget_left(job: &Path) -> bool {
    let budget = env::var(DAILY_BUDGET)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(DEFAULT_DAILY_BUDGET_USD);
    recent_worker_cost(job) < budget
}

/// One round by a named agent. The record carries what the harness reported, not what was asked for.
fn call_agent(
    agent: &crate::crew::Agent,
    scratch: &Path,
    job: &Path,
    prompt: &str,
    progress: &mut Progress,
    show_code: impl FnMut(&str),
) -> Result<(String, String)> {
    use crate::crew::Harness;
    let started = Instant::now();
    let missing = |worker: &'static str| {
        let mut failure = WorkerFailure::new("setup", "configuration", started);
        failure.worker = worker;
        failure
    };
    let located = |name: &str, worker: &'static str| -> Result<String> {
        crate::crew::program(name)
            .and_then(|path| path.to_str().map(str::to_owned))
            .ok_or_else(|| missing(worker).into())
    };
    match agent.harness {
        Harness::Savant => claude_as(
            SAVANT,
            scratch,
            job,
            prompt,
            &agent.model,
            &agent.effort,
            show_code,
        ),
        Harness::Claude => claude_as(
            &located("claude", "Claude")?,
            scratch,
            job,
            prompt,
            &agent.model,
            &agent.effort,
            show_code,
        ),
        Harness::Grok => grok(
            &located("grok", "Grok")?,
            scratch,
            job,
            prompt,
            &agent.model,
            &agent.effort,
            show_code,
        ),
        Harness::Pi => pi(
            &located("pi", "Pi")?,
            scratch,
            job,
            prompt,
            &agent.model,
            &agent.effort,
            show_code,
        ),
        Harness::Codex => {
            if !budget_left(job) {
                return Err(WorkerFailure::new("setup", "budget", started).into());
            }
            let mut usage = None;
            let html = codex_metered(scratch, job, prompt, progress, show_code, &mut usage)?;
            // With tokens and a listed price the amount is computed. Otherwise a fixed amount counts.
            let tokens = usage.map_or_else(String::new, |(input, _, output)| {
                format!("worker_input_tokens={input}\nworker_output_tokens={output}\n")
            });
            let cost = usage
                .and_then(|(input, cached, output)| {
                    crate::crew::cost(&agent.model, input, cached, output)
                })
                .map_or_else(
                    || format!("worker_cost_assumed_usd={UNREPORTED_COST_USD}"),
                    |cost| format!("worker_cost_usd={cost:.6}\nworker_cost_basis=price list"),
                );
            Ok((
                html,
                format!(
                    "{tokens}{cost}\nworker_duration_ms={}\n",
                    started.elapsed().as_millis()
                ),
            ))
        }
    }
}

// Model and effort may be empty. Then the harness default of the Factory applies.
fn claude_as(
    program: &str,
    scratch: &Path,
    job: &Path,
    prompt: &str,
    model: &str,
    effort: &str,
    mut show_code: impl FnMut(&str),
) -> Result<(String, String)> {
    let started = Instant::now();
    let savant = program == SAVANT;
    let named = move |mut failure: WorkerFailure| {
        failure.worker = if savant { "Savant" } else { "Claude" };
        failure
    };
    // The worker spends the user's account. Refuse before the call once the day's bound is reached.
    if !budget_left(job) {
        return Err(named(WorkerFailure::new("setup", "budget", started)).into());
    }
    if savant {
        return self::savant(
            scratch, job, prompt, model, effort, show_code, started, &named,
        );
    }
    let model = if model.is_empty() {
        "claude-opus-5-5"
    } else {
        model
    };
    let effort = if effort.is_empty() { "medium" } else { effort };
    let mut command = sandbox(scratch, job, program, true)
        .map_err(|_| named(WorkerFailure::new("setup", "sandbox", started)))?;
    command
        .args([
            "-p",
            "--model",
            model,
            "--effort",
            effort,
            "--tools",
            "",
            "--no-session-persistence",
            "--strict-mcp-config",
            "--setting-sources",
            "",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--system-prompt",
            "You are a code worker. Reply with the requested document only.",
            "--",
        ])
        .arg(prompt);
    // Each text delta extends the draft that the viewer shows while the worker writes.
    let mut next_line = 0;
    let mut written = String::new();
    let mut account = String::new();
    let response = run_worker_limited(
        &mut command,
        worker_ceiling(),
        STREAM_LIMIT,
        "execute",
        started,
        |data| {
            while let Some(newline) = data[next_line..].iter().position(|byte| *byte == b'\n') {
                let end = next_line + newline;
                if let Ok(event) =
                    serde_json::from_slice::<serde_json::Value>(&data[next_line..end])
                    && let Some(text) = event["event"]["delta"]["text"].as_str()
                    && event["event"]["delta"]["type"] == "text_delta"
                {
                    written.push_str(text);
                    show_code(&written);
                } else if let Ok(event) =
                    serde_json::from_slice::<serde_json::Value>(&data[next_line..end])
                    && event["type"] == "rate_limit_event"
                {
                    let info = &event["rate_limit_info"];
                    account = format!(
                        "{} {} {}",
                        info["rateLimitType"].as_str().unwrap_or(""),
                        info["status"].as_str().unwrap_or(""),
                        info["utilization"].as_f64().unwrap_or(0.0),
                    );
                }
                next_line = end + 1;
            }
        },
    )
    .map_err(|error| match error.downcast::<WorkerFailure>() {
        Ok(failure) => named(*failure).into(),
        Err(error) => error,
    })?;
    let invalid = || named(WorkerFailure::new("output", "invalid-response", started));
    let report = response
        .stdout
        .split(|byte| *byte == b'\n')
        .rev()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .find(|event| event["type"] == "result")
        .ok_or_else(invalid)?;
    if report["is_error"].as_bool() != Some(false) {
        return Err(invalid().into());
    }
    let html = report["result"]
        .as_str()
        .filter(|text| text.len() <= LIMIT)
        .and_then(unfenced)
        .ok_or_else(invalid)?;
    let model = report["modelUsage"]
        .as_object()
        .and_then(|models| models.keys().next())
        .map_or("", String::as_str);
    let usage = format!(
        "worker_model={}\nworker_output_tokens={}\nworker_cost_usd={}\nworker_duration_ms={}\nworker_account_usage={}\n",
        single_line(model),
        report["usage"]["output_tokens"].as_u64().unwrap_or(0),
        report["total_cost_usd"].as_f64().unwrap_or(0.0),
        started.elapsed().as_millis(),
        single_line(account.trim()),
    );
    Ok((html, usage))
}

fn codex_home(scratch: &Path) -> Result<PathBuf> {
    let isolated = scratch.join("codex-home");
    fs::create_dir(&isolated)?;
    let native = match env::var_os("CODEX_HOME") {
        Some(home) => PathBuf::from(home),
        None => PathBuf::from(env::var_os("HOME").ok_or("subscription session requires HOME")?)
            .join(".codex"),
    };
    symlink(
        native.join("auth.json").canonicalize()?,
        isolated.join("auth.json"),
    )?;
    Ok(isolated)
}

#[derive(Debug)]
struct WorkerFailure {
    stage: &'static str,
    kind: &'static str,
    elapsed: Duration,
    status: Option<ExitStatus>,
    provider_hint: &'static str,
    worker: &'static str,
}

impl WorkerFailure {
    fn new(stage: &'static str, kind: &'static str, started: Instant) -> Self {
        Self {
            stage,
            kind,
            elapsed: started.elapsed(),
            status: None,
            provider_hint: "",
            worker: "Codex",
        }
    }

    fn record_fields(&self) -> String {
        let exit_code = self.status.and_then(|status| status.code());
        let signal = self.status.and_then(|status| status.signal());
        format!(
            "worker_stage={}\nworker_failure={}\nworker_elapsed_ms={}\nworker_exit_code={}\nworker_signal={}\nworker_provider_hint={}\n",
            self.stage,
            self.kind,
            self.elapsed.as_millis(),
            exit_code.map_or_else(String::new, |code| code.to_string()),
            signal.map_or_else(String::new, |signal| signal.to_string()),
            self.provider_hint,
        )
    }
}

impl Display for WorkerFailure {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{} {} {}", self.worker, self.stage, self.kind)?;
        if let Some(status) = self.status {
            write!(output, " ({status})")?;
        }
        if !self.provider_hint.is_empty() {
            write!(output, " [{}]", self.provider_hint)?;
        }
        write!(output, " after {} ms", self.elapsed.as_millis())
    }
}

impl std::error::Error for WorkerFailure {}

fn run_worker(
    command: &mut Command,
    timeout: Duration,
    stage: &'static str,
    started: Instant,
    update: impl FnMut(&[u8]),
) -> Result<ChildResult> {
    run_worker_limited(command, timeout, 4 * LIMIT, stage, started, update)
}

fn run_worker_limited(
    command: &mut Command,
    timeout: Duration,
    limit: usize,
    stage: &'static str,
    started: Instant,
    update: impl FnMut(&[u8]),
) -> Result<ChildResult> {
    // A timeout at the ceiling means a leased worker call. Shorter calls such as a login have no lease.
    let lease = (timeout == worker_ceiling()).then(worker_lease);
    let result = run_child_limited(command, timeout, lease, limit, update).map_err(|error| {
        let kind = match error.downcast_ref::<ProcessLimit>() {
            Some(ProcessLimit::Deadline) => "timeout",
            Some(ProcessLimit::Stalled) => "stalled",
            Some(ProcessLimit::Output) => "output-limit",
            None => "process-error",
        };
        WorkerFailure::new(stage, kind, started)
    })?;
    if result.status.success() {
        return Ok(result);
    }
    let mut failure = WorkerFailure::new(stage, "exit", started);
    failure.status = Some(result.status);
    failure.provider_hint = provider_hint(&result.stderr);
    Err(failure.into())
}

fn provider_hint(stderr: &str) -> &'static str {
    // A diagnostic hint is not authority to retry. Never persist the worker's raw stderr.
    let stderr = stderr.to_ascii_lowercase();
    if stderr.contains("rate_limit_exceeded") || stderr.contains("rate limit exceeded") {
        "rate-limit"
    } else if stderr.contains("connection refused") || stderr.contains("failed to resolve host") {
        "unreachable"
    } else {
        ""
    }
}

fn verify(
    scratch: &Path,
    job: &Path,
    gate: &str,
    html: &str,
    continued: bool,
) -> Result<ChildResult> {
    fs::write(scratch.join("gate.mjs"), gate)?;
    fs::write(scratch.join("candidate.html"), html)?;
    run_child(
        sandbox(scratch, job, "node", false)?.args([
            "gate.mjs",
            "candidate.html",
            if continued { "follow-up" } else { "initial" },
        ]),
        GATE_TIMEOUT,
        |_| {},
    )
}

pub(crate) fn single_line(text: &str) -> String {
    text.chars()
        .take(1024)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

pub(crate) struct ChildResult {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: String,
}

#[derive(Debug)]
enum ProcessLimit {
    Deadline,
    Stalled,
    Output,
}

impl Display for ProcessLimit {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Deadline => "process deadline exceeded",
            Self::Stalled => "worker sent nothing within its lease",
            Self::Output => "worker output exceeded byte limit",
        })
    }
}

impl std::error::Error for ProcessLimit {}

impl ChildResult {
    fn follow_up(&self) -> Option<String> {
        let output = std::str::from_utf8(&self.stdout).ok()?;
        let mut findings = output
            .lines()
            .filter_map(|line| line.strip_prefix(FOLLOW_UP_LINE));
        let finding = findings.next()?;
        findings.next().is_none().then(|| finding.to_owned())
    }

    pub(crate) fn text(&self) -> String {
        format!("{}\n{}", String::from_utf8_lossy(&self.stdout), self.stderr)
    }

    fn diagnosis(&self, success: bool) -> String {
        if success {
            let text = String::from_utf8_lossy(&self.stdout);
            let checks: Vec<_> = text
                .lines()
                .filter(|line| !line.starts_with(FOLLOW_UP_LINE))
                .collect();
            return single_line(&checks.join(" "));
        }
        self.stderr
            .lines()
            .find(|line| line.contains("Error:"))
            .map(single_line)
            .unwrap_or_else(|| format!("Verifier exited with {}", self.status))
    }

    fn assertion(&self) -> Option<String> {
        if self.status.code() != Some(1) {
            return None;
        }
        self.stderr
            .lines()
            .find(|line| {
                line.starts_with("AssertionError [ERR_ASSERTION]:")
                    || line.starts_with("AssertionError:")
            })
            .map(single_line)
    }
}

/// A call without a ceiling of its own. It lives as long as it keeps its lease by output.
pub(crate) fn run_leased(
    command: &mut Command,
    lease: Duration,
    update: impl FnMut(&[u8]),
) -> Result<ChildResult> {
    run_child_limited(command, WORKER_CEILING, Some(lease), 128 * LIMIT, update)
}

pub(crate) fn run_child(
    command: &mut Command,
    timeout: Duration,
    update: impl FnMut(&[u8]),
) -> Result<ChildResult> {
    run_child_limited(command, timeout, None, 4 * LIMIT, update)
}

fn run_child_limited(
    command: &mut Command,
    timeout: Duration,
    lease: Option<Duration>,
    limit: usize,
    update: impl FnMut(&[u8]),
) -> Result<ChildResult> {
    let deadline = Instant::now() + timeout;
    let mut stderr = tempfile::tempfile()?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(stderr.try_clone()?)
        .process_group(0);
    // Only an async-signal-safe system call executes after fork.
    unsafe {
        command.pre_exec(|| {
            let limit = libc::rlimit {
                rlim_cur: 16 * 1024 * 1024,
                rlim_max: 16 * 1024 * 1024,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn()?;
    let execution = (|| -> Result<ChildResult> {
        let fd = child
            .stdout
            .as_ref()
            .ok_or("missing worker output")?
            .as_raw_fd();
        let bytes = read_bytes(fd, deadline, lease, limit, update)?;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                return Err(ProcessLimit::Deadline.into());
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        stderr.rewind()?;
        let mut diagnostic = Vec::new();
        stderr.take(8192).read_to_end(&mut diagnostic)?;
        Ok(ChildResult {
            status,
            stdout: bytes,
            stderr: String::from_utf8_lossy(&diagnostic).into_owned(),
        })
    })();
    if execution.is_err() {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.wait();
    }
    execution
}

fn read_bytes(
    fd: RawFd,
    deadline: Instant,
    lease: Option<Duration>,
    limit: usize,
    mut update: impl FnMut(&[u8]),
) -> Result<Vec<u8>> {
    let mut all = Vec::new();
    let mut buffer = [0u8; 4096];
    let mut renewed = Instant::now();
    loop {
        let lease_end = lease.map(|lease| renewed + lease);
        let stalls_first = lease_end.is_some_and(|end| end < deadline);
        let until = if stalls_first {
            lease_end.unwrap_or(deadline)
        } else {
            deadline
        };
        let expired = || {
            if stalls_first {
                ProcessLimit::Stalled
            } else {
                ProcessLimit::Deadline
            }
        };
        let remaining = until.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(expired().into());
        }
        let mut signal = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut signal, 1, remaining.as_millis().max(1) as i32) };
        if ready == 0 {
            return Err(expired().into());
        }
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        let size = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if size == 0 {
            return Ok(all);
        }
        if size < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        let size = size as usize;
        if all.len() + size > limit {
            return Err(ProcessLimit::Output.into());
        }
        all.extend_from_slice(&buffer[..size]);
        renewed = Instant::now();
        update(&all);
    }
}
