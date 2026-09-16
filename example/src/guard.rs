//! Nonblocking process exclusion on macOS/Linux. The kernel owns the lock.
//! Never unlink the file: replacing its inode lets another process bypass a
//! live owner's lock. PID/timestamp are diagnostic metadata, not ownership.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::error::AppError;

#[allow(dead_code)] // Reference primitive; the scaffold updater returns instructions.
pub struct DuplicateGuard {
    lock_path: PathBuf,
    file: Option<File>,
}

#[allow(dead_code)]
impl DuplicateGuard {
    pub fn new(data_dir: &Path, operation: &str) -> Self {
        Self {
            lock_path: data_dir.join("locks").join(format!("{operation}.lock")),
            file: None,
        }
    }

    pub fn acquire(&mut self, force: bool) -> Result<(), AppError> {
        if force || self.file.is_some() {
            return Ok(());
        }
        std::fs::create_dir_all(self.lock_path.parent().expect("lock directory"))?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&self.lock_path)?;
        // SAFETY: the descriptor is valid and remains open for the lock lifetime.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = std::io::Error::last_os_error();
            return if error.kind() == std::io::ErrorKind::WouldBlock {
                Err(AppError::OperationBusy)
            } else {
                Err(error.into())
            };
        }
        let metadata = serde_json::json!({
            "pid": std::process::id(),
            "started_at": chrono::Utc::now().to_rfc3339(),
            "operation": self.lock_path.file_stem().unwrap_or_default().to_string_lossy(),
        });
        let contents = serde_json::to_vec(&metadata).map_err(|_| AppError::Serialization)?;
        file.set_len(0)?;
        file.write_all(&contents)?;
        // Early errors above close the local File and release only our lock.
        self.file = Some(file);
        Ok(())
    }
}
// Dropping File releases the OS lock, including during unwinding. Process death
// also releases it. No custom Drop or file deletion is needed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_and_forced_callers_cannot_release_the_owner() {
        let tmp = tempfile::tempdir().unwrap();
        let mut owner = DuplicateGuard::new(tmp.path(), "op");
        owner.acquire(false).unwrap();
        let contents = std::fs::read(&owner.lock_path).unwrap();
        {
            let mut rejected = DuplicateGuard::new(tmp.path(), "op");
            assert_eq!(rejected.acquire(false).unwrap_err().exit_code(), 3);
        }
        {
            let mut forced = DuplicateGuard::new(tmp.path(), "op");
            forced.acquire(true).unwrap();
        }
        assert_eq!(std::fs::read(&owner.lock_path).unwrap(), contents);
        let mut third = DuplicateGuard::new(tmp.path(), "op");
        assert_eq!(
            third.acquire(false).unwrap_err().error_code(),
            "operation_busy"
        );
        let path = owner.lock_path.clone();
        drop(owner);
        assert!(path.exists(), "keep the inode stable");
        third.acquire(false).unwrap();
    }

    #[test]
    fn abandoned_metadata_does_not_block_new_owners() {
        let tmp = tempfile::tempdir().unwrap();
        let mut guard = DuplicateGuard::new(tmp.path(), "op");
        std::fs::create_dir_all(guard.lock_path.parent().unwrap()).unwrap();
        std::fs::write(&guard.lock_path, b"invalid or stale metadata").unwrap();
        guard.acquire(false).unwrap();
    }

    #[test]
    fn simultaneous_callers_have_exactly_one_owner() {
        let tmp = tempfile::tempdir().unwrap();
        let barrier = std::sync::Barrier::new(8);
        let outcomes = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        let mut guard = DuplicateGuard::new(tmp.path(), "op");
                        barrier.wait();
                        let result = guard.acquire(false);
                        barrier.wait(); // Keep the winner alive until all have attempted.
                        result
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(
            outcomes
                .iter()
                .filter_map(|r| r.as_ref().err())
                .all(|e| e.error_code() == "operation_busy")
        );
    }

    #[test]
    fn different_resources_do_not_block_each_other() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = DuplicateGuard::new(tmp.path(), "a");
        let mut b = DuplicateGuard::new(tmp.path(), "b");
        a.acquire(false).unwrap();
        b.acquire(false).unwrap();
    }
}
