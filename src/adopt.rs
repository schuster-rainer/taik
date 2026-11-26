//! Brings the accepted commit of a session job into the repository it came from, as branch
//! `taik/NAME`. The repository gets the objects and one new branch, or an existing branch on an
//! ancestor moves forward. Nothing is checked out, merged or changed in its working files.

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
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
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

/// The branch of a settled job, written once.
pub const ADOPTED: &str = "adopted.txt";
/// Why the last settling failed, in one line. Replaced by the next failure.
pub const ADOPT_FAILED: &str = "adopt-failed.txt";

// One nonempty line: line breaks become spaces.
fn one_line(message: &str) -> String {
    let line = message.replace(['\r', '\n'], " ");
    let line = line.trim();
    if line.is_empty() {
        "adopt failed".to_owned()
    } else {
        line.to_owned()
    }
}

fn pending_in(job: &Path, text: &str) -> Result<tempfile::NamedTempFile> {
    use std::io::Write;
    let mut pending = tempfile::Builder::new()
        .prefix(".taik-")
        .suffix(".pending")
        .tempfile_in(job)?;
    pending.write_all(text.as_bytes())?;
    pending.as_file().sync_all()?;
    Ok(pending)
}

/// Adopts the accepted commit of a session job with [`adopt`] and records the result in the job
/// folder: the branch in `adopted.txt` (written once), or the failure in `adopt-failed.txt`.
/// A job without `session.repo` has nothing to adopt: `Ok(None)`, nothing is written.
/// An unknown job is an error and nothing is created.
pub fn settle(state: &Path, job_name: &str) -> Result<Option<String>> {
    let mut parts = Path::new(job_name).components();
    if !matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(_)), None)
    ) {
        return Err("a job is named by one folder name".into());
    }
    let job = state.join("jobs").join(job_name);
    if !job.is_dir() {
        return Err(format!("no such job: {}", job.display()).into());
    }
    if job.join("session.repo").symlink_metadata().is_err() {
        return Ok(None);
    }
    match adopt(state, job_name) {
        Ok(branch) => {
            let adopted = job.join(ADOPTED);
            if adopted.symlink_metadata().is_err() {
                // Never replaces an existing adopted.txt.
                let pending = pending_in(&job, &format!("{branch}\n"))?;
                if let Err(error) = pending.persist_noclobber(&adopted)
                    && error.error.kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(error.error.into());
                }
            }
            Ok(Some(branch))
        }
        Err(error) => {
            let message = one_line(&error.to_string());
            pending_in(&job, &format!("{message}\n"))
                .and_then(|pending| {
                    pending
                        .persist(job.join(ADOPT_FAILED))
                        .map_err(|error| error.error.into())
                })
                .map_err(|written| format!("{message}; {ADOPT_FAILED}: {written}"))?;
            Err(message.into())
        }
    }
}

fn git_failed(what: &str, output: &std::process::Output) -> Box<dyn std::error::Error> {
    format!(
        "git {what}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into()
}

// Whether `reference` is checked out in the repository or in one of its worktrees.
fn checked_out(repo: &Path, reference: &str) -> Result<bool> {
    let output = git(repo, &["worktree", "list", "--porcelain"])?;
    if !output.status.success() {
        return Err(git_failed("worktree list", &output));
    }
    let wanted = format!("branch {reference}");
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line == wanted))
}

// Moves an existing branch from `current` forward to `commit`, never backwards or sideways,
// and never while the branch is checked out.
fn advance(
    repo: &Path,
    work: &str,
    branch: &str,
    reference: &str,
    current: &str,
    commit: &str,
) -> Result<String> {
    if checked_out(repo, reference)? {
        return Err(format!(
            "{branch} is checked out in {} and points at {current}, not at {commit}",
            repo.display()
        )
        .into());
    }
    // Only the objects; no reference of the repository is written by this fetch.
    let output = git(repo, &["fetch", "--quiet", "--no-tags", work, commit])?;
    if !output.status.success() {
        return Err(git_failed("fetch", &output));
    }
    let output = git(repo, &["merge-base", "--is-ancestor", current, commit])?;
    match output.status.code() {
        Some(0) => {}
        Some(1) => {
            return Err(format!(
                "{branch} already points at {current}, which is not an ancestor of {commit}"
            )
            .into());
        }
        _ => return Err(git_failed("merge-base", &output)),
    }
    // The old value guards against a concurrent change of the branch.
    let output = git(
        repo,
        &[
            "update-ref",
            "-m",
            "taik adopt: fast-forward",
            reference,
            commit,
            current,
        ],
    )?;
    if !output.status.success() {
        return Err(git_failed("update-ref", &output));
    }
    match branch_target(repo, reference)? {
        Some(now) if now == commit => Ok(branch.to_owned()),
        other => {
            Err(format!("{branch} points at {other:?} after the update, not at {commit}").into())
        }
    }
}

/// Returns the branch name `taik/NAME` once it points at the accepted commit.
/// An existing branch on an ancestor of that commit moves forward to it, unless it is checked out.
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
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or_else(|| format!("no commit in {}", round.display()))?;

    let branch = format!("taik/{name}");
    let reference = format!("refs/heads/{branch}");
    let existing = match branch_target(repo, &reference)? {
        Some(current) if current == commit => return Ok(branch),
        other => other,
    };

    let work = state.join("work").join(name).canonicalize()?;
    let work = work.to_str().ok_or("work path is not UTF-8")?;
    if let Some(current) = existing {
        return advance(repo, work, &branch, &reference, &current, commit);
    }
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
        other => {
            Err(format!("{branch} points at {other:?} after the fetch, not at {commit}").into())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_failure_is_one_nonempty_line() {
        assert_eq!(super::one_line("a\nb\r\nc\n"), "a b  c");
        assert_eq!(super::one_line("\n"), "adopt failed");
    }
}
