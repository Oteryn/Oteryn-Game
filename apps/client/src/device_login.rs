//! T067 consumer: browser bootstrap, process sessions and opt-in OS-vault families.
mod attempt;
mod revocation_intent;
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
    vault: Arc<dyn vault::Vault>,
    state: Arc<tokio::sync::Mutex<State>>,
    folder: Option<PathBuf>,
}
impl Default for DeviceLogin {
    fn default() -> Self {
        Self {
            enabled: false,
            forget_requested: AtomicBool::new(false),
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
        if !self.enabled && !self.revocation_pending()? {
            return Ok(None);
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
        let forget =
            pending || self.forget_requested.load(Ordering::Acquire) || state.vault_clear_failed;
        if self.load_remembered(&mut state).is_err() {
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
        };
        if forget || !options.stay_logged_in || attempt.state.remembered != remembered {
            attempt.sign_out(client).await;
            if forget && !attempt.state.vault_clear_failed {
                self.complete_local_revocation()?;
            } else if forget {
                return Ok(None);
            }
        } else {
            let _ = attempt.revoke_pending(client).await;
        }
        if cancel.is_cancelled() {
            attempt.sign_out(client).await;
            return Err(storage_error());
        }
        Ok(self.enabled.then_some(attempt))
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
            // Another admission or worker already cleared the durable credential.
            let mut state = state;
            state.session = None;
            state.remembered = false;
            state.key = None;
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
        };
        let known_session = attempt.state.session.clone();
        let loaded = self.load_remembered(&mut attempt.state);
        if loaded.is_err() && attempt.state.session.is_none() {
            attempt.state.session = known_session;
        }
        let revoked = attempt.revoke(client).await;
        if loaded.is_ok() && !attempt.state.vault_clear_failed {
            self.complete_local_revocation()?;
        }
        loaded.and(revoked)
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
