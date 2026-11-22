use super::*;
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, sync::Mutex};

type ResolveVault = dyn Fn(&str) -> Arc<dyn vault::Vault> + Send + Sync;
const PREFIX: &str = "revoke-credential-";

#[derive(Default)]
struct ProcessCredential {
    credential: Option<DeviceCredential>,
    forgetting: bool,
    pending: Vec<DeviceCredential>,
}

pub(super) struct Journal {
    folder: Option<PathBuf>,
    resolve: Arc<ResolveVault>,
    process: Mutex<ProcessCredential>,
}

impl Journal {
    pub(super) fn new(folder: Option<PathBuf>) -> Self {
        let namespace = folder
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self::with_vault(
            folder,
            Arc::new(move |reference| {
                vault::revocations(format!("Oteryn/NativeRevoke/{namespace}/{reference}"))
            }),
        )
    }

    pub(super) fn with_vault(folder: Option<PathBuf>, resolve: Arc<ResolveVault>) -> Self {
        Self {
            folder,
            resolve,
            process: Mutex::default(),
        }
    }

    /// Returns true when a specific process credential, rather than the remembered vault
    /// entry, is the target. The marker contains only a digest that names a vault entry.
    pub(super) fn record_process_intent(&self) -> Result<bool, NativeLoginError> {
        let mut process = self.process.lock().map_err(|_| storage_error())?;
        if let Some(credential) = process.credential.clone() {
            self.stage_process(&mut process, &credential)?;
            process.forgetting = true;
            return Ok(true);
        }
        process.forgetting = true;
        Ok(false)
    }

    pub(super) fn observe_process_credential(
        &self,
        credential: Option<&DeviceCredential>,
    ) -> Result<(), NativeLoginError> {
        let mut process = self.process.lock().map_err(|_| storage_error())?;
        if process.forgetting
            && let Some(credential) = credential
        {
            self.stage_process(&mut process, credential)?;
        }
        process.credential = credential.cloned();
        Ok(())
    }

    pub(super) fn finish(&self) -> Result<(), NativeLoginError> {
        *self.process.lock().map_err(|_| storage_error())? = ProcessCredential::default();
        Ok(())
    }

    pub(super) fn stage(&self, credential: &DeviceCredential) -> Result<(), NativeLoginError> {
        self.publish(credential, false).map(|_| ())
    }

    fn stage_process(
        &self,
        process: &mut ProcessCredential,
        credential: &DeviceCredential,
    ) -> Result<(), NativeLoginError> {
        if !self.publish(credential, true)? && !process.pending.contains(credential) {
            process.pending.push(credential.clone());
        }
        Ok(())
    }

    fn publish(
        &self,
        credential: &DeviceCredential,
        process_only: bool,
    ) -> Result<bool, NativeLoginError> {
        let Some(folder) = &self.folder else {
            return Ok(false);
        };
        let reference = credential_reference(credential.expose());
        let marker = folder.join(format!("{PREFIX}{reference}"));
        let _publication = crate::atomic_file::try_lock(&folder.join("revocations"))
            .map_err(|_| storage_error())?;
        let vault = (self.resolve)(&reference);
        let bytes = credential.expose().as_bytes();
        if process_only {
            if !vault.write_process(bytes).map_err(|_| storage_error())? {
                return Ok(false);
            }
        } else {
            vault.write(bytes).map_err(|_| storage_error())?;
        }
        crate::atomic_file::replace(&marker, reference.as_bytes(), true)
            .map_err(|_| storage_error())?;
        sync_directory(folder)?;
        Ok(true)
    }

    pub(super) async fn replay(&self, client: &NativeLoginClient) -> Result<(), NativeLoginError> {
        let pending = self
            .process
            .lock()
            .map_err(|_| storage_error())?
            .pending
            .clone();
        for credential in pending {
            client
                .revoke_device(&credential, CancellationToken::new())
                .await?;
            self.process
                .lock()
                .map_err(|_| storage_error())?
                .pending
                .retain(|item| item != &credential);
        }
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        let entries = match std::fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(storage_error()),
        };
        for entry in entries {
            let entry = entry.map_err(|_| storage_error())?;
            let name = entry.file_name();
            let Some(reference) = name.to_str().and_then(|name| name.strip_prefix(PREFIX)) else {
                continue;
            };
            let marker = entry.path();
            self.revoke_reference(client, &marker, reference).await?;
        }
        Ok(())
    }

    async fn revoke_reference(
        &self,
        client: &NativeLoginClient,
        marker: &Path,
        reference: &str,
    ) -> Result<(), NativeLoginError> {
        let mut readback = String::new();
        std::fs::File::open(marker)
            .and_then(|file| file.take(65).read_to_string(&mut readback))
            .map_err(|_| storage_error())?;
        if reference.len() != 64
            || !reference.bytes().all(|byte| byte.is_ascii_hexdigit())
            || readback != reference
        {
            return Err(storage_error());
        }
        let vault = (self.resolve)(reference);
        let bytes = vault
            .read()
            .map_err(|_| storage_error())?
            .ok_or_else(storage_error)?;
        let credential =
            DeviceCredential::parse(std::str::from_utf8(&bytes).map_err(|_| storage_error())?)?;
        if credential_reference(credential.expose()) != reference {
            return Err(storage_error());
        }
        // The revoke route acknowledges already-absent families with the same success body.
        client
            .revoke_device(&credential, CancellationToken::new())
            .await?;
        let _publication = crate::atomic_file::lock(&marker.with_file_name("revocations"))
            .map_err(|_| storage_error())?;
        std::fs::remove_file(marker).map_err(|_| storage_error())?;
        if let Some(folder) = &self.folder {
            sync_directory(folder)?;
        }
        vault.clear().map_err(|_| storage_error())
    }

    pub(super) fn has_folder(&self) -> bool {
        self.folder.is_some()
    }
}

pub(super) fn sync_directory(path: &Path) -> Result<(), NativeLoginError> {
    #[cfg(unix)]
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| storage_error())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn credential_reference(credential: &str) -> String {
    Sha256::digest(credential.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
