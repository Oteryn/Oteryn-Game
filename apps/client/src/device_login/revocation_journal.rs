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
        self.publish(credential, false, None).map(|_| ())
    }

    pub(super) fn stage_rotation(&self, session: &DeviceSession) -> Result<(), NativeLoginError> {
        self.publish(&session.device_credential, false, Some(&session.family_id))
            .map(|_| ())
    }

    fn stage_process(
        &self,
        process: &mut ProcessCredential,
        credential: &DeviceCredential,
    ) -> Result<(), NativeLoginError> {
        if !self.publish(credential, true, None)? && !process.pending.contains(credential) {
            process.pending.push(credential.clone());
        }
        Ok(())
    }

    fn publish(
        &self,
        credential: &DeviceCredential,
        process_only: bool,
        rotation_family: Option<&str>,
    ) -> Result<bool, NativeLoginError> {
        let Some(folder) = &self.folder else {
            return Ok(false);
        };
        let reference = reference_digest(credential.expose());
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
        let contents = match rotation_family {
            Some(family) => rotation_marker(&reference, family),
            None => reference,
        };
        crate::atomic_file::replace(&marker, contents.as_bytes(), true)
            .map_err(|_| storage_error())?;
        sync_directory(folder)?;
        Ok(true)
    }

    pub(super) async fn replay(
        &self,
        client: &NativeLoginClient,
        deferred: &[DeviceCredential],
        recovery_vault: Option<&dyn vault::Vault>,
    ) -> Result<(), NativeLoginError> {
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
            // The live attempt retries these under its recovery deadline. After a
            // restart the in-memory queue is empty and every marker is replayed.
            if deferred
                .iter()
                .any(|credential| reference_digest(credential.expose()) == reference)
            {
                continue;
            }
            let marker = entry.path();
            self.revoke_reference(client, &marker, reference, recovery_vault)
                .await?;
        }
        Ok(())
    }

    async fn revoke_reference(
        &self,
        client: &NativeLoginClient,
        marker: &Path,
        reference: &str,
        recovery_vault: Option<&dyn vault::Vault>,
    ) -> Result<(), NativeLoginError> {
        let readback = read_marker(marker)?;
        let (stored_reference, rotation_family) = readback
            .split_once('\n')
            .map_or((readback.as_str(), None), |(reference, family)| {
                (reference, Some(family))
            });
        if !valid_reference(reference)
            || stored_reference != reference
            || rotation_family.is_some_and(|family| !valid_reference(family))
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
        if reference_digest(credential.expose()) != reference {
            return Err(storage_error());
        }
        // This marker was written before rotating. The same family with a
        // different credential proves the successor reached the live vault.
        if let (Some(family), Some(live_vault)) = (rotation_family, recovery_vault) {
            let when = now()?;
            let saved = live_vault
                .read()
                .map_err(|_| storage_error())?
                .and_then(|bytes| DeviceSession::from_vault(&bytes, when).ok());
            if saved.is_some_and(|session| {
                family == reference_digest(&session.family_id)
                    && session.device_credential != credential
            }) && self.discard_marker(&credential, Some(&readback))?
            {
                return Ok(());
            }
        }
        // The revoke route acknowledges already-absent families with the same success body.
        client
            .revoke_device(&credential, CancellationToken::new())
            .await?;
        self.discard(&credential)
    }

    pub(super) fn discard(&self, credential: &DeviceCredential) -> Result<(), NativeLoginError> {
        self.discard_marker(credential, None).map(|_| ())
    }

    pub(super) fn discard_rotation(
        &self,
        credential: &DeviceCredential,
        family: &str,
    ) -> Result<(), NativeLoginError> {
        let marker = rotation_marker(&reference_digest(credential.expose()), family);
        self.discard_marker(credential, Some(&marker)).map(|_| ())
    }

    fn discard_marker(
        &self,
        credential: &DeviceCredential,
        expected: Option<&str>,
    ) -> Result<bool, NativeLoginError> {
        let Some(folder) = &self.folder else {
            return Ok(true);
        };
        let reference = reference_digest(credential.expose());
        let marker = folder.join(format!("{PREFIX}{reference}"));
        let _publication =
            crate::atomic_file::lock(&folder.join("revocations")).map_err(|_| storage_error())?;
        if marker.try_exists().map_err(|_| storage_error())? {
            if let Some(expected) = expected
                && read_marker(&marker)? != expected
            {
                // An explicit revocation may have replaced the rotation marker.
                return Ok(false);
            }
            std::fs::remove_file(marker).map_err(|_| storage_error())?;
            sync_directory(folder)?;
            (self.resolve)(&reference)
                .clear()
                .map_err(|_| storage_error())?;
        }
        Ok(true)
    }

    pub(super) fn has_folder(&self) -> bool {
        self.folder.is_some()
    }
}

fn read_marker(path: &Path) -> Result<String, NativeLoginError> {
    let mut contents = String::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(130).read_to_string(&mut contents))
        .map_err(|_| storage_error())?;
    Ok(contents)
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

fn valid_reference(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn rotation_marker(reference: &str, family: &str) -> String {
    format!("{reference}\n{}", reference_digest(family))
}

fn reference_digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
