use std::path::Path;

use crate::procedure::{Search, search_gate};
use crate::{Progress, Result, emit, record};

pub const JOB: &str = "job-selbstverbesserung";
pub const REQUEST: &str = "decision=accepted\nkind=search\nprocedure=all\ntags=\nintent=Reduce repeated and invalid search candidates while preserving the selected procedure.\nend\n";

pub fn drive(job: &Path) -> Result<(Search, bool)> {
    if !job.try_exists()? {
        return Ok((Search::All, true));
    }
    if record::read_regular(&job.join("request.run"), record::MAX_RECORD_BYTES)? != REQUEST {
        return Err("self-improvement request does not match the fixed search scope".into());
    }
    let mut selected = Search::All;
    for number in 1..=3 {
        let stem = format!("round-{number:02}");
        let path = job.join(format!("{stem}.run"));
        let verdict = if path.try_exists()? {
            record::read_regular(&path, record::MAX_RECORD_BYTES)?
        } else {
            reflect(job, &stem, selected)?
        };
        let field = |key| verdict.lines().find_map(|line| line.strip_prefix(key));
        if field("kind=") != Some("search")
            || !verdict.ends_with("end\n")
            || field("baseline_policy=") != Some(selected.name())
        {
            return Err("invalid self-improvement history".into());
        }
        if field("decision=") == Some("rejected") && field("advance=") == Some("false") {
            return Ok((selected, false));
        }
        if field("decision=") != Some("accepted") {
            return Err("missing self-improvement decision".into());
        }
        match field("advance=") {
            Some("false") if selected.next().is_none() => return Ok((selected, true)),
            Some("true") => {
                let candidate = selected.next().ok_or("search is already at its limit")?;
                let artifact = format!("{stem}.policy");
                if field("policy=") != Some(candidate.name())
                    || field("artifact=") != Some(artifact.as_str())
                    || record::read_regular(&job.join(artifact), 64)?.trim() != candidate.name()
                {
                    return Err(
                        "accepted search policy does not match its recorded artifact".into(),
                    );
                }
                selected = candidate;
            }
            _ => return Err("inconsistent self-improvement continuation".into()),
        }
    }
    Err("self-improvement round budget exhausted".into())
}

fn reflect(job: &Path, stem: &str, baseline: Search) -> Result<String> {
    let output = job.join(format!("{stem}.run"));
    let mut progress = Progress::for_round(&output);
    progress.event("start", format_args!("Search policy: {}", baseline.name()));
    let mut artifact = String::new();
    let (candidate, accepted, advance, observed, continuation) = match baseline.next() {
        Some(candidate) => {
            let (accepted, before, after, cases) = search_gate(baseline, candidate);
            let observed = format!(
                "policies agreed with the reference and reduced work: {accepted}; legal orders tested: {cases}; \
                 candidates constructed: {} -> {}; probe executions: {} -> {}",
                before[0], after[0], before[1], after[1]
            );
            progress.event("checks", format_args!("{observed}"));
            if accepted {
                artifact = format!("{stem}.policy");
                let path = job.join(&artifact);
                let policy = format!("{}\n", candidate.name());
                if path.try_exists()? {
                    if record::read_regular(&path, 64)? != policy {
                        return Err(
                            "interrupted policy output differs from the verified candidate".into(),
                        );
                    }
                } else {
                    record::publish(&path, &policy)?;
                }
            }
            let continuation = if accepted {
                "Use the accepted policy in the next round and subsequent tag jobs."
            } else {
                "Keep the prior policy. This candidate failed the fixed search gate."
            };
            (candidate, accepted, accepted, observed, continuation)
        }
        None => (
            baseline,
            true,
            false,
            "Both scoped reductions have already been evaluated.".to_owned(),
            "Keep the accepted search policy. Await new evidence or work.",
        ),
    };
    let decision = if accepted { "accepted" } else { "rejected" };
    let line = format!(
        "decision={decision}\nkind=search\nprocedure={}\ntags=\nidentity={stem}\n\
         intent=Reduce unnecessary work in the Factory's own procedure search\n\
         baseline_policy={}\npolicy={}\nlimits=three rounds; all seven legal orders; fixed probes and promotion criteria\n\
         operations=Compare bounded search candidates against the unrestricted reference search\n\
         checks={observed}\nartifact={artifact}\n\
         limitations=counts cover constructed candidates and probes; elapsed speed was not established\n\
         continuation={continuation}\nadvance={advance}\nend\n",
        if accepted {
            candidate.name()
        } else {
            baseline.name()
        },
        baseline.name(),
        candidate.name(),
    );
    emit(&output, &line)?;
    progress.event(decision, format_args!("{continuation}"));
    Ok(line)
}
