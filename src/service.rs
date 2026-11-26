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
const DEFAULT_PARALLEL: usize = 3;
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

/// A reply of the user to a job. It is written into the job folder, and the service is woken the way
/// a new job wakes it: by a change in the job directory.
#[cfg(target_os = "macos")]
pub fn reply(state: &Path, name: &str, text: &str) -> Result<PathBuf> {
    let plain = name.strip_prefix("job-").is_some_and(|rest| {
        !rest.is_empty()
            && rest.len() <= 60
            && rest
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    });
    let jobs = jobs_in(state)?;
    // An answer to a question of the orchestrator about a request that has no proposal yet.
    if let Some(ticket) = crate::worker_name(name).filter(|_| !plain) {
        let requests = state.join("requests");
        let request = requests.join(format!("{ticket}.run"));
        let talk = crate::orchestrator::conversation(&request);
        if request.is_file()
            && fs::symlink_metadata(state.join("proposals").join(ticket)).is_err()
            && crate::talk::waits_for_user(&crate::talk::read(&talk))
        {
            let message = crate::talk::write(&talk, crate::talk::Author::User, 0, text)?;
            let wake = tempfile::Builder::new()
                .prefix(".wake-")
                .tempdir_in(&jobs)?;
            drop(wake);
            return Ok(message);
        }
    }
    // A comment on a proposal is kept with the proposal and moves into the job on approval.
    if !plain {
        let proposal = crate::worker_name(name)
            .map(|name| state.join("proposals").join(name))
            .filter(|proposal| {
                proposal.join("brief.md").is_file() && !proposal.join("approved.txt").exists()
            })
            .ok_or("a reply names a job or a waiting proposal")?;
        return crate::talk::write(&proposal, crate::talk::Author::User, 0, text);
    }
    let job = jobs.join(name);
    if !job.join("submission.brief.md").is_file() && !job.join(crate::session::BRIEF).is_file() {
        return Err("replies go to queued software jobs".into());
    }
    let message = crate::talk::write(
        &job,
        crate::talk::Author::User,
        crate::talk::rounds_done(&job),
        text,
    )?;
    let wake = tempfile::Builder::new()
        .prefix(".wake-")
        .tempdir_in(&jobs)?;
    drop(wake);
    Ok(message)
}

/// A request of the user in plain words. The service derives a proposal from it.
#[cfg(target_os = "macos")]
pub fn request(state: &Path, text: &str) -> Result<String> {
    let text = text.trim();
    if text.is_empty() || text.len() > crate::talk::MAX_MESSAGE_BYTES / 2 {
        return Err("a request is between 1 and 4096 bytes".into());
    }
    let (title, rest) = text.split_once('\n').unwrap_or((text, ""));
    let jobs = jobs_in(state)?;
    let requests = state.join("requests");
    fs::create_dir_all(&requests)?;
    let name = (1..=999)
        .map(|number| format!("ticket-{number:03}"))
        .find(|name| {
            !requests.join(format!("{name}.run")).exists()
                && !state.join("proposals").join(name).exists()
                && !jobs.join(format!("job-{name}")).exists()
        })
        .ok_or("too many requests")?;
    record::publish_text(
        &requests.join(format!("{name}.run")),
        &format!(
            "decision=observed\nidentity={name}\nintent={}\nobservation={}\ncontinuation=Derive one job that fulfils this request of the user. Stop if the request is unclear or cannot be checked by a gate.\nend\n",
            crate::source::single_line(title),
            // The whole request on one line. It is not cut: the worker must read all of it.
            rest.trim()
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect::<String>(),
        ),
        MAX_RECORD_BYTES,
    )?;
    let wake = tempfile::Builder::new()
        .prefix(".wake-")
        .tempdir_in(&jobs)?;
    drop(wake);
    Ok(name)
}

// Requests that have neither a proposal nor an answer yet. Each is derived once.
#[cfg(target_os = "macos")]
fn derive_requests(home: &Path) {
    let requests = home.join("requests");
    let Ok(entries) = fs::read_dir(&requests) else {
        return;
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".run")
                .filter(|stem| stem.starts_with("ticket-") && !stem.contains('.'))
                .map(str::to_owned)
        })
        .collect();
    names.sort_unstable();
    for name in names {
        if requests.join(format!("reflection-{name}.run")).exists()
            || requests.join(format!("{name}.failed")).exists()
            || home.join("proposals").join(&name).exists()
        {
            continue;
        }
        // The orchestrator decides how the request is carried out, then the agent it names writes the gate.
        let request = requests.join(format!("{name}.run"));
        // The orchestrator asked the user something: the request waits for the answer.
        if crate::talk::waits_for_user(&crate::talk::read(&crate::orchestrator::conversation(
            &request,
        ))) {
            continue;
        }
        let derived = crate::orchestrator::plan(home, &name, &request).and_then(|plan| {
            let Some(plan) = plan else {
                return Ok(None);
            };
            let mut outcome = crate::proposal::propose_planned(home, &name, &request, Some(&plan));
            // An agent that cannot be reached does not end the request: another one writes the gate.
            let mut tried = plan.gate.clone();
            for substitute in crate::orchestrator::substitutes(home, &plan) {
                let Err(error) = &outcome else { break };
                eprintln!("request {name}: gate by {tried}: {error}");
                tried.clone_from(&substitute.gate);
                outcome =
                    crate::proposal::propose_planned(home, &name, &request, Some(&substitute));
            }
            outcome
        });
        if let Err(error) = derived {
            eprintln!("request {name}: {error}");
            let _ = fs::write(
                requests.join(format!("{name}.failed")),
                format!("{}\n", crate::source::single_line(&error.to_string())),
            );
        }
    }
}

pub fn queue_self(state: &Path) -> Result<PathBuf> {
    let jobs = jobs_in(state)?;
    let job = stage(&jobs)?;
    record::publish(&job.path().join("request.run"), self_improve::REQUEST)?;
    publish_job(job, jobs.join(self_improve::JOB))
}

/// What a queued job takes along besides its brief and gate: the agent that works it, what was
/// said before it existed, and the plan of its proposal.
#[cfg(target_os = "macos")]
#[derive(Default)]
pub struct Handover<'a> {
    pub agent: Option<&'a str>,
    pub said: &'a [crate::talk::Message],
    pub plan: Option<&'a Path>,
}

#[cfg(target_os = "macos")]
pub fn queue_source(
    state: &Path,
    name: &str,
    brief: &Path,
    gate: &Path,
    seed: Option<&Path>,
    handover: Handover<'_>,
) -> Result<PathBuf> {
    let Handover {
        agent,
        said: conversation,
        plan,
    } = handover;
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
    // The agent of the job is in place before the service can see the job.
    #[cfg(target_os = "macos")]
    if let Some(agent) = agent {
        if !crate::crew::valid_name(agent) {
            return Err("invalid agent name".into());
        }
        fs::write(job.path().join("agent"), format!("{agent}\n"))?;
    }
    #[cfg(not(target_os = "macos"))]
    let _ = agent;
    // An accepted artifact from elsewhere that the job builds on.
    if let Some(seed) = seed {
        let source = record::read_regular(seed, crate::source::LIMIT)?;
        record::publish_text(
            &job.path().join(crate::source::SEED),
            &source,
            crate::source::LIMIT,
        )?;
    }
    // What was said before the job existed is in place before the service can see the job.
    for message in conversation {
        crate::talk::write(job.path(), message.author, 0, &message.text)?;
    }
    // The plan of the proposal travels with the job, byte for byte, before the service sees it.
    if let Some(plan) = plan {
        let text = record::read_regular(plan, MAX_RECORD_BYTES)?;
        record::publish_text(&job.path().join("plan.txt"), &text, MAX_RECORD_BYTES)?;
    }
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
    // Settings, agents and proposals live in the state directory the user names. The job folder may
    // be a link into a checkout, so its parent is not that directory.
    let home = state.canonicalize()?;
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
            &home,
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
    home: &Path,
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
    #[cfg(target_os = "macos")]
    let mut source_jobs = Vec::new();
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
            // A closed job gets no further round.
            if job.join("baseline.run").is_file() && !job.join(crate::CLOSED).exists() {
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
        if job.join("submission.brief.md").is_file() || job.join(crate::session::BRIEF).is_file() {
            source_jobs.push(job);
        }
    }
    // Software jobs wait on their workers, so several run side by side. Each job has its own lock,
    // folder and records. A job never runs twice at once.
    #[cfg(target_os = "macos")]
    {
        // The viewer writes STATE/parallel. It wins over the environment and is read on every pass.
        let width = fs::read_to_string(home.join("parallel"))
            .ok()
            .or_else(|| std::env::var("TAIK_PARALLEL").ok())
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(DEFAULT_PARALLEL)
            .clamp(1, 8);
        // Jobs that arrive while others run are picked up by the next free slot.
        struct Pool {
            waiting: std::collections::VecDeque<PathBuf>,
            seen: std::collections::HashSet<PathBuf>,
            active: usize,
        }
        let pool = std::sync::Mutex::new(Pool {
            seen: source_jobs.iter().cloned().collect(),
            waiting: source_jobs.into(),
            active: 0,
        });
        let changed = std::sync::Condvar::new();
        let all_accepted = std::sync::atomic::AtomicBool::new(true);
        let arrivals = || -> Vec<PathBuf> {
            let Ok(entries) = fs::read_dir(jobs) else {
                return Vec::new();
            };
            let mut found: Vec<_> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|job| {
                    job.join("submission.brief.md").is_file()
                        || job.join(crate::session::BRIEF).is_file()
                })
                .collect();
            found.sort_unstable();
            found
        };
        let working = std::sync::atomic::AtomicUsize::new(width);
        std::thread::scope(|scope| {
            // Requests are derived while jobs run, not only between passes: a wish of the user does
            // not wait for a long session to end.
            scope.spawn(|| {
                loop {
                    derive_requests(home);
                    if working.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                        break;
                    }
                    // Short enough that the pass ends at once when the last job is done.
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            });
            for _ in 0..width {
                scope.spawn(|| {
                    // Counted down however the worker thread ends.
                    struct Done<'a>(&'a std::sync::atomic::AtomicUsize);
                    impl Drop for Done<'_> {
                        fn drop(&mut self) {
                            self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    let _done = Done(&working);
                    loop {
                        let job = {
                            let Ok(mut pool) = pool.lock() else { break };
                            loop {
                                if pool.waiting.is_empty() {
                                    for job in arrivals() {
                                        if pool.seen.insert(job.clone()) {
                                            pool.waiting.push_back(job);
                                        }
                                    }
                                }
                                if let Some(job) = pool.waiting.pop_front() {
                                    pool.active += 1;
                                    break Some(job);
                                }
                                if pool.active == 0 {
                                    break None;
                                }
                                // Others are still working. Look again when one finishes or after a second.
                                let Ok((next, _)) =
                                    changed.wait_timeout(pool, std::time::Duration::from_secs(1))
                                else {
                                    return;
                                };
                                pool = next;
                            }
                        };
                        let Some(job) = job else {
                            changed.notify_all();
                            break;
                        };
                        let accepted = run_source_job(home, jobs, &job).unwrap_or_else(|error| {
                            eprintln!("{}: {error}", job.display());
                            false
                        });
                        if !accepted {
                            all_accepted.store(false, std::sync::atomic::Ordering::Relaxed);
                        }
                        if let Ok(mut pool) = pool.lock() {
                            pool.active -= 1;
                        }
                        changed.notify_all();
                    }
                });
            }
        });
        succeeded &= all_accepted.load(std::sync::atomic::Ordering::Relaxed);
    }
    Ok(succeeded)
}

#[cfg(target_os = "macos")]
fn run_source_job(home: &Path, jobs: &Path, job: &Path) -> Result<bool> {
    // A closed job gets no further round, not even after a reply.
    if job.join(crate::CLOSED).exists() {
        return Ok(true);
    }
    // The agent of this job, if it names one. Read before every pass, so the viewer can change it.
    let agent = crate::crew::for_job(home, job);
    let agent = match agent {
        Ok(agent) => agent,
        Err(error) => {
            // A choice that cannot be honoured stops the job. It never falls back to another account.
            eprintln!("{}: agent: {error}", job.display());
            return Ok(false);
        }
    };
    // A session job: an agent with file tools in its own copy, judged by commit.
    #[cfg(target_os = "macos")]
    if job.join(crate::session::BRIEF).is_file() {
        let outcome = crate::session::run_queued(home, job, agent.as_ref())?;
        // An accepted round is brought into the repository it came from. A failure is recorded in
        // the job and changes neither the decision nor the rounds.
        if outcome == Some(true)
            && let Some(name) = job.file_name().and_then(|name| name.to_str())
            && let Err(error) = crate::adopt::settle(home, name)
        {
            eprintln!("{}: settle: {error}", job.display());
        }
        if outcome == Some(false)
            && let Err(error) = crate::analysis::of_last_round(job)
        {
            eprintln!("{}: analysis: {error}", job.display());
        }
        return Ok(outcome.unwrap_or(true));
    }
    let (mut outcome, follow_up) = crate::source::run_queued(job, agent.as_ref())?;
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
            outcome = crate::source::run_queued(&destination, agent.as_ref())?
                .0
                .or(outcome);
        }
    }
    // A gate may name a release step for its accepted artifact. The job's author wrote that line
    // before the candidate existed, and the approval of the job covers it.
    #[cfg(target_os = "macos")]
    if outcome == Some(true)
        && let Err(error) = release(job)
    {
        eprintln!("{}: release: {error}", job.display());
    }
    // A rejection is explained in the ticket, in a few plain lines.
    #[cfg(target_os = "macos")]
    if outcome == Some(false)
        && let Err(error) = crate::analysis::of_last_round(job)
    {
        eprintln!("{}: analysis: {error}", job.display());
    }
    // A round ran in this call and the job is final. Ask once what follows, if the user enabled it.
    #[cfg(target_os = "macos")]
    if outcome.is_some()
        && std::env::var_os(crate::proposal::AUTO).is_some()
        && let Err(error) = crate::proposal::reflect(home, jobs, job)
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

// Runs the script named by a gate line "// TAIK: release tools/NAME.sh" with the job folder.
#[cfg(target_os = "macos")]
fn release(job: &Path) -> Result<()> {
    let gate = record::read_regular(&job.join("submission.gate.mjs"), crate::source::LIMIT)?;
    let Some(script) = gate
        .lines()
        .find_map(|line| line.strip_prefix("// TAIK: release "))
        .map(str::trim)
    else {
        return Ok(());
    };
    let name = script.strip_prefix("tools/").unwrap_or("");
    let plain = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && !name.contains("..");
    if !plain {
        return Err("a release step must be a script directly in tools/".into());
    }
    let log = File::options()
        .create(true)
        .append(true)
        .open(job.join("release.log"))?;
    let mut child = std::process::Command::new(script)
        .arg(job)
        .stdin(std::process::Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("{script} ended with {status}").into())
            };
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{script} exceeded 300 s").into());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
