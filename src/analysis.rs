//! A short analysis of a rejected round, written for the user who reads the ticket.
//!
//! The worker of the service gets the work order, the pinned gate and the finding, and answers in
//! four labelled lines. The analysis is kept beside the round, create-only like every record.

use std::fs;
use std::path::{Path, PathBuf};

use crate::progress::Progress;
use crate::{Result, record, source};

const MAX_ANALYSIS_BYTES: usize = 1600;
const LABELS: [&str; 4] = [
    "Verlangt:",
    "Gescheitert an:",
    "Ursache:",
    "Nächster Schritt:",
];
const CONTRACT: &str = "A round of a software job was rejected. Explain it to the person who ordered the work. \
Answer in German with exactly these four lines and nothing else, each one sentence of at most 30 words, plain words, no code blocks:\n\
Verlangt: what the failed check demanded\n\
Gescheitert an: what the candidate did instead, as far as the finding shows it\n\
Ursache: the most likely reason. Say whether the fault lies in the candidate, in the work order, in the gate or outside (worker, time, budget)\n\
Nächster Schritt: what should happen now and who acts, the agent or the user\n\
Do not guess beyond the material. If the finding does not show something, say so in that line.";

/// Writes the analysis of the latest round if it was rejected and has none yet.
pub fn of_last_round(job: &Path) -> Result<Option<PathBuf>> {
    let Some(number) = (1..=99)
        .take_while(|number| job.join(format!("round-{number:02}.run")).is_file())
        .last()
    else {
        return Ok(None);
    };
    let stem = format!("round-{number:02}");
    let target = job.join(format!("{stem}.analysis.md"));
    let round = record::read_regular(&job.join(format!("{stem}.run")), record::MAX_RECORD_BYTES)?;
    if !round.starts_with("decision=rejected\n") || target.try_exists()? {
        return Ok(None);
    }
    // The worker delivered nothing. Asking it again would cost time and explain nothing.
    if let Some(failure) = round
        .lines()
        .find_map(|line| line.strip_prefix("worker_failure="))
        .filter(|failure| !failure.is_empty())
    {
        let cause = match failure {
            "stalled" => {
                "Der Agent hat länger als seine Lease nichts ausgegeben und wurde beendet."
            }
            "timeout" => "Der Agent lief bis zu seiner Zeitgrenze und wurde beendet.",
            "budget" => "Das Tagesbudget für Worker war erreicht, der Aufruf fand nicht statt.",
            "output-limit" => "Der Agent hat mehr ausgegeben, als ein Aufruf aufnehmen darf.",
            _ => "Der Aufruf des Agenten ist fehlgeschlagen, bevor ein Kandidat entstand.",
        };
        let text = format!(
            "Verlangt: Der Agent sollte einen Kandidaten liefern.\nGescheitert an: Es kam kein Kandidat an ({failure}), das Gate hat nichts geprüft.\nUrsache: {cause} Der Fehler liegt außerhalb von Auftrag und Gate.\nNächster Schritt: Eine Antwort von dir startet eine neue Runde, bei Bedarf mit einem anderen Agenten.\n"
        );
        record::publish_text(&target, &text, MAX_ANALYSIS_BYTES)?;
        return Ok(Some(target));
    }
    let read = |name: &str, limit: usize| {
        let mut text = fs::read_to_string(job.join(name)).unwrap_or_default();
        if text.len() > limit {
            let mut end = limit;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        text
    };
    let brief = ["submission.brief.md", "session.brief.md"]
        .into_iter()
        .map(|name| read(name, 6000))
        .find(|text| !text.is_empty())
        .unwrap_or_else(|| read(&format!("{stem}.brief.md"), 6000));
    let gate = ["submission.gate.mjs", "session.gate.sh"]
        .into_iter()
        .map(|name| read(name, 24_000))
        .find(|text| !text.is_empty())
        .unwrap_or_default();
    let prompt = format!(
        "{CONTRACT}\n\nWORK ORDER:\n{brief}\n\nDECISION RECORD OF THE REJECTED ROUND:\n{round}\n\nPINNED GATE (the check that judged the candidate):\n{}",
        if gate.is_empty() {
            "(not kept in the job)"
        } else {
            &gate
        },
    );
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let workspace = scratch.join("worker");
    fs::create_dir(&workspace)?;
    let (answer, worker) = source::ask(&workspace, job, &prompt, &mut Progress::silent());
    let (answer, usage) = answer?;
    // Keep the four lines the contract names, in its order. Anything else is dropped.
    let lines: Vec<String> = LABELS
        .iter()
        .filter_map(|label| {
            answer
                .lines()
                .map(|line| {
                    line.trim()
                        .trim_start_matches(['-', '*', ' '])
                        .replace("**", "")
                })
                .find(|line| line.starts_with(label) && line.len() > label.len() + 1)
        })
        .collect();
    if lines.len() != LABELS.len() {
        return Err("the analysis does not have its four lines".into());
    }
    let text = format!("{}\n", lines.join("\n"));
    if text.len() > MAX_ANALYSIS_BYTES {
        return Err("the analysis is too long".into());
    }
    record::publish_text(&target, &text, MAX_ANALYSIS_BYTES)?;
    // The call is on the books like any other worker call.
    record::publish_text(
        &job.join(format!("{stem}.analysis.run")),
        &format!("decision=observed\nidentity={stem}.analysis\nworker={worker}\n{usage}end\n"),
        record::MAX_RECORD_BYTES,
    )?;
    Ok(Some(target))
}
