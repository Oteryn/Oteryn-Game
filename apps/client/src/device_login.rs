//! T067 consumer: browser bootstrap, process sessions and opt-in OS-vault families.
mod attempt;
pub mod vault;
use crate::play::MiscellaneousPreferences;
pub use attempt::DeviceAttempt;
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::{
    device_sessions::{DeviceCredential, DeviceSession, SessionKey, now},
    native_login::{NativeLoginClient, NativeLoginError},
};
use std::{path::PathBuf, sync::Arc, time::Duration};

const CREDENTIAL_LOCK_TIMEOUT: Duration = Duration::from_secs(1);
#[derive(Default)]
struct State {
    session: Option<DeviceSession>,
    pending: Option<DeviceCredential>,
    key: Option<SessionKey>,
    remembered: bool,
}
pub struct DeviceLogin {
    enabled: bool,
    vault: Arc<dyn vault::Vault>,
    state: Arc<tokio::sync::Mutex<State>>,
    folder: Option<PathBuf>,
}
impl Default for DeviceLogin {
    fn default() -> Self {
        Self {
            enabled: false,
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
        if !self.enabled {
            return Ok(None);
        }
        let mut state = self.state.clone().lock_owned().await;
        let lock = self.lock_credential().await?;
        if let Some(pending) = state.pending.take() {
            let _ = client
                .revoke_device(&pending, CancellationToken::new())
                .await;
        }
        self.load_remembered(&mut state)?;
        let remembered = options.remember_device && self.vault.available();
        let mut attempt = DeviceAttempt {
            state,
            vault: self.vault.clone(),
            _lock: lock,
            remembered,
            options,
            folder: self.folder.clone(),
        };
        if !options.stay_logged_in || attempt.state.remembered != remembered {
            attempt.sign_out(client).await;
        }
        if cancel.is_cancelled() {
            attempt.sign_out(client).await;
            return Err(storage_error());
        }
        Ok(Some(attempt))
    }
    async fn lock_credential(&self) -> Result<Option<std::fs::File>, NativeLoginError> {
        let Some(folder) = self.folder.clone() else {
            return Ok(None);
        };
        tokio::task::spawn_blocking(move || {
            use std::{fs::TryLockError, time::Instant};

            std::fs::create_dir_all(&folder).map_err(|_| storage_error())?;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(folder.join("credential.lock"))
                .map_err(|_| storage_error())?;
            let deadline = Instant::now() + CREDENTIAL_LOCK_TIMEOUT;
            loop {
                match file.try_lock() {
                    Ok(()) => return Ok(Some(file)),
                    Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => return Err(storage_error()),
                }
            }
        })
        .await
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
        let state = tokio::time::timeout(CREDENTIAL_LOCK_TIMEOUT, self.state.clone().lock_owned())
            .await
            .map_err(|_| storage_error())?;
        let lock = self.lock_credential().await?;
        let mut attempt = DeviceAttempt {
            state,
            vault: self.vault.clone(),
            _lock: lock,
            remembered: false,
            options: MiscellaneousPreferences::default(),
            folder: self.folder.clone(),
        };
        self.load_remembered(&mut attempt.state)?;
        attempt.revoke(client).await
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
mod test_server;
#[cfg(test)]
mod tests;
