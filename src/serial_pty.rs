//! Host TTYA transport. The SCC remains entirely in Helios.
use std::{collections::VecDeque, path::Path};

#[cfg(unix)]
pub const QUEUE_LIMIT: usize = 64 * 1024;
#[cfg(unix)]
const IO_BUDGET: usize = 4096;

#[cfg(unix)]
mod unix {
    use super::*;
    use nix::{
        fcntl::{FcntlArg, FdFlag, OFlag, fcntl},
        pty::openpty,
        sys::termios::{self, ControlFlags, FlushArg, SetArg},
        unistd::ttyname,
    };
    use std::{
        fs::{self, File, OpenOptions},
        io::{self, Read, Seek, SeekFrom, Write},
        os::unix::fs::{MetadataExt, OpenOptionsExt, symlink},
        path::PathBuf,
    };

    const MAGIC: &str = "heliacal-serial-pty-v1\n";

    struct Alias {
        path: PathBuf,
        target: PathBuf,
        identity: (u64, u64),
        _lock: File,
    }

    impl Alias {
        fn create(path: &Path, target: &Path) -> Result<Self, String> {
            let mut lock_path = path.as_os_str().to_os_string();
            lock_path.push(".heliacal-lock");
            let lock_path = PathBuf::from(lock_path);
            let open = |create| {
                let mut options = OpenOptions::new();
                options.read(true).write(true).mode(0o600).custom_flags(
                    OFlag::O_NOFOLLOW.bits() | OFlag::O_CLOEXEC.bits() | OFlag::O_NONBLOCK.bits(),
                );
                if create {
                    options.create_new(true);
                }
                options.open(&lock_path)
            };
            let (mut lock, created) = match open(true) {
                Ok(file) => (file, true),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (
                    open(false).map_err(|e| format!("PTY alias lock: {e}"))?,
                    false,
                ),
                Err(e) => return Err(format!("PTY alias lock: {e}")),
            };
            lock.try_lock()
                .map_err(|e| format!("PTY alias is already in use: {e}"))?;
            let metadata = lock.metadata().map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.len() > 4096 {
                return Err("PTY alias lock is not a Heliacal state file".into());
            }
            if created {
                lock.write_all(MAGIC.as_bytes())
                    .map_err(|e| e.to_string())?;
                lock.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            }
            let mut record = String::new();
            lock.read_to_string(&mut record)
                .map_err(|e| e.to_string())?;
            if !record.starts_with(MAGIC) {
                return Err("PTY alias lock belongs to another application".into());
            }
            match fs::symlink_metadata(path) {
                Ok(meta) => {
                    let old = fs::read_link(path).ok();
                    let ours = old.as_ref().is_some_and(|old| {
                        record.strip_prefix(MAGIC).is_some_and(|r| r == old.to_string_lossy())
                            && matches!(fs::metadata(old), Err(e) if e.kind() == io::ErrorKind::NotFound)
                    });
                    if !meta.file_type().is_symlink() || !ours {
                        return Err(format!(
                            "PTY alias {} already exists; choose an unused path",
                            path.display()
                        ));
                    }
                    fs::remove_file(path).map_err(|e| e.to_string())?;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("PTY alias: {e}")),
            }
            // Keep the lock inode across runs: unlinking a held lock permits a
            // second process to lock a new inode at the same name.
            lock.set_len(0).map_err(|e| e.to_string())?;
            lock.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            write!(lock, "{MAGIC}{}", target.display()).map_err(|e| e.to_string())?;
            lock.sync_all().map_err(|e| e.to_string())?;
            symlink(target, path).map_err(|e| format!("PTY alias {}: {e}", path.display()))?;
            let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            Ok(Self {
                path: path.into(),
                target: target.into(),
                identity: (meta.dev(), meta.ino()),
                _lock: lock,
            })
        }
    }
    impl Drop for Alias {
        fn drop(&mut self) {
            if fs::symlink_metadata(&self.path).is_ok_and(|m| (m.dev(), m.ino()) == self.identity)
                && fs::read_link(&self.path).is_ok_and(|target| target == self.target)
            {
                let _ = fs::remove_file(&self.path);
            }
        }
    }

    pub struct SerialPty {
        master: File,
        slave: File,
        path: PathBuf,
        alias: Option<Alias>,
        pub input: VecDeque<u8>,
        output: VecDeque<u8>,
        pub dropped: u64,
    }
    impl SerialPty {
        pub fn open(alias: &str) -> Result<Self, String> {
            let pair = openpty(None, None).map_err(|e| format!("Open serial PTY: {e}"))?;
            let mut term = termios::tcgetattr(&pair.slave).map_err(|e| e.to_string())?;
            termios::cfmakeraw(&mut term);
            term.control_flags
                .insert(ControlFlags::CLOCAL | ControlFlags::CREAD);
            termios::cfsetspeed(&mut term, termios::BaudRate::B9600).map_err(|e| e.to_string())?;
            termios::tcsetattr(&pair.slave, SetArg::TCSANOW, &term).map_err(|e| e.to_string())?;
            for fd in [&pair.master, &pair.slave] {
                fcntl(fd, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).map_err(|e| e.to_string())?;
                fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(|e| e.to_string())?;
            }
            let path = ttyname(&pair.slave).map_err(|e| e.to_string())?;
            let alias = if alias.is_empty() {
                None
            } else {
                Some(Alias::create(Path::new(alias), &path)?)
            };
            Ok(Self {
                master: pair.master.into(),
                slave: pair.slave.into(),
                path,
                alias,
                input: VecDeque::new(),
                output: VecDeque::new(),
                dropped: 0,
            })
        }
        pub fn path(&self) -> &Path {
            &self.path
        }
        pub fn endpoint(&self) -> &Path {
            self.alias.as_ref().map_or(&self.path, |a| &a.path)
        }
        pub fn alias_matches(&self, path: &str) -> bool {
            self.alias
                .as_ref()
                .map_or(path.is_empty(), |a| a.path == Path::new(path))
        }
        pub fn clear_input(&mut self) {
            self.input.clear();
            let _ = termios::tcflush(&self.master, FlushArg::TCIFLUSH);
        }
        pub fn reset(&mut self) {
            self.clear_input();
            self.output.clear();
            self.dropped = 0;
            let _ = termios::tcflush(&self.slave, FlushArg::TCIOFLUSH);
        }
        pub fn send(&mut self, bytes: &[u8]) {
            let overflow = (self.output.len() + bytes.len()).saturating_sub(QUEUE_LIMIT);
            self.dropped = self.dropped.saturating_add(overflow as u64);
            self.output.drain(..overflow.min(self.output.len()));
            self.output.extend(
                bytes
                    .iter()
                    .skip(bytes.len().saturating_sub(QUEUE_LIMIT))
                    .copied(),
            );
        }
        pub fn poll(&mut self) -> Result<(), String> {
            let mut buf = [0; IO_BUDGET];
            let room = (QUEUE_LIMIT - self.input.len()).min(buf.len());
            if room != 0 {
                match self.master.read(&mut buf[..room]) {
                    Ok(n) => self.input.extend(&buf[..n]),
                    Err(e) if transient(&e) => {}
                    Err(e) => return Err(format!("Serial PTY read: {e}")),
                }
            }
            let (a, b) = self.output.as_slices();
            let bytes = if a.is_empty() { b } else { a };
            if !bytes.is_empty() {
                match self.master.write(&bytes[..bytes.len().min(IO_BUDGET)]) {
                    Ok(n) => {
                        self.output.drain(..n);
                    }
                    Err(e) if transient(&e) => {}
                    Err(e) => return Err(format!("Serial PTY write: {e}")),
                }
            }
            Ok(())
        }
    }
    fn transient(e: &io::Error) -> bool {
        matches!(
            e.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
        ) || e.raw_os_error() == Some(nix::libc::EIO)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn client(pty: &SerialPty) -> File {
            OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(OFlag::O_NONBLOCK.bits())
                .open(pty.path())
                .unwrap()
        }
        fn read_available(file: &mut File) -> Vec<u8> {
            let mut result = Vec::new();
            let mut buf = [0; 4096];
            loop {
                match file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => result.extend(&buf[..n]),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => panic!("{e}"),
                }
            }
            result
        }
        #[test]
        fn raw_bytes_reconnect_and_descriptor_ownership() {
            let mut pty = SerialPty::open("").unwrap();
            for fd in [&pty.master, &pty.slave] {
                assert_ne!(
                    fcntl(fd, FcntlArg::F_GETFD).unwrap() & FdFlag::FD_CLOEXEC.bits(),
                    0
                );
            }
            for _ in 0..2 {
                let mut client = client(&pty);
                let bytes = b"\0\r\n\x1b[2J\x80\xff";
                client.write_all(bytes).unwrap();
                pty.poll().unwrap();
                assert_eq!(pty.input.drain(..).collect::<Vec<_>>(), bytes);
                pty.send(bytes);
                pty.poll().unwrap();
                assert_eq!(read_available(&mut client), bytes);
            }
            pty.poll().unwrap(); // No client: no failure, no lost descriptor.
        }
        #[test]
        fn slow_reader_is_bounded_and_partial_writes_preserve_order() {
            let mut pty = SerialPty::open("").unwrap();
            let mut client = client(&pty);
            let bytes: Vec<_> = (0..QUEUE_LIMIT * 2).map(|i| (i % 251) as u8).collect();
            pty.send(&bytes);
            assert_eq!(pty.output.len(), QUEUE_LIMIT);
            assert_eq!(pty.dropped, QUEUE_LIMIT as u64);
            for _ in 0..100 {
                pty.poll().unwrap();
            }
            assert!(!pty.output.is_empty()); // Kernel backpressure retained the remainder.
            let mut received = Vec::new();
            for _ in 0..100 {
                received.extend(read_available(&mut client));
                pty.poll().unwrap();
                if received.len() == QUEUE_LIMIT {
                    break;
                }
            }
            assert_eq!(received, bytes[QUEUE_LIMIT..]);
            pty.input.extend([1, 2, 3]);
            pty.send(b"stale");
            pty.reset();
            assert!(pty.input.is_empty() && pty.output.is_empty());
        }
        #[test]
        fn alias_collision_cleanup_and_replacement_are_owned() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("serial");
            fs::write(&path, "keep").unwrap();
            assert!(SerialPty::open(path.to_str().unwrap()).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), "keep");
            fs::remove_file(&path).unwrap();
            symlink("/nonexistent/foreign", &path).unwrap();
            assert!(SerialPty::open(path.to_str().unwrap()).is_err());
            assert_eq!(
                fs::read_link(&path).unwrap(),
                Path::new("/nonexistent/foreign")
            );
            fs::remove_file(&path).unwrap();
            let pty = SerialPty::open(path.to_str().unwrap()).unwrap();
            assert_eq!(fs::read_link(&path).unwrap(), pty.path());
            assert!(SerialPty::open(path.to_str().unwrap()).is_err());
            drop(pty);
            assert!(fs::symlink_metadata(&path).is_err());
            let pty = SerialPty::open(path.to_str().unwrap()).unwrap();
            fs::remove_file(&path).unwrap();
            fs::write(&path, "replacement").unwrap();
            drop(pty);
            assert_eq!(fs::read_to_string(&path).unwrap(), "replacement");
        }
        #[test]
        fn recovers_only_recorded_dead_alias_and_preserves_foreign_lock() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("serial");
            let lock = dir.path().join("serial.heliacal-lock");
            fs::write(&lock, "").unwrap();
            assert!(SerialPty::open(path.to_str().unwrap()).is_err());
            assert_eq!(fs::read(&lock).unwrap(), b"");
            fs::write(&lock, format!("{MAGIC}/nonexistent/previous-pty")).unwrap();
            symlink("/nonexistent/previous-pty", &path).unwrap();
            let pty = SerialPty::open(path.to_str().unwrap()).unwrap();
            assert_eq!(fs::read_link(&path).unwrap(), pty.path());
            drop(pty);
            assert!(fs::symlink_metadata(&path).is_err());
        }
    }
}
#[cfg(unix)]
pub use unix::SerialPty;

#[cfg(not(unix))]
pub struct SerialPty {
    pub input: VecDeque<u8>,
    pub dropped: u64,
}
#[cfg(not(unix))]
impl SerialPty {
    pub fn open(_: &str) -> Result<Self, String> {
        Err("Serial PTYs require a Unix host".into())
    }
    pub fn path(&self) -> &Path {
        Path::new("")
    }
    pub fn endpoint(&self) -> &Path {
        self.path()
    }
    pub fn alias_matches(&self, _: &str) -> bool {
        false
    }
    pub fn clear_input(&mut self) {
        self.input.clear();
    }
    pub fn reset(&mut self) {
        self.clear_input();
    }
    pub fn send(&mut self, _: &[u8]) {}
    pub fn poll(&mut self) -> Result<(), String> {
        Ok(())
    }
}
