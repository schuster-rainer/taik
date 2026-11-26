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

Candidate format: one text document of at most 512 KiB. Either a standalone HTML document that starts with <!doctype html>, or a bundle of source files, each introduced by a line '=== path ===', when the job is a program.

Gate: a Node.js ES module of at most 12000 bytes. It reads the candidate from process.argv[2], uses node:assert/strict, and exits with a failed assertion that carries a clear message when a requirement is not met. On success it prints one line that starts with 'PASS: '. It runs in a sandbox without network. It may write only inside its working directory and has 15 seconds. It may unpack a bundle there and compile and run it. rustc is at process.env.HOME + '/.cargo/bin/rustc'. No other compiler is guaranteed.

Rules for the gate: check behaviour and results, not the presence of words. Every requirement must be satisfiable together with all others. Never require what the brief does not state. Keep the scope of the job as small as the record's continuation justifies.

If the record justifies no further job, reply with exactly one line 'STOP: <reason in one German sentence>' and nothing else. Do not invent work.";

const LOOP: &str = "\n\nThe continuation of a round record that says no further work was authorized concerns that round alone. It is not a reason to stop. Choose the next job from the open intents listed below when one of them can be done as one job, and begin your reply with the line 'INTENT: <number>' when you take one.\n\nTo change a listed product instead of writing a new artifact, reply with the INTENT line if any, then the line '=== product <name> ===' and below it only the brief. Write no gate for a product: its own pinned gate applies. A product brief names exactly what changes and says that everything else stays as it is.\n\nReply 'STOP: <reason>' only when no open intent can be done as one job and the record justifies nothing else.";

/// What the Factory itself wrote down as justified next work and nobody has taken up yet: the
/// continuation of its observation records. Newest first.
fn open_intents(jobs: &Path) -> Vec<(String, String)> {
    let mut taken = Vec::new();
    let mut found: Vec<(std::time::SystemTime, String, String)> = Vec::new();
    for folder in fs::read_dir(jobs).into_iter().flatten().flatten() {
        for file in fs::read_dir(folder.path()).into_iter().flatten().flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".run") {
                continue;
            }
            let Ok(text) = record::read_regular(&file.path(), record::MAX_RECORD_BYTES) else {
                continue;
            };
            if name.starts_with("reflection-") {
                taken.extend(
                    text.lines()
                        .filter_map(|line| line.strip_prefix("intent_from="))
                        .map(str::to_owned),
                );
            } else if (name.starts_with("summary") || name.starts_with("observation"))
                && text.starts_with("decision=observed\n")
                && let Some(next) = text
                    .lines()
                    .rev()
                    .find_map(|line| line.strip_prefix("continuation="))
            {
                let changed = file
                    .metadata()
                    .and_then(|data| data.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                found.push((
                    changed,
                    file.path().display().to_string(),
                    source::single_line(next),
                ));
            }
        }
    }
    found.sort_by(|first, second| second.0.cmp(&first.0));
    found
        .into_iter()
        .filter(|(_, path, _)| !taken.contains(path))
        .map(|(_, path, text)| (path, text))
        .take(25)
        .collect()
}

/// An existing artifact with its pinned gate that a job may build on, from STATE/products/NAME.product.
pub struct Product {
    pub name: String,
    pub about: String,
    seed: PathBuf,
    gate: PathBuf,
}

pub fn products(state: &Path) -> Vec<Product> {
    let mut list = Vec::new();
    for file in fs::read_dir(state.join("products"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let Some(name) = file
            .file_name()
            .to_str()
            .and_then(|name| name.strip_suffix(".product"))
            .map(str::to_owned)
        else {
            continue;
        };
        let Ok(text) = record::read_regular(&file.path(), record::MAX_RECORD_BYTES) else {
            continue;
        };
        let field = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key))
                .unwrap_or("")
                .trim()
                .to_owned()
        };
        if crate::worker_name(&name).is_some()
            && text.ends_with("end\n")
            && !field("seed=").is_empty()
            && !field("gate=").is_empty()
        {
            list.push(Product {
                name,
                about: field("about="),
                seed: field("seed=").into(),
                gate: field("gate=").into(),
            });
        }
    }
    list.sort_by(|first, second| first.name.cmp(&second.name));
    list
}

pub fn propose(state: &Path, name: &str, record_path: &Path) -> Result<Option<PathBuf>> {
    propose_planned(state, name, record_path, None)
}

const SESSION_CONTRACT: &str = "You prepare a job of the TAIK Factory that an agent will do in a session on a repository. Write a brief and an independent gate. Another agent will later see only the brief, work in a copy of the repository with file tools and no shell, and never see the gate.

Reply with exactly two files in this form and nothing else:

=== brief.md ===
<brief>
=== gate.sh ===
<gate>

Brief: Markdown in German, at most 6000 bytes. Its first line is one sentence that names the intent, without Markdown markup. State everything the agent needs: which behaviour must hold afterwards, every name and value that the gate checks, and what must stay as it is.

Gate: a POSIX shell script of at most 12000 bytes. It runs in a clean checkout of the agent's commit, as the working directory, in a sandbox without network. It exits with a status other than 0 and prints one clear line when a requirement is not met. On success its last line starts with 'PASS: '. It may use only tools that the listed files show the repository already uses, plus sh, grep, test and node.

Rules for the gate: check behaviour and results, not the presence of words. Every requirement must be satisfiable together with all others. Never require what the brief does not state. Keep the job as small as the request justifies. The sandbox of the gate cannot start another sandbox and has no network: run only the test targets this job adds or names, never the whole test suite of the repository. Print the reason of a failure, including the relevant output of a failed build or test.

If the request cannot be checked by a gate or is unclear, reply with exactly one line 'STOP: <reason in one German sentence>' and nothing else.";

/// Derives a proposal. With a plan of the orchestrator the gate is written by the agent the plan
/// names, and a session job gets a shell gate for its repository.
pub fn propose_planned(
    state: &Path,
    name: &str,
    record_path: &Path,
    plan: Option<&crate::orchestrator::Plan>,
) -> Result<Option<PathBuf>> {
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
    if job.join(format!("reflection-{name}.run")).try_exists()? {
        return Err("this record was already reflected under that name".into());
    }
    let origin_brief =
        record::read_regular(&job.join("submission.brief.md"), record::MAX_RECORD_BYTES)
            .unwrap_or_default();
    let intents = open_intents(&state.join("jobs"));
    let products = products(&state);
    let mut listing =
        String::from("\n\nOPEN INTENTS OF THE FACTORY, from its own observation records:\n");
    if intents.is_empty() {
        listing.push_str("(none)\n");
    }
    for (number, (_, text)) in intents.iter().enumerate() {
        listing.push_str(&format!("{}. {text}\n", number + 1));
    }
    listing.push_str("\nPRODUCTS THAT A JOB MAY CHANGE. Such a job starts from the product's accepted source and is judged by the product's own pinned gate:\n");
    if products.is_empty() {
        listing.push_str("(none)\n");
    }
    for product in &products {
        listing.push_str(&format!("- {}: {}\n", product.name, product.about));
    }
    let session = plan
        .filter(|plan| plan.session)
        .and_then(|plan| plan.repo.as_ref());
    let prompt = if let Some((repo, path)) = session {
        let files = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["ls-files"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default();
        let files: String = files
            .lines()
            .filter(|file| !file.starts_with("evidence/"))
            .take(300)
            .collect::<Vec<_>>()
            .join("\n");
        let about: String = fs::read_to_string(path.join("README.md"))
            .unwrap_or_default()
            .chars()
            .take(4000)
            .collect();
        format!(
            "{SESSION_CONTRACT}\n\nREPOSITORY {repo}, its files:\n{files}\n\nBEGINNING OF ITS README:\n{about}\n\nREQUEST ({}):\n{decision}",
            record_path.display()
        )
    } else {
        format!(
            "{CONTRACT}{LOOP}{listing}\n\nDECISION RECORD ({}):\n{decision}\nBRIEF OF THAT JOB:\n{}",
            record_path.display(),
            if origin_brief.is_empty() {
                "(none)"
            } else {
                &origin_brief
            },
        )
    };
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let workspace = scratch.join("worker");
    fs::create_dir(&workspace)?;
    // The plan names who writes the gate. Without a plan it is the standard worker.
    let author = plan
        .map(|plan| plan.gate.as_str())
        .filter(|name| *name != crate::orchestrator::STANDARD)
        .map(|name| crate::crew::load(&state, name))
        .transpose()?;
    // The author of a session gate reads the repository, when its harness has file tools. A gate
    // written blind checks against names and layouts that do not exist.
    let reads = session.is_some()
        && author
            .as_ref()
            .is_none_or(|agent| agent.harness == crate::crew::Harness::Claude);
    let (answer, worker) = match &author {
        _ if reads => (
            crate::session::read_and_answer(
                &state,
                &session.expect("a session plan names its repository").1,
                author.as_ref(),
                &format!(
                    "{prompt}\n\nYou are in a copy of the repository and can read it with Read, Glob and Grep. Read the code this job touches before you write the gate. Check against the real paths, file formats and commands you find there, never against ones you assume."
                ),
            ),
            "standard-reader",
        ),
        Some(agent) => (
            source::call_agent(
                agent,
                &workspace,
                job,
                &prompt,
                &mut Progress::silent(),
                |_| {},
            ),
            "named-agent",
        ),
        None => source::ask(&workspace, job, &prompt, &mut Progress::silent()),
    };
    let worker = author.as_ref().map_or(worker, |agent| agent.name.as_str());
    // A named gate author whose call fails is marked, so that the orchestrator passes it over for a while.
    if let (Err(error), Some(agent)) = (&answer, &author) {
        crate::orchestrator::mark_gate_author(&state, &agent.name, Some(&error.to_string()));
    }
    let (answer, usage) = answer?;
    // An optional first line names the open intent that the worker took up.
    let mut answer = answer.trim_start().to_owned();
    let mut intent_from = String::new();
    if let Some(rest) = answer.strip_prefix("INTENT:") {
        let (number, tail) = rest.split_once('\n').unwrap_or((rest, ""));
        if let Some((path, _)) = number
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|number| intents.get(number.wrapping_sub(1)))
        {
            intent_from = format!("intent_from={path}\n");
        }
        answer = tail.trim_start().to_owned();
    }
    // The reflection is kept beside the record it came from, with its cost.
    let note = |outcome: &str| {
        record::publish_text(
            &job.join(format!("reflection-{name}.run")),
            &format!(
                "decision=observed\nidentity=reflection-{name}\nderived_from={}\n{intent_from}worker={worker}\n{usage}outcome={}\nend\n",
                record_path.display(),
                source::single_line(outcome),
            ),
            record::MAX_RECORD_BYTES,
        )
    };
    if let Some(reason) = answer.trim().strip_prefix("STOP:") {
        note(&format!("stop: {}", reason.trim()))?;
        return Ok(None);
    }
    // A job on a product takes the product's gate and starts from its accepted source.
    let (brief, gate, seed) = if let Some(rest) = answer.strip_prefix("=== product ") {
        let (head, text) = rest
            .split_once('\n')
            .ok_or("a product proposal without a brief")?;
        let wanted = head.trim().trim_end_matches('=').trim();
        let product = products
            .iter()
            .find(|product| product.name == wanted)
            .ok_or("the worker named an unknown product")?;
        (
            format!("{}\n", text.trim()),
            record::read_regular(&product.gate, source::LIMIT)?,
            Some((
                product.name.clone(),
                record::read_regular(&product.seed, source::LIMIT)?,
            )),
        )
    } else if session.is_some() {
        let (brief, gate) =
            split(&answer, "gate.sh").ok_or("the worker did not return brief.md and gate.sh")?;
        (brief, gate, None)
    } else {
        let (brief, gate) = split(&answer, "verify.mjs")
            .ok_or("the worker did not return brief.md and verify.mjs")?;
        (brief, gate, None)
    };
    source::intent(&brief)?;
    if brief.len() > record::MAX_RECORD_BYTES
        || gate.trim().is_empty()
        || gate.len() > source::LIMIT
    {
        return Err("the proposed brief or gate exceeds its size limit".into());
    }
    let check = scratch.join("check");
    fs::create_dir(&check)?;
    let gate_name = if session.is_some() {
        "gate.sh"
    } else {
        "verify.mjs"
    };
    fs::write(check.join(gate_name), &gate)?;
    let (program, flag) = if session.is_some() {
        ("/bin/sh", "-n")
    } else {
        ("node", "--check")
    };
    let syntax = source::run_child(
        source::sandbox(&check, job, program, false)?.args([flag, gate_name]),
        CHECK_TIMEOUT,
        |_| {},
    )?;
    if !syntax.status.success() {
        return Err("the proposed gate has a syntax error".into());
    }
    fs::create_dir_all(&target)?;
    record::publish_text(&target.join("brief.md"), &brief, record::MAX_RECORD_BYTES)?;
    record::publish_text(&target.join(gate_name), &gate, source::LIMIT)?;
    if let Some(plan) = plan {
        record::publish_text(
            &target.join("plan.txt"),
            &plan.text(),
            record::MAX_RECORD_BYTES,
        )?;
    }
    let mut product_line = String::new();
    if let Some((product, source_text)) = &seed {
        record::publish_text(&target.join("seed.html"), source_text, source::LIMIT)?;
        product_line = format!("product={product}\n");
    }
    record::publish_text(
        &target.join("origin.txt"),
        &format!(
            "derived_from={}\n{intent_from}{product_line}proposed_by={worker}\nstate=waiting for approval\n",
            record_path.display()
        ),
        record::MAX_RECORD_BYTES,
    )?;
    note(&format!("proposal {} waits for approval", target.display()))?;
    if let Some(agent) = &author {
        crate::orchestrator::mark_gate_author(&state, &agent.name, None);
    }
    Ok(Some(target))
}

const DAILY_REFLECTIONS: usize = 3;
pub const AUTO: &str = "TAIK_AUTO_PROPOSE";

// After a job reached its final decision the service asks once what follows from it.
pub fn reflect(state: &Path, jobs: &Path, job: &Path) -> Result<()> {
    let recent = |path: &Path| {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < Duration::from_secs(24 * 60 * 60))
    };
    let reflections = fs::read_dir(jobs)?
        .flatten()
        .filter_map(|entry| fs::read_dir(entry.path()).ok())
        .flat_map(|files| files.flatten())
        .filter(|file| {
            file.file_name()
                .to_string_lossy()
                .starts_with("reflection-")
                && recent(&file.path())
        })
        .count();
    if reflections >= DAILY_REFLECTIONS {
        return Ok(());
    }
    let label = job
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("job-"))
        .ok_or("job without a name")?;
    let name = format!("{}-next", &label[..label.len().min(27)]);
    let last = (1..=4)
        .rev()
        .map(|number| job.join(format!("round-{number:02}.run")))
        .find(|path| path.is_file())
        .ok_or("job without a round record")?;
    propose(state, &name, &last).map(|_| ())
}

pub fn approve(state: &Path, name: &str) -> Result<PathBuf> {
    let name = crate::worker_name(name).ok_or("invalid proposal name")?;
    let proposal = state.canonicalize()?.join("proposals").join(name);
    let plan = crate::orchestrator::Plan::read(&proposal.join("plan.txt"));
    let worker = plan
        .as_ref()
        .map(|plan| plan.work.as_str())
        .filter(|name| *name != crate::orchestrator::STANDARD);
    let said = crate::talk::read(&proposal);
    let job = if let Some((_, repo)) = plan
        .as_ref()
        .filter(|plan| plan.session)
        .and_then(|plan| plan.repo.as_ref())
    {
        crate::session::queue(
            state,
            name,
            &proposal.join("brief.md"),
            &proposal.join("gate.sh"),
            Some(repo),
            worker,
            &said,
        )?
    } else {
        service::queue_source(
            state,
            name,
            &proposal.join("brief.md"),
            &proposal.join("verify.mjs"),
            Some(proposal.join("seed.html"))
                .filter(|seed| seed.is_file())
                .as_deref(),
            worker,
            // What the user said about the proposal starts the conversation of the job.
            &said,
        )?
    };
    record::publish_text(
        &proposal.join("approved.txt"),
        &format!("job={}\n", job.display()),
        record::MAX_RECORD_BYTES,
    )?;
    Ok(job)
}

fn split(answer: &str, gate: &str) -> Option<(String, String)> {
    let rest = answer.trim_start().strip_prefix("=== brief.md ===\n")?;
    let (brief, gate) = rest.split_once(&format!("\n=== {gate} ===\n"))?;
    Some((format!("{}\n", brief.trim()), format!("{}\n", gate.trim())))
}
