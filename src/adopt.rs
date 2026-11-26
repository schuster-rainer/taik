//! Brings the accepted commit of a session job into the repository it came from, as branch
//! `taik/NAME`. The repository gets the objects and one new branch. Nothing is checked out,
//! merged or changed in its working files.

use std::path::Path;
use std::process::Command;

use crate::Result;

fn git(folder: &Path, arguments: &[&str]) -> Result<std::process::Output> {
    Ok(Command::new("git")
        .arg("-C")
        .arg(folder)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "gc.auto=0",
            "-c",
            "maintenance.auto=false",
            "-c",
            "fetch.recurseSubmodules=false",
            "-c",
            "uploadpack.allowAnySHA1InWant=true",
            "-c",
            "protocol.version=2",
        ])
        .args(arguments)
        .output()?)
}

fn branch_target(repo: &Path, reference: &str) -> Result<Option<String>> {
    let output = git(
        repo,
        &["rev-parse", "--verify", "--quiet", &format!("{reference}^{{commit}}")],
    )?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned()))
}

fn last_round(job: &Path) -> Result<std::path::PathBuf> {
    let mut last: Option<(u8, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(job)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(number) = name
            .strip_prefix("round-")
            .and_then(|rest| rest.strip_suffix(".run"))
            .filter(|digits| digits.len() == 2 && digits.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|digits| digits.parse::<u8>().ok())
        else {
            continue;
        };
        if last.as_ref().is_none_or(|(best, _)| number > *best) {
            last = Some((number, entry.path()));
        }
    }
    last.map(|(_, path)| path)
        .ok_or_else(|| format!("no round in {}", job.display()).into())
}

/// Returns the branch name `taik/NAME` once it points at the accepted commit.
pub fn adopt(state: &Path, job_name: &str) -> Result<String> {
    let name = job_name
        .strip_prefix("job-")
        .and_then(crate::worker_name)
        .ok_or("a session job is named job-NAME")?;
    let job = state.join("jobs").join(job_name);
    if !job.is_dir() {
        return Err(format!("no such job: {}", job.display()).into());
    }
    let repo_file = job.join("session.repo");
    let repo = std::fs::read_to_string(&repo_file)
        .map_err(|error| format!("{}: {error}", repo_file.display()))?;
    let repo = repo.strip_suffix('\n').unwrap_or(&repo);
    if repo.is_empty() || repo.contains('\n') {
        return Err(format!("{} names no repository", repo_file.display()).into());
    }
    let repo = Path::new(repo);

    let round = last_round(&job)?;
    let text = std::fs::read_to_string(&round)?;
    if text.lines().next() != Some("decision=accepted") {
        return Err(format!("the last round is not accepted: {}", round.display()).into());
    }
    let commit = text
        .lines()
        .find_map(|line| line.strip_prefix("commit="))
        .filter(|sha| {
            matches!(sha.len(), 40 | 64)
                && sha.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or_else(|| format!("no commit in {}", round.display()))?;

    let branch = format!("taik/{name}");
    let reference = format!("refs/heads/{branch}");
    match branch_target(repo, &reference)? {
        Some(current) if current == commit => return Ok(branch),
        Some(current) => {
            return Err(format!("{branch} already points at {current}, not at {commit}").into());
        }
        None => {}
    }

    let work = state.join("work").join(name).canonicalize()?;
    let work = work.to_str().ok_or("work path is not UTF-8")?;
    // No "+": the fetch never moves an existing branch backwards or sideways.
    let output = git(
        repo,
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            work,
            &format!("{commit}:{reference}"),
        ],
    )?;
    if !output.status.success() {
        return Err(format!(
            "git fetch: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    match branch_target(repo, &reference)? {
        Some(current) if current == commit => Ok(branch),
        other => Err(format!("{branch} points at {other:?} after the fetch, not at {commit}").into()),
    }
}
