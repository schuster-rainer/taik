use std::fs;
use std::path::Path;

use crate::procedure::{Search, SearchTable};
use crate::{Progress, Result, emit, record};

pub const JOB: &str = "job-suchentscheidungen-wiederverwenden";
const GOAL: &str =
    "Wiederholte Suchentscheidungen aus gemessener Prüfleistung ableiten und wiederverwenden";
const EVIDENCE: &str = "../job-selbstverbesserung/round-02.run";

pub fn drive(jobs: &Path, policy: Search) -> Result<Option<SearchTable>> {
    if policy != Search::ValidUnique {
        return Ok(None);
    }

    let job = jobs.join(JOB);
    let output = job.join("round-01.run");
    if output.try_exists()? {
        return resume(&output);
    }

    // The original policy is certified. Measure the work it still repeats on fixed inputs.
    let (candidate, probes) = SearchTable::measure(policy);
    if probes == 0 {
        return Ok(None);
    }

    fs::create_dir_all(&job)?;
    let request = job.join("request.run");
    if !request.try_exists()? {
        let request_record = format!(
            "decision=accepted\nkind=search\nprocedure={}\ntags=\nintent={GOAL}\n\
             evidence={EVIDENCE}\nobserved_candidate_probes={probes}\nend\n",
            policy.name(),
        );
        record::publish(&request, &request_record)?;
    }
    let mut progress = Progress::for_round(&output);
    progress.event("start", format_args!("{GOAL}"));
    progress.event(
        "probe",
        format_args!("{probes} wiederholbare Kandidatenprüfungen im gesamten Suchraum"),
    );

    let (passed, reference_probes) = candidate.verify();
    let decision = if passed { "accepted" } else { "rejected" };
    let (selected, continuation) = if passed {
        (
            "precomputed",
            "Use the retained search decisions in subsequent tag jobs. No further repeated candidate probes remain in this search space.",
        )
    } else {
        (
            policy.name(),
            "Retain the verified search policy. The proposal differed from the exhaustive reference.",
        )
    };
    let checks = format!("all seven admitted orders compared with exhaustive search: {passed}");
    let retained = if passed {
        candidate.encode()
    } else {
        String::new()
    };
    progress.event("checks", format_args!("{checks}"));
    let report = format!(
        "decision={decision}\nkind=search\nprocedure={selected}\ntags=\nidentity=round-01\nintent={GOAL}\n\
         baseline_policy={}\npolicy={selected}\nevidence=request.run\n\
         operations=Retain one measured decision per admissible procedure and verify each against exhaustive search.\n\
         checks={checks}\ndiscovery_candidate_probes={probes}\nreference_candidate_probes={reference_probes}\n\
         retained_search_decisions={retained}\n\
         limitations=One bounded tag search; the reference runs again after service restart; per-round artifact checks still execute; elapsed speed was not measured.\n\
         continuation={continuation}\nadvance=false\nend\n",
        policy.name(),
    );
    emit(&output, &report)?;
    progress.event(decision, format_args!("{continuation}"));
    Ok(passed.then_some(candidate))
}

fn resume(output: &Path) -> Result<Option<SearchTable>> {
    let saved = record::read_regular(output, record::MAX_RECORD_BYTES)?;
    if saved.starts_with("decision=rejected\n") && saved.ends_with("end\n") {
        return Ok(None);
    }
    if !saved.starts_with("decision=accepted\nkind=search\n") || !saved.ends_with("end\n") {
        return Err("incomplete research decision".into());
    }
    let table = saved
        .lines()
        .find_map(|line| line.strip_prefix("retained_search_decisions="))
        .and_then(SearchTable::decode)
        .ok_or("research result is missing its retained search decisions")?;
    if !table.verify().0 {
        return Err("retained search decisions fail the current reference check".into());
    }
    Ok(Some(table))
}
