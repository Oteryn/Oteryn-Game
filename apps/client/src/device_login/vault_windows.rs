//! Windows Credential Manager owns encryption and credential persistence.
#![allow(unsafe_code)] // Exact bounded Credential Manager buffers; all returned memory is freed.
use super::Vault;
use windows_sys::Win32::{
    Foundation::{ERROR_NOT_FOUND, GetLastError},
    Security::Credentials::*,
};
pub(super) struct WindowsVault(pub String);
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
impl Vault for WindowsVault {
    fn available(&self) -> bool {
        true
    }
    fn read(&self) -> Result<Option<Vec<u8>>, String> {
        let target = wide(&self.0);
        let mut credential = std::ptr::null_mut();
        // SAFETY: target is terminated and output points to an initialized null pointer.
        if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
            // SAFETY: same-thread error read immediately follows the failed API call.
            return if unsafe { GetLastError() } == ERROR_NOT_FOUND {
                Ok(None)
            } else {
                Err("OS vault read failed".into())
            };
        }
        // SAFETY: success guarantees a valid CREDENTIALW allocation until CredFree.
        let result = unsafe {
            let c = &*credential;
            let bytes = if c.CredentialBlobSize == 0
                || c.CredentialBlobSize > 1024
                || c.CredentialBlob.is_null()
            {
                Err("Invalid OS vault credential".into())
            } else {
                Ok(Some(
                    std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize)
                        .to_vec(),
                ))
            };
            CredFree(credential.cast());
            bytes
        };
        result
    }
    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        if bytes.is_empty() || bytes.len() > 1024 {
            return Err("Invalid OS vault credential size".into());
        }
        let mut target = wide(&self.0);
        let mut user = wide("Oteryn native session");
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: bytes.len() as u32,
            CredentialBlob: bytes.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: user.as_mut_ptr(),
            ..Default::default()
        };
        // SAFETY: every pointed-to buffer lives until this synchronous API finishes; the API copies it.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            Err("OS vault write failed".into())
        } else {
            Ok(())
        }
    }
    fn clear(&self) -> Result<(), String> {
        let target = wide(&self.0);
        // SAFETY: target is a live, terminated UTF-16 string; no borrowed data is retained.
        if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } != 0 {
            return Ok(());
        }
        // SAFETY: last-error is read on the thread that just made the failed call.
        if unsafe { GetLastError() } == ERROR_NOT_FOUND {
            Ok(())
        } else {
            Err("OS vault delete failed".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_credential_manager_round_trips_only_the_synthetic_device_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let vault = WindowsVault(format!(
            "Oteryn/Test/NativeDevice/{}/{stamp}",
            std::process::id()
        ));
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/contracts/fixtures/remembered-device-v1/enroll-response.json"
        ));
        vault.write(bytes)?;
        let loaded = vault.read()?;
        vault.clear()?;
        assert_eq!(loaded.as_deref(), Some(bytes.as_slice()));
        assert!(vault.read()?.is_none());
        Ok(())
    }
}
