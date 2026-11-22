use super::*;

impl DeviceLogin {
    fn intent_path(&self) -> Option<PathBuf> {
        self.folder
            .as_ref()
            .map(|folder| folder.join("revoke.pending"))
    }

    /// Fence this process immediately and publish the intent before spawning a worker.
    pub(crate) fn block_remembered(&self) -> Result<(), NativeLoginError> {
        let process = self.journal.record_process_intent()?;
        if !process && let Some(path) = self.intent_path() {
            if let Some(bytes) = self.vault.read().map_err(|_| storage_error())?
                && bytes.len() <= 1024
                && let Ok(session) = serde_json::from_slice::<DeviceSession>(&bytes)
            {
                self.journal.stage(&session.device_credential)?;
            }
            crate::atomic_file::replace(
                &path, b"revoke
", true,
            )
            .map_err(|_| storage_error())?;
            self.sync_intent_directory()?;
        }
        self.forget_requested.store(true, Ordering::Release);
        Ok(())
    }

    pub(super) fn revocation_pending(&self) -> Result<bool, NativeLoginError> {
        self.intent_path().map_or(Ok(false), |path| {
            path.try_exists().map_err(|_| storage_error())
        })
    }

    /// The caller holds the credential lock and has successfully cleared the vault.
    pub(super) fn complete_local_revocation(&self) -> Result<(), NativeLoginError> {
        if let Some(path) = self.intent_path() {
            match std::fs::remove_file(path) {
                Ok(()) => self.sync_intent_directory()?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(storage_error()),
            }
        }
        self.forget_requested.store(false, Ordering::Release);
        Ok(())
    }

    fn sync_intent_directory(&self) -> Result<(), NativeLoginError> {
        #[cfg(unix)]
        if let Some(folder) = &self.folder {
            std::fs::File::open(folder)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| storage_error())?;
        }
        Ok(())
    }
}
