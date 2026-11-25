#[cfg(target_os = "macos")]
mod platform {
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::path::Path;
    use std::ptr;

    pub struct QueueEvents {
        event_queue: OwnedFd,
        _directory: File,
    }

    impl QueueEvents {
        pub fn watch(path: &Path) -> io::Result<Self> {
            let directory = File::open(path)?;
            // kqueue gives ownership of the descriptor to this process.
            let fd = unsafe { libc::kqueue() };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let event_queue = unsafe { OwnedFd::from_raw_fd(fd) };
            let change = libc::kevent {
                ident: directory.as_raw_fd() as _,
                filter: libc::EVFILT_VNODE,
                flags: libc::EV_ADD | libc::EV_CLEAR,
                fflags: libc::NOTE_WRITE | libc::NOTE_DELETE | libc::NOTE_RENAME,
                data: 0,
                udata: ptr::null_mut(),
            };
            if unsafe { libc::kevent(fd, &change, 1, ptr::null_mut(), 0, ptr::null()) } < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                event_queue,
                _directory: directory,
            })
        }

        pub fn wait(&mut self) -> io::Result<()> {
            loop {
                let mut event = std::mem::MaybeUninit::<libc::kevent>::uninit();
                let count = unsafe {
                    libc::kevent(
                        self.event_queue.as_raw_fd(),
                        ptr::null(),
                        0,
                        event.as_mut_ptr(),
                        1,
                        ptr::null(),
                    )
                };
                if count > 0 {
                    let event = unsafe { event.assume_init() };
                    if event.flags & libc::EV_ERROR != 0 {
                        return Err(io::Error::from_raw_os_error(event.data as i32));
                    }
                    if event.fflags & (libc::NOTE_DELETE | libc::NOTE_RENAME) != 0 {
                        return Err(io::Error::other("watched job directory changed identity"));
                    }
                    return Ok(());
                }
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::ffi::CString;
    use std::fs::File;
    use std::io::{self, Read};
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub struct QueueEvents(File);

    impl QueueEvents {
        pub fn watch(path: &Path) -> io::Result<Self> {
            let path = CString::new(path.as_os_str().as_bytes())?;
            let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let events = unsafe { File::from_raw_fd(fd) };
            let mask = libc::IN_MOVED_TO | libc::IN_DELETE_SELF | libc::IN_MOVE_SELF;
            if unsafe { libc::inotify_add_watch(fd, path.as_ptr(), mask) } < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self(events))
        }

        pub fn wait(&mut self) -> io::Result<()> {
            let mut events = [0; 4096];
            loop {
                match self.0.read(&mut events) {
                    Ok(0) => return Err(io::Error::other("job event stream closed")),
                    Ok(_) => return Ok(()),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => (),
                    Err(error) => return Err(error),
                }
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod platform {
    pub struct QueueEvents;

    impl QueueEvents {
        pub fn watch(_: &std::path::Path) -> std::io::Result<Self> {
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "no job directory watcher on this OS",
            ))
        }

        pub fn wait(&mut self) -> std::io::Result<()> {
            unreachable!("watch could not open a job event stream")
        }
    }
}

pub use platform::QueueEvents;
