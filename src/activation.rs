use std::fs;
use std::path::Path;

use crate::procedure::{Procedure, Search, SearchTable};
use crate::{Progress, Result, emit, record, source};

pub const JOB: &str = "job-suchtabelle-bei-bedarf";
const INTENT: &str = "Prüfung der Suchtabelle erst bei tatsächlicher Nutzung untersuchen";
const BASELINE: &str = "../job-suchentscheidungen-wiederverwenden/round-01.run";

pub fn accepted(jobs: &Path) -> Result<bool> {
    let path = jobs.join(JOB).join("round-02.run");
    if !path.try_exists()? {
        return Ok(false);
    }
    let decision = record::read_regular(&path, record::MAX_RECORD_BYTES)?;
    Ok(decision.starts_with("decision=accepted\nkind=search\n")
        && decision.ends_with("end\n")
        && decision.lines().any(|line| line == "policy=first-use"))
}

pub fn reflect(jobs: &Path, table: &SearchTable) -> Result<bool> {
    let job = jobs.join(JOB);
    let first = job.join("round-01.run");
    let second = job.join("round-02.run");
    if second.try_exists()? {
        return accepted(jobs);
    }
    if !first.try_exists()? {
        fs::create_dir_all(&job)?;
        let request = job.join("request.run");
        if !request.try_exists()? {
            record::publish(
                &request,
                &format!(
                    "decision=accepted\nkind=search\nprocedure=precomputed\ntags=\nintent={INTENT}\nbaseline={BASELINE}\nend\n"
                ),
            )?;
        }
        let mut progress = Progress::for_round(&first);
        let startup = replay(table, true, &[])?;
        let prompt = format!(
            "Use this observation from the TAIK Factory. An actual replay of loading the verified tag-search table with no tag jobs executed {startup} candidate probes. The table is only used for tag jobs; normal artifact checks are independent. Compare eager startup verification with verifying exactly once before first use. Choose first-use only if there is a concrete savings hypothesis, else stop. Return one JSON object with only policy (first-use or stop) and intent (one German sentence with the evidence and what to check). Never change the gate."
        );
        progress.event(
            "probe",
            format_args!("Neustart ohne Tag-Aufträge: {startup} Kandidatenprüfungen"),
        );
        progress.event(
            "worker",
            format_args!("Codex wertet die beobachtete Startprüfung aus"),
        );
        let workspace = tempfile::tempdir()?;
        let report = source::codex(
            &workspace.path().canonicalize()?,
            &job,
            &prompt,
            &mut progress,
            |_| {},
        )
        .and_then(|text| Ok(serde_json::from_str::<serde_json::Value>(&text)?));
        let (intent, note) = match report {
            Ok(proposal) => match proposal["intent"]
                .as_str()
                .filter(|intent| valid_intent(intent))
            {
                Some(intent) if proposal["policy"] == "first-use" => (
                    Some(intent.to_owned()),
                    "Run the proposed activation on independent service replays.".to_owned(),
                ),
                Some(intent) if proposal["policy"] == "stop" => (None, intent.to_owned()),
                _ => (
                    None,
                    "The worker did not return an admissible hypothesis.".into(),
                ),
            },
            Err(error) => (
                None,
                format!(
                    "Worker unavailable: {}",
                    source::single_line(&error.to_string())
                ),
            ),
        };
        let advance = intent.is_some() && startup > 0;
        let title = intent.as_deref().unwrap_or(INTENT);
        emit(
            &first,
            &format!(
                "decision=accepted\nkind=search\nprocedure=precomputed\ntags=\nintent={title}\n\
             evidence=request.run\nworker=codex-subscription\nstartup_candidate_probes={startup}\n\
             checks=Measured eager verification on a replay with no tag jobs.\n\
             continuation={note}\nadvance={advance}\nend\n"
            ),
        )?;
        progress.event("accepted", format_args!("{note}"));
    }
    let observation = record::read_regular(&first, record::MAX_RECORD_BYTES)?;
    if !observation.starts_with("decision=accepted\nkind=search\n")
        || !observation.ends_with("end\n")
    {
        return Err("incomplete activation observation".into());
    }
    let proposed = observation.lines().any(|line| line == "advance=true");
    let worker_unavailable = observation
        .lines()
        .any(|line| line.starts_with("continuation=Worker unavailable:"));
    let unused_probes = observation
        .lines()
        .find_map(|line| line.strip_prefix("startup_candidate_probes="))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let local_hypothesis = worker_unavailable && unused_probes > 0;
    if !proposed && !local_hypothesis {
        return Ok(false);
    }
    let origin = if local_hypothesis {
        "local: move verified work to the first actual consumer after measuring its cost without a consumer"
    } else {
        "worker proposal from round-01.run"
    };
    let mut progress = Progress::for_round(&second);
    progress.event(
        "start",
        format_args!("Vorschlag auf unabhängigen Abläufen prüfen"),
    );
    let orders: Vec<_> = table.orders().collect();
    let (before, after) = check(table, &orders)?;
    let decision = if after < before {
        "accepted"
    } else {
        "rejected"
    };
    let checks = format!(
        "Same decisions against ordinary search over all admitted orders; candidate probe calls over four service replays: {before} -> {after}."
    );
    progress.event("checks", format_args!("{checks}"));
    emit(
        &second,
        &format!(
            "decision={decision}\nkind=search\nprocedure=precomputed\ntags=\n\
         intent=Prüfe die bedarfsgesteuerte Tabellenprüfung auf neuen Dienstabläufen\n\
         baseline=round-01.run\npolicy=first-use\nproposal={origin}\nchecks={checks}\n\
         limitations=Simulated restart workloads exercising real probe calls. No elapsed-speed claim.\n\
         continuation=Use the accepted timing on future starts. Keep the original gate before first use.\n\
         advance=false\nend\n"
        ),
    )?;
    progress.event(decision, format_args!("Aktivierungsregel geprüft"));
    Ok(after < before)
}

fn valid_intent(intent: &str) -> bool {
    !intent.trim().is_empty() && intent.len() <= 512 && !intent.chars().any(char::is_control)
}

fn check(table: &SearchTable, orders: &[&Procedure]) -> Result<(usize, usize)> {
    if orders.len() != 7 {
        return Err("activation requires all seven admitted orders".into());
    }
    let sessions = [&[][..], &orders[..1], &orders[1..3], orders];
    sessions.iter().try_fold((0, 0), |(before, after), jobs| {
        let eager = replay(table, true, jobs)?;
        let lazy = replay(table, false, jobs)?;
        if lazy > eager {
            return Err("proposal increased checks on a replay".into());
        }
        Ok((before + eager, after + lazy))
    })
}

fn replay(table: &SearchTable, verify_at_start: bool, jobs: &[&Procedure]) -> Result<usize> {
    let mut cost = None;
    if verify_at_start || !jobs.is_empty() {
        let (passed, probes) = table.verify();
        if !passed {
            return Err("retained table failed the original independent gate".into());
        }
        cost = Some(probes);
    }
    for order in jobs {
        let expected = order.search(Search::ValidUnique, &mut Progress::silent()).0;
        if table.lookup(order) != Some(&expected) {
            return Err("proposal changed the selected procedure".into());
        }
    }
    Ok(cost.unwrap_or(0))
}
