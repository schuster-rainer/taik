//! A job done by an agent session: an agent with file tools works in a copy of a repository, keeps
//! its session over the rounds, and a pinned gate judges the commit it leaves behind.
//!
//! The copy is a local clone in `STATE/work/NAME`. The agent reads and writes there and nowhere else
//! in the user's home. It has no shell: the sign-in of the harness needs the keychain, and a shell in
//! the same sandbox could read other secrets from it. Building and testing is the work of the gate.
//!
//! After each call the Factory commits what the agent left, runs the gate on a clean checkout of
//! exactly that commit and writes the decision. The gate comes from the job folder, which the agent
//! cannot read. A change to a path the repository protects is rejected before the gate runs.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::progress::Progress;
use crate::{Result, record, source, talk};

pub const BRIEF: &str = "session.brief.md";
pub const GATE: &str = "session.gate.sh";
const REPO: &str = "session.repo";
// Assumptions of the author of this module, not rules of the user. Both can be set per service.
const ROUNDS: usize = 6;
const LEASE: Duration = Duration::from_secs(600);
const GATE_TIMEOUT: Duration = Duration::from_secs(3600);
const MAX_DIFF: usize = 512 * 1024;
const MAX_LOG_TAIL: u64 = 64 * 1024;
const TOOLS: &str = "Read,Edit,Write,Glob,Grep";
const RULES: &str = "You work in the current directory, a private copy of the repository. Change files there with your tools. \
You have no shell: do not try to build, test or commit. An independent gate builds and tests your result after you stop, and its finding comes back to you. \
When you are done, end with at most five plain sentences in German: what you changed and why, and anything the user must know. \
If the work cannot be done without an answer from the user, change nothing and end with the line '=== FRAGE ===' followed by one question in German.";

fn milliseconds(variable: &str, default: Duration) -> Duration {
    std::env::var(variable)
        .ok()
        .and_then(|value| value.parse().ok())
        .map_or(default, Duration::from_millis)
}

fn git(folder: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args([
            "-c",
            "user.name=TAIK Factory",
            "-c",
            "user.email=factory@taik.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {}: {}",
            arguments.first().unwrap_or(&""),
            source::single_line(&String::from_utf8_lossy(&output.stderr))
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Queues a session job. The repository is optional: without one the agent starts in an empty copy.
pub fn queue(
    state: &Path,
    name: &str,
    brief: &Path,
    gate: &Path,
    repo: Option<&Path>,
    handover: crate::service::Handover<'_>,
) -> Result<PathBuf> {
    let crate::service::Handover { agent, said, plan } = handover;
    let name = crate::worker_name(name).ok_or("invalid job name")?;
    let brief = record::read_regular(brief, record::MAX_RECORD_BYTES)?;
    source::intent(&brief)?;
    let gate = record::read_regular(gate, source::LIMIT)?;
    if gate.trim().is_empty() {
        return Err("a session job requires a nonempty gate".into());
    }
    let jobs = state.join("jobs").canonicalize()?;
    let job = jobs.join(format!("job-{name}"));
    if job.symlink_metadata().is_ok() {
        return Err(format!("job already exists: {}", job.display()).into());
    }
    let staged = tempfile::Builder::new()
        .prefix(".job-")
        .tempdir_in(jobs.parent().ok_or("job folder without a parent")?)?;
    record::publish_text(&staged.path().join(BRIEF), &brief, record::MAX_RECORD_BYTES)?;
    record::publish_text(&staged.path().join(GATE), &gate, source::LIMIT)?;
    if let Some(repo) = repo {
        let repo = repo.canonicalize()?;
        git(&repo, &["rev-parse", "--git-dir"])?;
        record::publish_text(
            &staged.path().join(REPO),
            &format!("{}\n", repo.display()),
            record::MAX_RECORD_BYTES,
        )?;
    }
    // The agent and what was said before the job existed are in place before the service sees it.
    if let Some(agent) = agent {
        if !crate::crew::valid_name(agent) {
            return Err("invalid agent name".into());
        }
        fs::write(staged.path().join("agent"), format!("{agent}\n"))?;
    }
    for message in said {
        talk::write(staged.path(), message.author, 0, &message.text)?;
    }
    // The plan of the proposal travels with the job, byte for byte, before the service sees it.
    if let Some(plan) = plan {
        let text = record::read_regular(plan, record::MAX_RECORD_BYTES)?;
        record::publish_text(
            &staged.path().join("plan.txt"),
            &text,
            record::MAX_RECORD_BYTES,
        )?;
    }
    record::publish(&staged.path().join("request.run"), source::REQUEST)?;
    fs::rename(staged.path(), &job)?;
    let _ = staged.keep();
    Ok(job)
}

/// Runs the rounds that are due. Returns the decision of the last round of this call, if one ran.
pub fn run_queued(
    home: &Path,
    job: &Path,
    agent: Option<&crate::crew::Agent>,
) -> Result<Option<bool>> {
    let _lock = source::lock_job(job)?;
    let mut outcome = None;
    loop {
        let rounds = talk::rounds_done(job);
        let said = talk::read(job);
        if talk::waits_for_user(&said) {
            return Ok(outcome);
        }
        let last = (rounds > 0)
            .then(|| {
                record::read_regular(
                    &job.join(format!("round-{rounds:02}.run")),
                    record::MAX_RECORD_BYTES,
                )
            })
            .transpose()?;
        let accepted = last
            .as_deref()
            .is_some_and(|text| text.starts_with("decision=accepted\n"));
        let replied = talk::open_reply(&said, rounds);
        if accepted && !replied {
            return Ok(outcome);
        }
        // A rejected round is followed by the next one until the budget is used. A reply adds one.
        if rounds >= ROUNDS + talk::granted_rounds(&said) && !replied {
            return Ok(outcome);
        }
        if rounds >= 99 {
            return Ok(outcome);
        }
        match round(home, job, rounds + 1, agent, &said, last.as_deref())? {
            Some(accepted) => outcome = Some(accepted),
            // The agent asked. The job waits for the user.
            None => return Ok(outcome),
        }
    }
}

fn session_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

// The copy of a job. Created once, from the repository the job names, at the commit it had then.
fn copy_of(home: &Path, job: &Path) -> Result<(PathBuf, String)> {
    let name = job
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("job-"))
        .ok_or("a session job is named job-NAME")?;
    let work = home.join("work");
    fs::create_dir_all(&work)?;
    let copy = work.canonicalize()?.join(name);
    let base_file = job.join("session.base");
    if copy.join(".git").exists() && base_file.is_file() {
        let base = record::read_regular(&base_file, 128)?;
        return Ok((copy, base.trim().to_owned()));
    }
    if copy.exists() {
        return Err("the copy of this job exists without its record".into());
    }
    match record::read_regular(&job.join(REPO), record::MAX_RECORD_BYTES) {
        Ok(repo) => {
            let output = Command::new("git")
                .args(["clone", "--quiet", "--local", "--no-hardlinks"])
                .arg(repo.trim())
                .arg(&copy)
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "cannot copy the repository: {}",
                    source::single_line(&String::from_utf8_lossy(&output.stderr))
                )
                .into());
            }
            // The copy has no way back to its origin. Results leave it only through the user.
            git(&copy, &["remote", "remove", "origin"])?;
        }
        Err(_) => {
            fs::create_dir(&copy)?;
            git(&copy, &["init", "--quiet"])?;
            git(
                &copy,
                &["commit", "--quiet", "--allow-empty", "-m", "Empty start"],
            )?;
        }
    }
    let base = git(&copy, &["rev-parse", "HEAD"])?.trim().to_owned();
    record::publish_text(&base_file, &format!("{base}\n"), 128)?;
    Ok((copy, base))
}

// Home is closed except for what the harness needs. The copy and a scratch folder are writable.
fn sandbox(program: &Path, copy: &Path, scratch: &Path, jobs: &Path) -> Result<Command> {
    let home = std::env::var("HOME")?;
    for path in [
        home.as_str(),
        copy.to_str().ok_or("copy path is not UTF-8")?,
        scratch.to_str().ok_or("scratch path is not UTF-8")?,
        jobs.to_str().ok_or("job path is not UTF-8")?,
    ] {
        if path.contains(['"', '\\', '(', ')']) {
            return Err("a path cannot be named in the sandbox policy".into());
        }
    }
    let policy = format!(
        r##"(version 1) (allow default) (deny file-write*)
(allow file-write* (subpath (param "WORK"))) (allow file-write* (subpath (param "TMP"))) (allow file-write* (literal "/dev/null"))
(allow file-write* (subpath "{home}/.claude")) (allow file-write* (regex #"^{home}/\.claude\.json"))
(deny file-read* (subpath "{home}"))
(allow file-read* (literal "{home}")) (allow file-read-metadata (subpath "{home}"))
(allow file-read* (subpath "{home}/.claude")) (allow file-read* (regex #"^{home}/\.claude\.json"))
(allow file-read* (subpath "{home}/.local/bin")) (allow file-read* (subpath "{home}/.local/share/claude"))
(allow file-read* (subpath "{home}/Library/Keychains"))
(allow file-read* (subpath (param "WORK"))) (allow file-read* (subpath (param "TMP")))
(deny file-read* (subpath (param "JOBS")))"##
    );
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command
        .arg("-D")
        .arg(format!("WORK={}", copy.display()))
        .arg("-D")
        .arg(format!("TMP={}", scratch.display()))
        .arg("-D")
        .arg(format!("JOBS={}", jobs.display()))
        .arg("-p")
        .arg(policy)
        .arg(program)
        .current_dir(copy)
        .env_clear();
    for key in ["PATH", "HOME", "LANG", "USER", "LOGNAME"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command.env("TMPDIR", scratch);
    Ok(command)
}

struct Call {
    result: String,
    model: String,
    cost: f64,
    cache_read: u64,
    output_tokens: u64,
}

// One call of the harness. Its stream is read line by line: what the agent says and which tool it
// uses on which file goes to the dialog of the round and to the work steps, while it happens.
fn call(
    program: &Path,
    arguments: &[String],
    copy: &Path,
    jobs: &Path,
    dialog: &Path,
    progress: &mut Progress,
) -> Result<Call> {
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let mut command = sandbox(program, copy, &scratch, jobs)?;
    command.args(arguments);
    let mut log = File::options().create(true).append(true).open(dialog)?;
    let mut next = 0;
    let mut call = None;
    let prefix = format!("{}/", copy.display());
    let response = source::run_leased(
        &mut command,
        milliseconds("TAIK_SESSION_LEASE_MS", LEASE),
        |data| {
            while let Some(newline) = data[next..].iter().position(|byte| *byte == b'\n') {
                let end = next + newline;
                let line = &data[next..end];
                next = end + 1;
                let Ok(event) = serde_json::from_slice::<serde_json::Value>(line) else {
                    continue;
                };
                if event["type"] == "assistant" {
                    for block in event["message"]["content"].as_array().into_iter().flatten() {
                        let entry = if block["type"] == "text" {
                            let text = block["text"].as_str().unwrap_or("").trim();
                            if text.is_empty() {
                                continue;
                            }
                            serde_json::json!({"who": "agent", "text": text})
                        } else if block["type"] == "tool_use" {
                            let tool = block["name"].as_str().unwrap_or("?");
                            let input = &block["input"];
                            let target = input["file_path"]
                                .as_str()
                                .or_else(|| input["pattern"].as_str())
                                .or_else(|| input["path"].as_str())
                                .unwrap_or("");
                            let target = target.strip_prefix(&prefix).unwrap_or(target);
                            progress.event(
                                "tool",
                                format_args!("{tool} {}", source::single_line(target)),
                            );
                            serde_json::json!({"tool": tool, "target": target})
                        } else {
                            continue;
                        };
                        let _ = writeln!(log, "{entry}");
                    }
                } else if event["type"] == "result" {
                    call = Some(event);
                }
            }
        },
    )?;
    // A last line without a line end still counts.
    if call.is_none()
        && let Some(event) = response
            .stdout
            .rsplit(|byte| *byte == b'\n')
            .find_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
            .filter(|event| event["type"] == "result")
    {
        call = Some(event);
    }
    if !response.status.success() && call.is_none() {
        return Err(format!(
            "the harness ended with {}: {}",
            response.status,
            source::single_line(&response.stderr)
                .chars()
                .take(200)
                .collect::<String>()
        )
        .into());
    }
    let report = call.ok_or("the harness sent no result")?;
    if report["is_error"].as_bool() != Some(false) {
        return Err(format!(
            "the harness reported an error: {}",
            source::single_line(report["result"].as_str().unwrap_or(""))
                .chars()
                .take(200)
                .collect::<String>()
        )
        .into());
    }
    Ok(Call {
        result: report["result"].as_str().unwrap_or("").trim().to_owned(),
        model: report["modelUsage"]
            .as_object()
            .and_then(|models| models.keys().next())
            .cloned()
            .unwrap_or_default(),
        cost: report["total_cost_usd"].as_f64().unwrap_or(0.0),
        cache_read: report["usage"]["cache_read_input_tokens"]
            .as_u64()
            .unwrap_or(0),
        output_tokens: report["usage"]["output_tokens"].as_u64().unwrap_or(0),
    })
}

/// One call of an agent that may read a repository and change nothing: the author of a gate. It
/// works in a throwaway copy with Read, Glob and Grep and returns its answer with the usage lines.
pub fn read_and_answer(
    home: &Path,
    repo: &Path,
    agent: Option<&crate::crew::Agent>,
    prompt: &str,
) -> Result<(String, String)> {
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let copy = scratch.join("copy");
    let cloned = Command::new("git")
        .args(["clone", "--quiet", "--local", "--no-hardlinks"])
        .arg(repo)
        .arg(&copy)
        .output()?;
    if !cloned.status.success() {
        return Err("cannot copy the repository for the author of the gate".into());
    }
    let program = std::env::var_os("TAIK_SESSION_AGENT")
        .map(PathBuf::from)
        .or_else(|| crate::crew::program("claude"))
        .ok_or("the Claude CLI is not installed")?;
    let (model, effort) = agent.map_or(("", ""), |agent| {
        (agent.model.as_str(), agent.effort.as_str())
    });
    let arguments: Vec<String> = [
        "-p",
        "--model",
        if model.is_empty() {
            "claude-opus-5-5"
        } else {
            model
        },
        "--effort",
        if effort.is_empty() { "medium" } else { effort },
        "--tools",
        "Read,Glob,Grep",
        "--permission-mode",
        "bypassPermissions",
        "--strict-mcp-config",
        "--setting-sources",
        "",
        "--no-session-persistence",
        "--output-format",
        "stream-json",
        "--verbose",
        "--",
        prompt,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let jobs = home.join("jobs").canonicalize()?;
    let answer = call(
        &program,
        &arguments,
        &copy,
        &jobs,
        &scratch.join("dialog.jsonl"),
        &mut Progress::silent(),
    )?;
    Ok((
        answer.result,
        format!(
            "worker_model={}\nworker_output_tokens={}\nsession_listed_usd={}\n",
            source::single_line(&answer.model),
            answer.output_tokens,
            answer.cost
        ),
    ))
}

// The pinned gate judges a clean checkout of one commit. Its output is kept beside the round.
fn judge(job: &Path, copy: &Path, commit: &str, gate: &str, log: &Path) -> Result<(bool, String)> {
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let checkout = scratch.join("checkout");
    let cloned = Command::new("git")
        .args(["clone", "--quiet", "--local", "--no-hardlinks"])
        .arg(copy)
        .arg(&checkout)
        .output()?;
    if !cloned.status.success() {
        return Err("cannot check out the commit for the gate".into());
    }
    git(&checkout, &["checkout", "--quiet", "--detach", commit])?;
    fs::write(scratch.join("gate.sh"), gate)?;
    let mut command = source::sandbox_for_build(&scratch, job, "/bin/sh")?;
    command
        .args(["-c", "/bin/sh ../gate.sh > ../gate.log 2>&1"])
        .current_dir(&checkout);
    let ran = source::run_child(
        &mut command,
        milliseconds("TAIK_SESSION_GATE_MS", GATE_TIMEOUT),
        |_| {},
    );
    let mut tail = Vec::new();
    if let Ok(mut file) = File::open(scratch.join("gate.log")) {
        use std::io::Seek;
        let length = file.metadata()?.len();
        file.seek(std::io::SeekFrom::Start(
            length.saturating_sub(MAX_LOG_TAIL),
        ))?;
        file.read_to_end(&mut tail)?;
    }
    fs::write(log, &tail)?;
    let text = String::from_utf8_lossy(&tail);
    let last = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map_or_else(
            || "the gate printed nothing".to_owned(),
            source::single_line,
        );
    match ran {
        Ok(result) if result.status.success() => Ok((true, last)),
        Ok(result) => Ok((false, format!("gate failed ({}): {last}", result.status))),
        Err(error) => Ok((false, format!("the gate did not finish: {error}"))),
    }
}

// One round. `Some(decision)` after a judged round, `None` when the agent asked the user.
fn round(
    home: &Path,
    job: &Path,
    number: usize,
    agent: Option<&crate::crew::Agent>,
    said: &[talk::Message],
    last: Option<&str>,
) -> Result<Option<bool>> {
    let started = Instant::now();
    let stem = format!("round-{number:02}");
    let output = job.join(format!("{stem}.run"));
    let brief = record::read_regular(&job.join(BRIEF), record::MAX_RECORD_BYTES)?;
    let intent = source::intent(&brief)?.to_owned();
    let gate = record::read_regular(&job.join(GATE), source::LIMIT)?;
    let digest = format!("{:x}", Sha256::digest(gate.as_bytes()));
    source::pin(&job.join("gate.sha256"), &format!("{digest}\n"))?;
    let mut progress = Progress::for_round(&output);
    let (copy, base) = copy_of(home, job)?;
    let before = git(&copy, &["rev-parse", "HEAD"])?.trim().to_owned();
    progress.event("start", format_args!("Sitzung in {}", copy.display()));

    // The first round gets the work order. Later rounds get what happened since: the finding of the
    // gate and what the user said. The session already holds the rest.
    let id_file = job.join("session.id");
    let known = record::read_regular(&id_file, 64)
        .ok()
        .map(|id| id.trim().to_owned());
    let mut prompt = String::new();
    if known.is_none() {
        prompt.push_str(&format!("{RULES}\n\nWORK ORDER:\n{brief}"));
    } else if let Some(record) = last
        && record.starts_with("decision=rejected\n")
    {
        let field = |key: &str| {
            record
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .unwrap_or("")
                .to_owned()
        };
        let log = fs::read(job.join(format!("round-{:02}.gate.log", number - 1)))
            .map(|bytes| {
                let start = bytes.len().saturating_sub(4000);
                String::from_utf8_lossy(&bytes[start..]).into_owned()
            })
            .unwrap_or_default();
        prompt.push_str(&format!(
            "Your last state was rejected. Finding: {}\n\nEnd of the gate's output:\n{log}\n\nCorrect your work in the copy.",
            field("checks=")
        ));
    }
    let news: Vec<_> = said
        .iter()
        .filter(|message| message.author == talk::Author::User && message.round + 1 >= number)
        .collect();
    for message in &news {
        prompt.push_str(&format!("\n\nThe user says: {}", message.text));
    }
    if prompt.is_empty() {
        prompt.push_str("Continue the work order.");
    }
    if let Some(agent) = agent
        && !agent.persona.is_empty()
        && known.is_none()
    {
        prompt = format!("{}\n\n{prompt}", agent.persona);
    }

    let program = std::env::var_os("TAIK_SESSION_AGENT")
        .map(PathBuf::from)
        .or_else(|| crate::crew::program("claude"))
        .ok_or("the Claude CLI is not installed")?;
    let (model, effort) = match agent {
        Some(agent) if agent.harness == crate::crew::Harness::Claude => {
            (agent.model.as_str(), agent.effort.as_str())
        }
        Some(_) => return Err("a session needs an agent on the Claude harness".into()),
        None => ("", ""),
    };
    let id = match &known {
        Some(id) => id.clone(),
        None => session_id()?,
    };
    let mut arguments: Vec<String> = [
        "-p",
        "--model",
        if model.is_empty() {
            "claude-opus-5-5"
        } else {
            model
        },
        "--effort",
        if effort.is_empty() { "medium" } else { effort },
        "--tools",
        TOOLS,
        "--permission-mode",
        "bypassPermissions",
        "--strict-mcp-config",
        "--setting-sources",
        "",
        "--output-format",
        "stream-json",
        "--verbose",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    arguments.push(
        if known.is_some() {
            "--resume"
        } else {
            "--session-id"
        }
        .to_owned(),
    );
    arguments.push(id.clone());
    arguments.push("--".to_owned());
    arguments.push(prompt);

    let jobs = job.parent().ok_or("job without a queue")?;
    let dialog = job.join(format!("{stem}.dialog.jsonl"));
    {
        let mut log = File::options().create(true).append(true).open(&dialog)?;
        for message in &news {
            let _ = writeln!(
                log,
                "{}",
                serde_json::json!({"who": "user", "text": message.text})
            );
        }
    }
    progress.event(
        "worker",
        format_args!(
            "{} die Sitzung {}",
            if known.is_some() {
                "Setzt fort:"
            } else {
                "Beginnt"
            },
            &id[..8]
        ),
    );
    let answer = call(&program, &arguments, &copy, jobs, &dialog, &mut progress);
    if known.is_none() && answer.is_ok() {
        record::publish_text(&id_file, &format!("{id}\n"), 64)?;
    }
    let mut usage = String::new();
    let mut summary = String::new();
    let mut failure = String::new();
    match answer {
        Ok(call) => {
            // The price at the list is told, not charged: the session runs on the user's subscription.
            usage = format!(
                "worker_model={}\nworker_output_tokens={}\nworker_cache_read_tokens={}\nsession_listed_usd={}\n",
                source::single_line(&call.model),
                call.output_tokens,
                call.cache_read,
                call.cost,
            );
            if let Some(question) = call.result.split("=== FRAGE ===").nth(1) {
                let question = question.trim();
                if !question.is_empty() {
                    talk::write(job, talk::Author::Agent, number - 1, question)?;
                    progress.event("ask", format_args!("{}", source::single_line(question)));
                    return Ok(None);
                }
            }
            summary = source::single_line(&call.result)
                .chars()
                .take(1500)
                .collect();
        }
        Err(error) => failure = source::single_line(&error.to_string()),
    }

    // What the agent left is one commit. Exactly that commit is judged.
    git(&copy, &["add", "--all"])?;
    let changed = Command::new("git")
        .arg("-C")
        .arg(&copy)
        .args(["diff", "--cached", "--quiet"])
        .status()?
        .code()
        == Some(1);
    if changed {
        git(
            &copy,
            &["commit", "--quiet", "-m", &format!("TAIK {stem}: {intent}")],
        )?;
    }
    let commit = git(&copy, &["rev-parse", "HEAD"])?.trim().to_owned();
    let files: Vec<String> = git(&copy, &["diff", "--numstat", &before, &commit])?
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let (added, removed, path) = (parts.next()?, parts.next()?, parts.next()?);
            Some(format!("{path} +{added} -{removed}"))
        })
        .collect();
    let mut patch = git(&copy, &["diff", &before, &commit])?;
    if patch.len() > MAX_DIFF {
        let mut end = MAX_DIFF;
        while !patch.is_char_boundary(end) {
            end -= 1;
        }
        patch.truncate(end);
    }
    fs::write(job.join(format!("{stem}.diff")), &patch)?;
    progress.event(
        "checks",
        format_args!("{} Dateien geändert, Commit {}", files.len(), &commit[..8]),
    );

    // Paths the repository protected at the start of the job. The agent cannot change that list.
    let protected: Vec<String> = git(&copy, &["show", &format!("{base}:.taik/protected")])
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    let touched: Vec<String> = git(&copy, &["diff", "--name-only", &base, &commit])?
        .lines()
        .filter(|path| {
            *path == ".taik/protected" || protected.iter().any(|prefix| path.starts_with(prefix))
        })
        .map(str::to_owned)
        .collect();

    let gate_started = Instant::now();
    let mut gate_ms = String::new();
    let (accepted, checks) = if !failure.is_empty() {
        (false, format!("the agent did not finish: {failure}"))
    } else if !touched.is_empty() {
        (
            false,
            format!("the change touches protected paths: {}", touched.join(", ")),
        )
    } else if commit == base {
        (false, "the agent changed nothing".to_owned())
    } else {
        progress.event(
            "evaluate",
            format_args!("Gate prüft Commit {}", &commit[..8]),
        );
        let judged = judge(
            job,
            &copy,
            &commit,
            &gate,
            &job.join(format!("{stem}.gate.log")),
        )?;
        gate_ms = gate_started.elapsed().as_millis().to_string();
        judged
    };
    let decision = if accepted { "accepted" } else { "rejected" };
    let report = format!(
        "decision={decision}\nkind=software\nprocedure=\ntags=\nidentity={stem}\nintent={intent}\nbaseline={base}\nworker=claude-session\n{usage}\
         worker_failure={}\nworker_duration_ms={}\nsession={id}\nlimits={ROUNDS} rounds and one per reply; lease {}s without ceiling; file tools only\n\
         gate_sha256={digest}\ngate_ms={gate_ms}\nchecks={}\nartifact={}\ncommit={commit}\nfiles={}\nsummary={summary}\n\
         continuation={}\nadvance=false\nend\n",
        if failure.is_empty() { "" } else { "session" },
        started.elapsed().as_millis(),
        milliseconds("TAIK_SESSION_LEASE_MS", LEASE).as_secs(),
        source::single_line(&checks)
            .chars()
            .take(1200)
            .collect::<String>(),
        if accepted { commit.as_str() } else { "" },
        source::single_line(&files.join("; "))
            .chars()
            .take(1500)
            .collect::<String>(),
        if accepted {
            format!(
                "The accepted state is commit {} in {}.",
                &commit[..8],
                copy.display()
            )
        } else {
            "The copy keeps the rejected state. The next round continues the session with this finding.".to_owned()
        },
    );
    crate::emit(&output, &report)?;
    progress.event(decision, format_args!("{}", source::single_line(&checks)));
    Ok(Some(accepted))
}
