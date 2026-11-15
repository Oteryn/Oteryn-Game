//! Monotone nonrollback T94 fence shared by every publisher of a namespace.
use crate::account_characters::{EpochFence, EpochFenceFile, FenceUnusable};
use std::path::PathBuf;

pub struct LeagueEpochFenceFile {
    inner: EpochFenceFile,
}
impl LeagueEpochFenceFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            inner: EpochFenceFile::new(path),
        }
    }
}
impl EpochFence for LeagueEpochFenceFile {
    fn read(&mut self) -> Result<u64, FenceUnusable> {
        self.inner.read()
    }
    fn persist(&mut self, epoch: u64) -> Result<(), FenceUnusable> {
        // The shared writer locks the stable directory inode and rechecks F.
        self.inner.persist(epoch)
    }
}
