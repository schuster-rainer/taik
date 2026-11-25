use std::collections::HashMap;
use std::fs::{self, File, Metadata};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::queue_events::QueueEvents;
use crate::{
    MAX_RECORD_BYTES, Outcome, Result, evolve_progress,
    procedure::{Search, SearchTable},
    progress::Progress,
    record, research, self_improve,
};

const MAX_ROUNDS: usize = 20;
const MAX_CACHED_TAG_JOBS: usize = 4096;
#[cfg(target_os = "macos")]
use crate::source::FOLLOW_UP_SUFFIX;

fn jobs_in(state: &Path) -> Result<PathBuf> {
    let jobs = state.join("jobs");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&jobs)?;
    Ok(jobs.canonicalize()?)
}

pub fn queue(state: &Path, baseline: &Path) -> Result<PathBuf> {
    if baseline.extension().is_none_or(|ext| ext != "run") {
        return Err("a queued baseline must be a published .run file".into());
    }
    let source = record::read_regular(baseline, MAX_RECORD_BYTES)?;
    record::parse_accepted(&source)?;
    let jobs = jobs_in(state)?;
    let job = stage(&jobs)?;
    let mut origin = File::create(job.path().join("origin.txt"))?;
    let canonical = baseline.canonicalize()?;
    let reference = canonical.strip_prefix(&jobs).map_or_else(
        |_| canonical.clone(),
        |relative| Path::new("..").join(relative),
    );
    writeln!(origin, "source={reference:?}")?;
    origin.sync_all()?;
    record::publish(&job.path().join("baseline.run"), &source)?;
    let name = job.path().file_name().unwrap().to_string_lossy();
    let published = jobs.join(name.trim_start_matches('.'));
    publish_job(job, published)
}

pub fn queue_self(state: &Path) -> Result<PathBuf> {
    let jobs = jobs_in(state)?;
    let job = stage(&jobs)?;
    record::publish(&job.path().join("request.run"), self_improve::REQUEST)?;
    publish_job(job, jobs.join(self_improve::JOB))
}

#[cfg(target_os = "macos")]
pub fn queue_source(state: &Path, name: &str, brief: &Path, gate: &Path) -> Result<PathBuf> {
    if name.is_empty()
        || name.len() > 48
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("use a short job name with lowercase ASCII letters, digits or hyphens".into());
    }
    let destination = format!("job-{name}");
    if destination == self_improve::JOB
        || destination == research::JOB
        || destination == crate::activation::JOB
        || destination == crate::restart_probe::JOB
        || destination.ends_with(FOLLOW_UP_SUFFIX)
    {
        return Err("reserved Factory job name".into());
    }
    let brief = record::read_regular(brief, MAX_RECORD_BYTES)?;
    crate::source::intent(&brief)?;
    let gate = record::read_regular(gate, crate::source::LIMIT)?;
    if gate.trim().is_empty() {
        return Err("source jobs require a nonempty verification gate".into());
    }
    let jobs = jobs_in(state)?;
    let job = stage(&jobs)?;
    record::publish(&job.path().join("submission.brief.md"), &brief)?;
    record::publish_text(
        &job.path().join("submission.gate.mjs"),
        &gate,
        crate::source::LIMIT,
    )?;
    record::publish(&job.path().join("request.run"), crate::source::REQUEST)?;
    publish_job(job, jobs.join(destination))
}

fn stage(jobs: &Path) -> Result<tempfile::TempDir> {
    Ok(tempfile::Builder::new()
        .prefix(".job-")
        .tempdir_in(jobs.parent().unwrap())?)
}

fn publish_job(job: tempfile::TempDir, published: PathBuf) -> Result<PathBuf> {
    if published.symlink_metadata().is_ok() {
        return Err(format!("job already exists: {}", published.display()).into());
    }
    // Readiness and the kernel wake-up are the same filesystem operation.
    fs::rename(job.path(), &published)?;
    let _ = job.keep();
    if let Err(error) = File::open(published.parent().unwrap()).and_then(|dir| dir.sync_all()) {
        eprintln!(
            "queued {}, directory sync failed: {error}",
            published.display()
        );
    }
    Ok(published)
}

pub fn run(state: &Path, continuous: bool, reflection: bool) -> Result<bool> {
    let jobs = jobs_in(state)?;
    let state = jobs.parent().unwrap();
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(state.join("service.lock"))?;
    // The lock is released by the OS if the daemon is killed.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("another Factory worker holds this state directory".into());
    }
    let mut events = continuous.then(|| QueueEvents::watch(&jobs)).transpose()?;
    if events.is_some() {
        eprintln!("watching for jobs in {}", jobs.display());
    }
    let started_as = installed_identity();
    let mut table = None;
    let mut completed = HashMap::new();
    #[cfg(target_os = "macos")]
    let mut first_use = crate::activation::accepted(&jobs).unwrap_or_else(|error| {
        eprintln!("{}: {error}", crate::activation::JOB);
        false
    });
    #[cfg(not(target_os = "macos"))]
    let mut first_use = false;
    loop {
        let succeeded = drain(
            &jobs,
            &mut table,
            &mut first_use,
            reflection,
            &mut completed,
        )?;
        let Some(events) = events.as_mut() else {
            return Ok(succeeded);
        };
        events.wait()?;
        // After a promotion the next job belongs to the accepted binary. The supervisor starts it.
        if installed_identity() != started_as {
            eprintln!("the installed binary changed; leaving the work to its successor");
            return Ok(true);
        }
    }
}

fn installed_identity() -> Option<(u64, u64)> {
    let metadata = std::env::current_exe().ok()?.metadata().ok()?;
    Some((metadata.dev(), metadata.ino()))
}

fn drain(
    jobs: &Path,
    table: &mut Option<SearchTable>,
    first_use: &mut bool,
    _reflection: bool,
    completed: &mut HashMap<PathBuf, Vec<RecordStamp>>,
) -> Result<bool> {
    let mut entries = fs::read_dir(jobs)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_unstable_by_key(fs::DirEntry::file_name);
    let (search, mut succeeded) = self_improve::drive(&jobs.join(self_improve::JOB))?;
    if search != Search::ValidUnique {
        *table = None;
    }
    if succeeded && table.is_none() && !*first_use {
        match research::drive(jobs, search) {
            Ok(accepted) => *table = accepted,
            Err(error) => {
                eprintln!("{}: {error}", research::JOB);
                succeeded = false;
            }
        }
    }
    #[cfg(target_os = "macos")]
    if _reflection
        && !*first_use
        && let Some(current) = table.as_ref()
    {
        match crate::activation::reflect(jobs, current) {
            Ok(accepted) => *first_use = accepted,
            Err(error) => {
                eprintln!("{}: {error}", crate::activation::JOB);
                succeeded = false;
            }
        }
    }
    #[cfg(target_os = "macos")]
    if _reflection
        && *first_use
        && search == Search::ValidUnique
        && let Err(error) = crate::restart_probe::reflect(jobs)
    {
        eprintln!("{}: {error}", crate::restart_probe::JOB);
        succeeded = false;
    }
    for entry in entries {
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let job = entry.path();
        let verified = completed
            .get(&job)
            .is_some_and(|history| history.iter().all(RecordStamp::matches));
        if !verified {
            completed.remove(&job);
            if job.join("baseline.run").is_file() {
                match run_job(&job, search, table, *first_use) {
                    Ok((accepted, history)) => {
                        succeeded &= accepted;
                        if let Some(history) = history
                            && completed.len() < MAX_CACHED_TAG_JOBS
                        {
                            completed.insert(job.clone(), history);
                        }
                    }
                    Err(error) => {
                        eprintln!("{}: {error}", job.display());
                        succeeded = false;
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        if job.join("submission.brief.md").is_file() {
            match run_source_job(jobs, &job) {
                Ok(accepted) => succeeded &= accepted,
                Err(error) => {
                    eprintln!("{}: {error}", job.display());
                    succeeded = false;
                }
            }
        }
    }
    Ok(succeeded)
}

#[cfg(target_os = "macos")]
fn run_source_job(jobs: &Path, job: &Path) -> Result<bool> {
    let (mut outcome, follow_up) = crate::source::run_queued(job)?;
    if let Some(follow_up) = follow_up {
        let mut name = job
            .file_name()
            .ok_or("missing source job name")?
            .to_os_string();
        name.push(FOLLOW_UP_SUFFIX);
        let destination = jobs.join(name);
        if destination.try_exists()? {
            let origin = record::read_regular(&destination.join("origin.txt"), MAX_RECORD_BYTES)?;
            if origin != follow_up.origin()? {
                return Err("follow-up name is occupied by another job".into());
            }
        } else {
            let staged = stage(jobs)?;
            follow_up.write(staged.path())?;
            let destination = publish_job(staged, destination)?;
            outcome = crate::source::run_queued(&destination)?.0.or(outcome);
        }
    }
    // A round ran in this call and the job is final. Ask once what follows, if the user enabled it.
    #[cfg(target_os = "macos")]
    if outcome.is_some()
        && std::env::var_os(crate::proposal::AUTO).is_some()
        && let Err(error) = crate::proposal::reflect(jobs, job)
    {
        eprintln!("{}: reflection: {error}", job.display());
    }
    Ok(outcome.unwrap_or(true))
}

fn run_job(
    job: &Path,
    search: Search,
    table: &mut Option<SearchTable>,
    first_use: bool,
) -> Result<(bool, Option<Vec<RecordStamp>>)> {
    let mut baseline = job.join("baseline.run");
    let mut stamps = baseline
        .symlink_metadata()
        .ok()
        .and_then(|metadata| RecordStamp::regular(&baseline, &metadata))
        .map(|stamp| vec![stamp]);
    for index in 1..=MAX_ROUNDS {
        let target = job.join(format!("round-{index:02}.run"));
        let outcome = match target.symlink_metadata() {
            Ok(metadata) => {
                let outcome = read_outcome(&target)?;
                if let Some(history) = stamps.as_mut() {
                    if let Some(stamp) = RecordStamp::regular(&target, &metadata) {
                        history.push(stamp);
                    } else {
                        stamps = None;
                    }
                }
                outcome
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // Newly written histories will be cached after their first complete reread.
                stamps = None;
                let mut progress = Progress::for_round(&target);
                if first_use && search == Search::ValidUnique && table.is_none() {
                    match research::drive(job.parent().unwrap(), search) {
                        Ok(verified) => {
                            if verified.is_some() {
                                progress.event(
                                    "checks",
                                    format_args!("Suchtabelle vor der ersten Nutzung geprüft"),
                                );
                            }
                            *table = verified;
                        }
                        Err(error) => progress
                            .event("error", format_args!("Search table unavailable: {error}")),
                    }
                }
                let outcome =
                    evolve_progress(&baseline, &target, search, table.as_ref(), &mut progress);
                if let Err(error) = &outcome {
                    progress.event("error", format_args!("{error:?}"));
                }
                outcome?
            }
            Err(error) => return Err(error.into()),
        };
        if !outcome.advance {
            return Ok((
                outcome.accepted,
                if outcome.accepted { stamps } else { None },
            ));
        }
        baseline = target;
    }
    Err(format!(
        "job reached the {MAX_ROUNDS}-round bound; its last accepted record is {}",
        baseline.display()
    )
    .into())
}

struct RecordStamp {
    path: PathBuf,
    version: RecordVersion,
}

#[derive(PartialEq)]
struct RecordVersion {
    identity: (u64, u64),
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl RecordVersion {
    fn of(metadata: &Metadata) -> Self {
        Self {
            identity: (metadata.dev(), metadata.ino()),
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

impl RecordStamp {
    fn regular(path: &Path, metadata: &Metadata) -> Option<Self> {
        metadata.is_file().then(|| Self {
            path: path.to_path_buf(),
            version: RecordVersion::of(metadata),
        })
    }

    fn matches(&self) -> bool {
        self.path.symlink_metadata().is_ok_and(|metadata| {
            metadata.is_file() && RecordVersion::of(&metadata) == self.version
        })
    }
}

fn read_outcome(path: &Path) -> Result<Outcome> {
    let text = record::read_regular(path, MAX_RECORD_BYTES)?;
    if !text.ends_with("end\n") {
        return Err("published round is incomplete".into());
    }
    if text.starts_with("decision=accepted\n") {
        record::parse_accepted(&text)?;
        let advance = match text.lines().find_map(|line| line.strip_prefix("advance=")) {
            Some("true") => true,
            Some("false") => false,
            _ => return Err("published round lacks a continuation decision".into()),
        };
        Ok(Outcome {
            accepted: true,
            advance,
        })
    } else if text.starts_with("decision=rejected\n") {
        Ok(Outcome {
            accepted: false,
            advance: false,
        })
    } else {
        Err("published round lacks an outcome".into())
    }
}
