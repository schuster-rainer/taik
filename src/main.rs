#[cfg(target_os = "macos")]
mod activation;
#[cfg(target_os = "macos")]
mod crew;
mod experiment;
mod procedure;
mod progress;
#[cfg(target_os = "macos")]
mod promote;
#[cfg(target_os = "macos")]
mod proposal;
#[cfg(unix)]
mod queue_events;
mod record;
#[cfg(unix)]
mod research;
#[cfg(target_os = "macos")]
mod restart_probe;
#[cfg(unix)]
mod self_improve;
#[cfg(unix)]
mod service;
#[cfg(target_os = "macos")]
mod source;
#[cfg(target_os = "macos")]
mod talk;

use std::env;
use std::error::Error;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use procedure::{Procedure, Search, SearchTable};
use progress::Progress;
use record::{Accepted, MAX_RECORD_BYTES};

const MAX_ADDITIONS: usize = 8;
const MAX_TAG_BYTES: usize = 64;
const MAX_TAGS: usize = 32;
const PROBE: &[&str] = &["  OAK ", "oak", "\tELM", " "];
const EXPECTED_PROBE: &[&str] = &["elm", "oak"];
const USAGE: &str = "usage: taik-seed seed OUTPUT.run
       taik-seed extend BASELINE.run OUTPUT.run TAG [TAG ...]
       taik-seed reject|refine BASELINE.run OUTPUT.run
       taik-seed evolve BASELINE.run OUTPUT.run
       taik-seed evaluate BASELINE.run OUTPUT.run ORDER
       taik-seed source JOB BRIEF GATE [--attached[=WORKER]]
       taik-seed promote JOB BRIEF GATE INSTALLED_BINARY WORKER
       taik-seed crew STATE
       taik-seed reply STATE JOB < MESSAGE
       taik-seed propose STATE NAME RECORD.run
       taik-seed approve STATE NAME
       taik-seed experiment BASELINE.run OUTPUT_DIRECTORY
       taik-seed queue STATE BASELINE.run
       taik-seed queue-source STATE NAME BRIEF GATE [ACCEPTED_SOURCE] [--agent=NAME]
       taik-seed queue-self STATE
       taik-seed drain|serve STATE\n       taik-seed serve STATE --reflect";

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// The record names whoever supplied the source. An unnamed attachment stays "attached".
fn attached_worker(flag: &std::ffi::OsStr) -> Option<&str> {
    let flag = flag.to_str()?.strip_prefix("--attached")?;
    if flag.is_empty() {
        Some("attached")
    } else {
        worker_name(flag.strip_prefix('=')?)
    }
}

fn worker_name(name: &str) -> Option<&str> {
    let valid = !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    valid.then_some(name)
}

fn valid_input(tag: &str) -> bool {
    tag.len() <= MAX_TAG_BYTES
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b' ' | b'\t'))
        && !tag.trim().bytes().any(|byte| byte.is_ascii_whitespace())
}

// Independent of the candidate order and of its deduplication implementation.
fn expected_tags(input: &[&str]) -> Vec<String> {
    input
        .iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

struct Trial {
    candidate: Procedure,
    additions: Vec<String>,
    intent: &'static str,
    operations: &'static str,
    change: Change,
    witness: String,
}

#[derive(Clone, Copy)]
enum Change {
    Artifact,
    Procedure,
    Retain,
}

#[derive(Clone, Copy)]
pub struct Outcome {
    pub accepted: bool,
    pub advance: bool,
}

fn assess(baseline: Option<(&Path, &Accepted)>, output: &Path, trial: Trial) -> Result<Outcome> {
    assess_progress(baseline, output, trial, &mut Progress::silent())
}

fn assess_progress(
    baseline: Option<(&Path, &Accepted)>,
    output: &Path,
    trial: Trial,
    progress: &mut Progress,
) -> Result<Outcome> {
    let start = Instant::now();
    let retained = baseline.map_or(&[][..], |(_, accepted)| accepted.tags.as_slice());
    let input: Vec<_> = retained
        .iter()
        .chain(&trial.additions)
        .map(String::as_str)
        .collect();
    progress.event("evaluate", format_args!("{}", trial.candidate));
    let result = trial.candidate.apply(&input);
    let expected = expected_tags(&input);
    let (probe, probe_ok) = trial.candidate.probes();
    let artifact_ok = result.tags == expected;
    let size_ok = result.tags.len() <= MAX_TAGS;
    let order_ok = trial.candidate.valid_order();
    progress.event(
        "checks",
        format_args!(
            "artifact:{artifact_ok}, probes:{probe_ok}, limit:{size_ok}, order:{order_ok}"
        ),
    );
    let mut comparison = String::new();
    let mut improvement_ok = true;
    if matches!(trial.change, Change::Procedure) {
        let previous = &baseline.ok_or("missing procedure baseline")?.1.procedure;
        let (previous_probe, previous_ok) = previous.probes();
        let relocation = previous.is_relocation(&trial.candidate);
        improvement_ok = previous_ok && relocation && probe.work.improves_on(previous_probe.work);
        progress.event(
            "compare",
            format_args!("{} → {}", previous_probe.work, probe.work),
        );
        comparison = format!(
            "baseline_work={}\nbaseline_probes_match={previous_ok}\none_step_relocated={relocation}\nrefinement_check={improvement_ok}\n",
            previous_probe.work,
        );
    }
    let outcome = Outcome {
        accepted: artifact_ok && probe_ok && size_ok && order_ok && improvement_ok,
        advance: false,
    };
    let outcome = Outcome {
        advance: outcome.accepted && matches!(trial.change, Change::Procedure),
        ..outcome
    };
    let next = match (outcome.accepted, trial.change) {
        (false, _) => format!(
            "Stop and preserve the previous accepted record. Failed checks: {}.",
            [(artifact_ok, "canonical artifact"), (probe_ok, "independent probes"),
             (size_ok, "tag limit"), (order_ok, "procedure dependencies"),
             (improvement_ok, "one-step work comparison")]
                .into_iter().filter_map(|(passed, label)| (!passed).then_some(label))
                .collect::<Vec<_>>().join(", "),
        ),
        (true, Change::Procedure) => format!(
            "Reuse the accepted procedure in a new bounded iteration to see whether work can fall below {}.",
            probe.work,
        ),
        (true, Change::Retain) => {
            "Stop proposing and retain the verified procedure. No single relocation reduced probe work within the trust boundary.".to_owned()
        }
        (true, Change::Artifact) => format!(
            "Stop at the one-candidate limit. Verified {} canonical tags from {} input tags. The retained procedure passed both probes.",
            result.tags.len(), input.len(),
        ),
    };
    let decision = if outcome.accepted {
        "accepted"
    } else {
        "rejected"
    };
    let limitations = if matches!(trial.change, Change::Artifact) {
        "This invocation checked canonical ASCII output and both independent probes."
    } else {
        "The work counters describe the probe only. The gate gives no end-to-end speed guarantee."
    };
    let baseline_path = baseline.map(|(path, _)| path);
    let baseline_procedure = baseline.map(|(_, record)| record.procedure.to_string());
    let output_name = output.file_name().unwrap_or_default();
    let record = format!(
        "decision={decision}\nprocedure={}\ntags={}\nidentity={output_name:?}\n\
         intent={}\nbaseline={baseline_path:?}\nbaseline_procedure={baseline_procedure:?}\noperations={}\n\
         scope=canonical ASCII tags and a local, create-only record\n\
         limits={MAX_ADDITIONS} additions, {MAX_TAG_BYTES} bytes/tag, {MAX_TAGS} retained tags, {MAX_RECORD_BYTES} bytes/record; one evaluated candidate\n\
         additions={:?}\nexpected_tags={expected:?}\nprobe_input={PROBE:?}\nprobe_tags={:?}\nprobe_expected={EXPECTED_PROBE:?}\n\
         checks=artifact_matches:{artifact_ok}, probe_matches:{probe_ok}, size_within_limit:{size_ok}\norder_preserves_contract={order_ok}\n\
         artifact_work={}\nprobe_work={}\n{comparison}{}evaluation_elapsed_us={}\n\
         limitations={limitations}\ncontinuation={next}\nadvance={}\nend\n",
        trial.candidate,
        result.tags.join(","),
        trial.intent,
        trial.operations,
        trial.additions,
        probe.tags,
        result.work,
        probe.work,
        trial.witness,
        start.elapsed().as_micros(),
        outcome.advance,
    );
    emit(output, &record)?;
    let event = match (outcome.accepted, outcome.advance) {
        (true, true) => "accepted",
        (true, false) => "retained",
        (false, _) => "rejected",
    };
    progress.event(event, format_args!("{next}"));
    Ok(outcome)
}

fn emit(output: &Path, record: &str) -> Result<()> {
    record::publish(output, record)?;
    // The durable decision is available even when the consumer closes stdout.
    use std::io::Write;
    let _ = std::io::stdout().lock().write_all(record.as_bytes());
    Ok(())
}

pub fn evolve(baseline_path: &Path, output: &Path) -> Result<Outcome> {
    evolve_progress(
        baseline_path,
        output,
        Search::All,
        None,
        &mut Progress::silent(),
    )
}

fn evolve_progress(
    baseline_path: &Path,
    output: &Path,
    search: Search,
    table: Option<&SearchTable>,
    progress: &mut Progress,
) -> Result<Outcome> {
    progress.event("start", format_args!("Single-step relocation search"));
    record::check_output(output)?;
    let baseline = record::read_accepted(baseline_path)?;
    progress.event("baseline", format_args!("{}", baseline.procedure));
    let started = Instant::now();
    let (planned, [built, probes], policy) =
        match table.and_then(|table| table.lookup(&baseline.procedure)) {
            Some(decision) => {
                progress.event("lookup", format_args!("Reused a verified search decision"));
                (decision.clone(), [0, 0], "precomputed")
            }
            None => {
                let (candidate, counts) = baseline.procedure.search(search, progress);
                (candidate, counts, search.name())
            }
        };
    let witness = format!(
        "worker=local\nsearch_policy={policy}\nsearch_candidates_built={built}\nsearch_probe_executions={probes}\nsearch_limit=20 one-step relocations\nsearch_elapsed_us={}\n",
        started.elapsed().as_micros(),
    );
    evaluate(baseline_path, output, &baseline, planned, witness, progress)
}

fn evaluate(
    baseline_path: &Path,
    output: &Path,
    baseline: &Accepted,
    planned: Option<Procedure>,
    witness: String,
    progress: &mut Progress,
) -> Result<Outcome> {
    let (candidate, operations, change) = match planned {
        Some(candidate) => (
            candidate,
            "Relocate one step of the retained executable procedure and compare both results",
            Change::Procedure,
        ),
        None => (
            baseline.procedure.clone(),
            "Explore bounded relocations and recheck the retained executable procedure",
            Change::Retain,
        ),
    };
    assess_progress(
        Some((baseline_path, baseline)),
        output,
        Trial {
            candidate,
            additions: Vec::new(),
            intent: "Improve the retained procedure from measured work",
            operations,
            change,
            witness,
        },
        progress,
    )
}

fn run() -> Result<bool> {
    let args: Vec<_> = env::args_os().skip(1).take(MAX_ADDITIONS + 4).collect();
    let command = args.first().and_then(|name| name.to_str()).ok_or(USAGE)?;
    let first = args.get(1).map(Path::new).ok_or(USAGE)?;
    let rest = &args[2..];
    match command {
        "seed" if rest.is_empty() => {
            record::check_output(first)?;
            Ok(assess(None, first, Trial {
                candidate: Procedure::original(),
                additions: vec!["  CEDAR ".to_owned(), "cedar".to_owned(), "\tASH".to_owned()],
                intent: "Create a canonical tag list and retain its procedure",
                operations: "Add raw tags, sort canonical forms, reduce duplicates, and inspect the result and probes",
                change: Change::Artifact, witness: String::new(),
            })?.accepted)
        }
        "evolve" if rest.len() == 1 => Ok(evolve(first, Path::new(&rest[0]))?.accepted),
        "experiment" if rest.len() == 1 => {
            experiment::run(first, Path::new(&rest[0]))?;
            Ok(true)
        }
        #[cfg(target_os = "macos")]
        "source"
            if matches!(rest, [_, _])
                || matches!(rest, [_, _, flag] if attached_worker(flag).is_some()) =>
        {
            source::run(
                first,
                Path::new(&rest[0]),
                Path::new(&rest[1]),
                rest.get(2).and_then(|flag| attached_worker(flag)),
            )
        }
        #[cfg(target_os = "macos")]
        "reply" if rest.len() == 1 => {
            use std::io::Read;
            let name = rest[0].to_str().ok_or("job name must be ASCII")?;
            let mut text = String::new();
            std::io::stdin()
                .take(talk::MAX_MESSAGE_BYTES as u64 + 1)
                .read_to_string(&mut text)?;
            println!("{}", service::reply(first, name, &text)?.display());
            Ok(true)
        }
        #[cfg(target_os = "macos")]
        "crew" if rest.is_empty() => {
            println!("{}", crew::publish_overview(first)?);
            Ok(true)
        }
        #[cfg(target_os = "macos")]
        "propose" if rest.len() == 2 => {
            let name = rest[0].to_str().ok_or("proposal name must be ASCII")?;
            match proposal::propose(first, name, Path::new(&rest[1]))? {
                Some(path) => println!("{}", path.display()),
                None => println!("stop"),
            }
            Ok(true)
        }
        #[cfg(target_os = "macos")]
        "approve" if rest.len() == 1 => {
            let name = rest[0].to_str().ok_or("proposal name must be ASCII")?;
            println!("{}", proposal::approve(first, name)?.display());
            Ok(true)
        }
        #[cfg(target_os = "macos")]
        "promote" if rest.len() == 4 => promote::run(
            first,
            Path::new(&rest[0]),
            Path::new(&rest[1]),
            Path::new(&rest[2]),
            rest[3]
                .to_str()
                .and_then(worker_name)
                .ok_or("invalid worker name")?,
        ),
        "evaluate" if rest.len() == 2 => {
            let output = Path::new(&rest[0]);
            record::check_output(output)?;
            let baseline = record::read_accepted(first)?;
            let order = rest[1]
                .to_str()
                .and_then(Procedure::parse)
                .ok_or("invalid procedure order")?;
            Ok(evaluate(
                first,
                output,
                &baseline,
                Some(order),
                "worker=operator\n".to_owned(),
                &mut Progress::silent(),
            )?
            .accepted)
        }
        #[cfg(unix)]
        "queue" if rest.len() == 1 => {
            println!("{}", service::queue(first, Path::new(&rest[0]))?.display());
            Ok(true)
        }
        #[cfg(target_os = "macos")]
        "queue-source" if (3..=5).contains(&rest.len()) => {
            let name = rest[0].to_str().ok_or("job name must be ASCII")?;
            // An optional last argument --agent=NAME names the agent that works the job.
            let agent = rest
                .last()
                .and_then(|last| last.to_str())
                .and_then(|last| last.strip_prefix("--agent="));
            let plain = &rest[..rest.len() - usize::from(agent.is_some())];
            if !(3..=4).contains(&plain.len()) {
                return Err(USAGE.into());
            }
            println!(
                "{}",
                service::queue_source(
                    first,
                    name,
                    Path::new(&plain[1]),
                    Path::new(&plain[2]),
                    plain.get(3).map(Path::new),
                    agent,
                )?
                .display()
            );
            Ok(true)
        }
        #[cfg(unix)]
        "queue-self" if rest.is_empty() => {
            println!("{}", service::queue_self(first)?.display());
            Ok(true)
        }
        #[cfg(unix)]
        "drain" | "serve" if rest.is_empty() => service::run(first, command == "serve", false),
        #[cfg(target_os = "macos")]
        "serve" if rest == ["--reflect"] => service::run(first, true, true),
        "extend" | "reject" | "refine" if !rest.is_empty() => {
            let output = Path::new(&rest[0]);
            record::check_output(output)?;
            let baseline = record::read_accepted(first)?;
            let (candidate, intent, operations, change) = match command {
                "reject" if rest.len() == 1 => (
                    baseline.procedure.without_lower(),
                    "Check whether removing lowercasing preserves canonical tags",
                    "Remove lowercasing and test the result against the independent contract",
                    Change::Procedure,
                ),
                "refine" if rest.len() == 1 => (
                    baseline
                        .procedure
                        .trim_first()
                        .ok_or("refine requires a lower-then-trim baseline")?,
                    "Reduce bytes submitted to lowercase while preserving canonical tags",
                    "Move trimming ahead of lowercasing and compare work on the same probe",
                    Change::Procedure,
                ),
                "extend" if (2..=MAX_ADDITIONS + 1).contains(&rest.len()) => (
                    baseline.procedure.clone(),
                    "Add tags using the retained procedure",
                    "Reuse the retained tags and executable procedure, add tags, sort, reduce and check output",
                    Change::Artifact,
                ),
                _ => return Err(USAGE.into()),
            };
            let additions = rest[1..].iter().map(|tag| {
                tag.to_str().filter(|tag| valid_input(tag)).map(str::to_owned)
                    .ok_or("tags must be ASCII letters, digits or hyphens with optional edge whitespace and fit in 64 bytes")
            }).collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(assess(
                Some((first, &baseline)),
                output,
                Trial {
                    candidate,
                    additions,
                    intent,
                    operations,
                    change,
                    witness: String::new(),
                },
            )?
            .accepted)
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
