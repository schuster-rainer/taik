use std::fmt;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

const LIMIT: u64 = 32_768;

pub struct Progress {
    file: Option<File>,
    started: Instant,
    bytes: u64,
}

impl Progress {
    pub fn silent() -> Self {
        Self {
            file: None,
            started: Instant::now(),
            bytes: 0,
        }
    }

    #[cfg(unix)]
    pub fn for_round(target: &std::path::Path) -> Self {
        use std::os::unix::fs::OpenOptionsExt;

        let mut progress = Self::silent();
        let path = target.with_extension("progress");
        let file = File::options()
            .create(true)
            .append(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path);
        match file.and_then(|file| Ok((file.metadata()?, file))) {
            Ok((info, file)) if info.is_file() => {
                progress.file = Some(file);
                progress.bytes = info.len();
            }
            Ok(_) => eprintln!("{} is not a regular progress file", path.display()),
            Err(error) => eprintln!("cannot record progress at {}: {error}", path.display()),
        }
        progress
    }

    pub fn event(&mut self, kind: &str, fact: fmt::Arguments<'_>) {
        let Some(file) = &mut self.file else { return };
        let line = format!("{}\t{kind}\t{fact}\n", self.started.elapsed().as_micros());
        let size = line.len() as u64;
        if self.bytes + size > LIMIT {
            return;
        }
        match file.write_all(line.as_bytes()) {
            Ok(()) => self.bytes += size,
            Err(error) => {
                eprintln!("cannot append round progress: {error}");
                self.file = None;
            }
        }
    }
}
