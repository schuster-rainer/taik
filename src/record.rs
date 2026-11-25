use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use crate::{MAX_TAGS, Procedure, Result, expected_tags, valid_input};

pub const MAX_RECORD_BYTES: usize = 8_192;
pub const PASSING_CHECKS: &str =
    "checks=artifact_matches:true, probe_matches:true, size_within_limit:true";

pub struct Accepted {
    pub procedure: Procedure,
    pub tags: Vec<String>,
}

pub fn read_regular(path: &Path, max_bytes: usize) -> Result<String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    if !fs::symlink_metadata(path)?.is_file() {
        return Err("expected a regular file".into());
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err("expected a regular file".into());
    }
    let mut content = String::new();
    file.take(max_bytes as u64 + 1)
        .read_to_string(&mut content)?;
    if content.len() > max_bytes {
        return Err("file exceeds the byte limit".into());
    }
    Ok(content)
}

pub fn read_accepted(path: &Path) -> Result<Accepted> {
    if path.extension().is_none_or(|ext| ext != "run") {
        return Err("a baseline must be a published .run file".into());
    }
    let record = read_regular(path, MAX_RECORD_BYTES)?;
    parse_accepted(&record)
}

pub fn parse_accepted(record: &str) -> Result<Accepted> {
    if !record.ends_with("end\n") {
        return Err("baseline record is incomplete".into());
    }
    let mut lines = record.lines();
    if lines.next() != Some("decision=accepted") {
        return Err("baseline was not accepted".into());
    }
    let procedure = lines
        .next()
        .and_then(|line| line.strip_prefix("procedure="))
        .and_then(Procedure::parse)
        .filter(|procedure| procedure.valid_order() && procedure.probes().1)
        .ok_or("baseline has no recognized procedure that passes the probe")?;
    let serialized_tags = lines
        .next()
        .and_then(|line| line.strip_prefix("tags="))
        .ok_or("baseline has no tags field")?;
    let tags: Vec<_> = serialized_tags.split_terminator(',').collect();
    if tags.len() > MAX_TAGS || tags.iter().any(|tag| !valid_input(tag)) {
        return Err("baseline contains invalid tags".into());
    }
    let tags = expected_tags(&tags);
    if tags.join(",") != serialized_tags {
        return Err("baseline contains noncanonical tags".into());
    }
    if !lines.any(|line| line == PASSING_CHECKS) {
        return Err("baseline lacks passing acceptance checks".into());
    }
    Ok(Accepted { procedure, tags })
}

pub fn check_output(path: &Path) -> Result<()> {
    if path.extension().is_none_or(|ext| ext != "run") {
        return Err("output must end in .run".into());
    }
    if path
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty() && !parent.is_dir())
    {
        return Err("output directory does not exist".into());
    }
    match path.symlink_metadata() {
        Ok(_) => Err(format!("output already exists: {}", path.display()).into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn publish(path: &Path, record: &str) -> Result<()> {
    publish_text(path, record, MAX_RECORD_BYTES)
}

pub fn publish_text(path: &Path, record: &str, max_bytes: usize) -> Result<()> {
    if record.len() > max_bytes {
        return Err("record exceeds the byte limit".into());
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut pending = tempfile::Builder::new()
        .prefix(".taik-")
        .suffix(".pending")
        .tempfile_in(parent)?;
    pending.write_all(record.as_bytes())?;
    pending.as_file().sync_all()?;
    // Only complete records get a .run entry. hard_link never replaces it.
    fs::hard_link(pending.path(), path)?;
    if let Err(error) = pending.close() {
        eprintln!(
            "published {}, staging cleanup failed: {error}",
            path.display()
        );
    }
    #[cfg(unix)]
    {
        if let Err(error) = File::open(parent).and_then(|directory| directory.sync_all()) {
            eprintln!(
                "published {}, directory sync failed: {error}",
                path.display()
            );
        }
    }
    Ok(())
}
