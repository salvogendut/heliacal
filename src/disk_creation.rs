//! Explicit file creation is independent from saving or replacing a guest.
use std::{
    path::PathBuf,
    thread::{self, JoinHandle},
};

pub struct Creation {
    path: PathBuf,
    target: usize,
    worker: Option<JoinHandle<Result<(), String>>>,
}

impl Creation {
    pub fn start(path: PathBuf, mib: u64, target: usize) -> Result<Self, String> {
        if !(1..=2_097_152).contains(&mib) || target >= 7 {
            return Err("Invalid disk capacity or SCSI target.".into());
        }
        let output = path.clone();
        let worker = thread::Builder::new()
            .name("chd-create".into())
            .spawn(move || {
                helios::media::create_disk(
                    output,
                    helios::media::ImageFormat::Chd,
                    mib * 1024 * 1024,
                )
                .map(|_| ())
                .map_err(|e| e.to_string())
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            target,
            worker: Some(worker),
        })
    }
    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(|w| w.is_finished())
    }
    pub fn finish(mut self) -> Result<(PathBuf, usize), String> {
        self.worker
            .take()
            .unwrap()
            .join()
            .map_err(|_| "Disk creation worker stopped unexpectedly.")??;
        Ok((self.path.clone(), self.target))
    }
}
impl Drop for Creation {
    fn drop(&mut self) {
        // On an exceptional frontend exit, finish the bounded sparse creation
        // so its private staging directory is cleaned rather than orphaned.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creation_publishes_a_blank_disk_and_cannot_overwrite_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.chd");
        let (actual, target) = Creation::start(path.clone(), 2, 3)
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!((actual, target), (path.clone(), 3));
        let before = std::fs::read(&path).unwrap();
        assert_eq!(
            helios::media::inspect(&path, helios::media::MediaKind::Disk)
                .unwrap()
                .logical_bytes,
            2 * 1024 * 1024
        );
        assert!(
            Creation::start(path.clone(), 1, 3)
                .unwrap()
                .finish()
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn invalid_creation_request_and_missing_directory_have_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing/new.chd");
        for (size, target) in [(0, 3), (2_097_153, 3), (1, 7)] {
            assert!(Creation::start(path.clone(), size, target).is_err());
        }
        assert!(
            Creation::start(path.clone(), 1, 3)
                .unwrap()
                .finish()
                .is_err()
        );
        assert!(!path.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
