//! Monotone nonrollback T94 fence shared by every publisher of a namespace.
use crate::account_characters::{EpochFence, EpochFenceFile, FenceUnusable, open_directory};
use rustix::fs::{FlockOperation, flock};
use std::path::PathBuf;

pub struct LeagueEpochFenceFile {
    path: PathBuf,
    inner: EpochFenceFile,
}
impl LeagueEpochFenceFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            inner: EpochFenceFile::new(path.clone()),
            path,
        }
    }
}
impl EpochFence for LeagueEpochFenceFile {
    fn read(&mut self) -> Result<u64, FenceUnusable> {
        self.inner.read()
    }
    fn persist(&mut self, epoch: u64) -> Result<(), FenceUnusable> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or(FenceUnusable)?;
        // The directory inode survives atomic fence-file replacement. A fresh FD
        // makes separate publisher instances/processes contend on the same lock.
        let lock = open_directory(parent)?;
        flock(&lock, FlockOperation::NonBlockingLockExclusive).map_err(|_| FenceUnusable)?;
        let current = self.inner.read()?;
        if epoch < current {
            return Err(FenceUnusable);
        }
        if epoch > current {
            self.inner.persist(epoch)?;
        }
        // Dropping the FD releases the lock on every success/refusal path.
        Ok(())
    }
}
