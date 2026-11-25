use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::{Progress, Result, activation, emit, record, research, self_improve, service, source};

pub const JOB: &str = "job-suchtabelle-kaltstart";
const INTENT: &str =
    "Wiederanlauf mit einer fehlerhaften Suchtabelle am echten Factory-Prozess prüfen";
const DECISIONS: &str = "retained_search_decisions=";
const TIMEOUT: Duration = Duration::from_secs(10);

pub fn reflect(jobs: &Path) -> Result<()> {
    let job = jobs.join(JOB);
    let output = job.join("round-01.run");
    if output.try_exists()? {
        return Ok(());
    }

    let table = record::read_regular(
        &jobs.join(research::JOB).join("round-01.run"),
        record::MAX_RECORD_BYTES,
    )?;
    let damaged = corrupt(&table)?;
    fs::create_dir_all(&job)?;
    let request = job.join("request.run");
    if !request.try_exists()? {
        record::publish(
            &request,
            &format!(
                "decision=accepted\nkind=search\nprocedure=valid-unique\ntags=\nintent={INTENT}\n\
             evidence=../{}/round-02.run\nobservation=The existing decision was based on simulated restarts.\nend\n",
                activation::JOB
            ),
        )?;
    }
    let mut progress = Progress::for_round(&output);
    progress.event("start", format_args!("{INTENT}"));
    let execution = exercise(jobs, &table, &damaged, &mut progress);
    let (decision, checks, next) = match execution {
        Ok(checks) => (
            "accepted",
            checks,
            "Keep the deferred verification and the original search as a fallback. Stop this experiment after both controls passed.",
        ),
        Err(error) => (
            "rejected",
            source::single_line(&error.to_string()),
            "Preserve live data. Inspect the failed process observation before changing the policy.",
        ),
    };
    progress.event("checks", format_args!("{checks}"));
    emit(
        &output,
        &format!(
            "decision={decision}\nkind=search\nprocedure=valid-unique\ntags=\nidentity=round-01\nintent={INTENT}\n\
         evidence=request.run\nworker=local\nbaseline=../{}/round-02.run\n\
         operations=Run the current executable against fresh isolated state with a verified table and a different well-formed search decision.\n\
         checks={checks}\nlimits=Four local process runs, at most 10 seconds each.\n\
         limitations=One accepted input and one incorrect but well-formed decision in isolated state. No change to the live table.\n\
         continuation={next}\nadvance=false\nend\n",
            activation::JOB,
        ),
    )?;
    progress.event(decision, format_args!("{next}"));
    Ok(())
}

fn exercise(jobs: &Path, table: &str, damaged: &str, progress: &mut Progress) -> Result<String> {
    let scratch = tempfile::tempdir()?;
    let healthy = scratch.path().join("healthy");
    let recovery = scratch.path().join("recovery");
    let seed = scratch.path().join("seed.run");
    run("seed", &seed)?;
    prepare(jobs, &healthy, table)?;
    prepare(jobs, &recovery, damaged)?;

    let cold = run("drain", &recovery)?;
    if cold.contains("retained search decisions fail the current reference check") {
        return Err("cold restart opened an unused table".into());
    }
    progress.event(
        "probe",
        format_args!("Kaltstart mit beschädigter Kopie und leerer Warteschlange abgeschlossen"),
    );

    let healthy_job = service::queue(&healthy, &seed)?;
    let recovery_job = service::queue(&recovery, &seed)?;
    run("drain", &healthy)?;
    run("drain", &recovery)?;

    let healthy_first = read_round(&healthy_job, 1)?;
    let recovery_first = read_round(&recovery_job, 1)?;
    let recovery_trace = record::read_regular(&recovery_job.join("round-01.progress"), 32_768)?;
    if !healthy_first
        .lines()
        .any(|line| line == "search_policy=precomputed")
        || !recovery_first
            .lines()
            .any(|line| line == "search_policy=valid-unique")
        || !recovery_trace.contains("retained search decisions fail the current reference check")
    {
        return Err("the live process did not distinguish verified reuse from fallback".into());
    }
    let good = terminal(&healthy_job)?;
    let recovered = terminal(&recovery_job)?;
    if good.tags != recovered.tags || good.procedure != recovered.procedure {
        return Err("the fallback changed the verified terminal tags or procedure".into());
    }
    Ok(format!(
        "Cold startup skipped an unused table; valid table used; varied decision rejected before lookup; fallback and control ended with tags={} and procedure={}",
        recovered.tags.join(","),
        recovered.procedure
    ))
}

fn prepare(source: &Path, state: &Path, table: &str) -> Result<()> {
    let jobs = state.join("jobs");
    for (name, files) in [
        (
            self_improve::JOB,
            &[
                "request.run",
                "round-01.run",
                "round-01.policy",
                "round-02.run",
                "round-02.policy",
                "round-03.run",
            ][..],
        ),
        (activation::JOB, &["round-02.run"][..]),
    ] {
        let target = jobs.join(name);
        fs::create_dir_all(&target)?;
        for file in files {
            let contents =
                record::read_regular(&source.join(name).join(file), record::MAX_RECORD_BYTES)?;
            record::publish(&target.join(file), &contents)?;
        }
    }
    let destination = jobs.join(research::JOB);
    fs::create_dir_all(&destination)?;
    record::publish(&destination.join("round-01.run"), table)
}

fn corrupt(table: &str) -> Result<String> {
    let line = table
        .lines()
        .find_map(|line| line.strip_prefix(DECISIONS))
        .ok_or("missing retained decisions")?;
    let (first, rest) = line.split_once(';').ok_or("missing remaining decisions")?;
    let (baseline, chosen) = first.split_once('>').ok_or("broken decision row")?;
    if chosen == "-" {
        return Err("first decision already retains its current procedure".into());
    }
    Ok(table.replacen(
        &format!("{DECISIONS}{line}"),
        &format!("{DECISIONS}{baseline}>-;{rest}"),
        1,
    ))
}

fn run(command: &str, path: &Path) -> Result<String> {
    let mut process = Command::new(std::env::current_exe()?);
    let result = source::run_child(process.arg(command).arg(path), TIMEOUT, |_| {})?;
    if !result.status.success() {
        return Err(format!(
            "{command} exited with {}: {}",
            result.status,
            source::single_line(&result.text())
        )
        .into());
    }
    Ok(result.text())
}

fn read_round(job: &Path, round: usize) -> Result<String> {
    record::read_regular(
        &job.join(format!("round-{round:02}.run")),
        record::MAX_RECORD_BYTES,
    )
}

fn terminal(job: &Path) -> Result<record::Accepted> {
    for round in 1..=20 {
        let verdict = read_round(job, round)?;
        if verdict.lines().any(|line| line == "advance=false") {
            return record::parse_accepted(&verdict);
        }
    }
    Err("verification ran out of tag rounds".into())
}
