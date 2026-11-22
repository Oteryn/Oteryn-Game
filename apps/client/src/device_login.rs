//! T067 consumer: browser bootstrap, process sessions and opt-in OS-vault families.
mod attempt;
mod revocation_intent;
mod revocation_journal;
pub mod vault;
use crate::play::MiscellaneousPreferences;
pub use attempt::DeviceAttempt;
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::{
    device_sessions::{DeviceCredential, DeviceSession, SessionKey, now},
    native_login::{NativeLoginClient, NativeLoginError},
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const CREDENTIAL_LOCK_TIMEOUT: Duration = Duration::from_secs(1);
#[derive(Default)]
struct State {
    session: Option<DeviceSession>,
    pending: Vec<DeviceCredential>,
    vault_clear_failed: bool,
    key: Option<SessionKey>,
    remembered: bool,
}
pub struct DeviceLogin {
    enabled: bool,
    forget_requested: AtomicBool,
    journal: Arc<revocation_journal::Journal>,
    vault: Arc<dyn vault::Vault>,
    state: Arc<tokio::sync::Mutex<State>>,
    folder: Option<PathBuf>,
}
impl Default for DeviceLogin {
    fn default() -> Self {
        Self {
            enabled: false,
            forget_requested: AtomicBool::new(false),
            journal: Arc::new(revocation_journal::Journal::new(None)),
            vault: vault::system("Oteryn/native-disabled".into()),
            state: Default::default(),
            folder: None,
        }
    }
}
impl DeviceLogin {
    pub fn configured(origin: &str, client: &str, enabled: bool) -> Self {
        let Ok(key) = oteryn_platform_client::device_sessions::vault_key(origin, client) else {
            return Self::default();
        };
        let folder = crate::play::PreferenceStore::path().and_then(|p| {
            p.parent()
                .map(|parent| parent.join("native-sessions").join(&key))
        });
        let Some(folder) = folder else {
            return Self::default();
        };
        Self {
            enabled,
            forget_requested: AtomicBool::new(false),
            journal: Arc::new(revocation_journal::Journal::new(Some(folder.clone()))),
            vault: vault::system(format!("Oteryn/NativeDevice/{key}")),
            state: Default::default(),
            folder: Some(folder),
        }
    }
    pub async fn begin(
        &self,
        client: &NativeLoginClient,
        options: MiscellaneousPreferences,
        cancel: CancellationToken,
    ) -> Result<Option<DeviceAttempt>, NativeLoginError> {
        if !self.enabled && self.folder.as_ref().is_none_or(|folder| !folder.exists()) {
            return Ok(None);
        }
        if cancel.is_cancelled() {
            return self.cancel_begin(client).await;
        }
        let Ok(mut state) =
            tokio::time::timeout(CREDENTIAL_LOCK_TIMEOUT, self.state.clone().lock_owned()).await
        else {
            return Ok(None);
        };
        let Ok(lock) = self.lock_credential().await else {
            return Ok(None);
        };
        let pending = self.revocation_pending()?;
        if !pending && self.journal.replay(client).await.is_err() {
            return Ok(None);
        }
        let requested = self.forget_requested.load(Ordering::Acquire);
        if requested && !pending && self.journal.has_folder() {
            state.session = None;
            state.key = None;
            state.remembered = false;
            self.journal.finish()?;
            self.forget_requested.store(false, Ordering::Release);
        }
        let forget =
            pending || (requested && !self.journal.has_folder()) || state.vault_clear_failed;
        if !forget && self.load_remembered(&mut state).is_err() {
            return Ok(None);
        }
        let remembered = !forget && options.remember_device && self.vault.available();
        let mut attempt = DeviceAttempt {
            state,
            vault: self.vault.clone(),
            _lock: lock,
            remembered,
            options,
            folder: self.folder.clone(),
            journal: self.journal.clone(),
        };
        if forget {
            if !pending && attempt.state.vault_clear_failed {
                if attempt.revoke_before_vault_clear(client).await.is_err() {
                    return Ok(None);
                }
            } else if self
                .finish_attempt_sign_out(&mut attempt, client)
                .await
                .is_err()
            {
                return Ok(None);
            }
        } else if !options.stay_logged_in || attempt.state.remembered != remembered {
            if self.folder.is_some() && attempt.state.remembered {
                attempt.revoke_before_vault_clear(client).await?;
            } else {
                attempt.sign_out(client).await;
            }
        } else {
            let _ = attempt.revoke_pending(client).await;
        }
        if cancel.is_cancelled() {
            drop(attempt);
            return self.cancel_begin(client).await;
        }
        Ok(self.enabled.then_some(attempt))
    }
    async fn cancel_begin(
        &self,
        client: &NativeLoginClient,
    ) -> Result<Option<DeviceAttempt>, NativeLoginError> {
        // Explicit UI cancellation has already queued its own worker.
        if !self.forget_requested.load(Ordering::Acquire) {
            self.sign_out(client).await?;
        }
        Err(storage_error())
    }

    async fn lock_credential(&self) -> Result<Option<std::fs::File>, NativeLoginError> {
        let Some(folder) = self.folder.clone() else {
            return Ok(None);
        };
        let deadline = std::time::Instant::now() + CREDENTIAL_LOCK_TIMEOUT;
        let worker = tokio::task::spawn_blocking(move || {
            use std::{fs::TryLockError, time::Instant};

            std::fs::create_dir_all(&folder).map_err(|_| storage_error())?;
            let path = folder.join("credential.lock");
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if !metadata.is_file() => return Err(storage_error()),
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(storage_error());
                }
                _ => {}
            }
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)
                .map_err(|_| storage_error())?;
            if !file.metadata().map_err(|_| storage_error())?.is_file() {
                return Err(storage_error());
            }
            while Instant::now() < deadline {
                match file.try_lock() {
                    Ok(()) => return Ok(Some(file)),
                    Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => return Err(storage_error()),
                }
            }
            Err(storage_error())
        });
        tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), worker)
            .await
            .map_err(|_| storage_error())?
            .map_err(|_| storage_error())?
    }

    fn load_remembered(&self, state: &mut State) -> Result<(), NativeLoginError> {
        if state.remembered {
            state.session = None;
        }
        if state.session.is_none() {
            match self.vault.read() {
                Ok(Some(bytes)) => match DeviceSession::from_vault(&bytes, now()?) {
                    Ok(value) => {
                        state.session = Some(value);
                        state.remembered = true;
                    }
                    Err(_) => {
                        self.vault.clear().map_err(|_| storage_error())?;
                    }
                },
                Ok(None) => {}
                Err(_) => {
                    self.vault.clear().map_err(|_| storage_error())?;
                }
            }
        }
        Ok(())
    }

    pub async fn sign_out(&self, client: &NativeLoginClient) -> Result<(), NativeLoginError> {
        self.block_remembered()?;
        self.finish_sign_out(client).await
    }

    pub(crate) async fn finish_sign_out(
        &self,
        client: &NativeLoginClient,
    ) -> Result<(), NativeLoginError> {
        let state = tokio::time::timeout(CREDENTIAL_LOCK_TIMEOUT, self.state.clone().lock_owned())
            .await
            .map_err(|_| storage_error())?;
        let lock = self.lock_credential().await?;
        if self.folder.is_some() && !self.revocation_pending()? {
            self.journal.replay(client).await?;
            let mut state = state;
            state.session = None;
            state.remembered = false;
            state.key = None;
            self.journal.finish()?;
            self.forget_requested.store(false, Ordering::Release);
            return Ok(());
        }
        let mut attempt = DeviceAttempt {
            state,
            vault: self.vault.clone(),
            _lock: lock,
            remembered: false,
            options: MiscellaneousPreferences::default(),
            folder: self.folder.clone(),
            journal: self.journal.clone(),
        };
        self.finish_attempt_sign_out(&mut attempt, client).await
    }

    pub(crate) async fn finish_attempt_sign_out(
        &self,
        attempt: &mut DeviceAttempt,
        client: &NativeLoginClient,
    ) -> Result<(), NativeLoginError> {
        if self.journal.has_folder() && !self.revocation_pending()? {
            self.journal.replay(client).await?;
            attempt.state.session = None;
            attempt.state.key = None;
            attempt.state.remembered = false;
        } else {
            attempt.revoke_before_vault_clear(client).await?;
            self.complete_local_revocation()?;
        }
        self.journal.finish()?;
        self.forget_requested.store(false, Ordering::Release);
        Ok(())
    }

    pub fn vault_available(&self) -> bool {
        self.vault.available()
    }
}
fn storage_error() -> NativeLoginError {
    oteryn_platform_client::PlatformClientError::Request.into()
}

#[cfg(windows)]
#[path = "device_login/confirmation.rs"]
mod confirmation;
#[cfg(windows)]
pub use confirmation::confirm_action;

#[cfg(test)]
mod revocation_tests;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod tests;
