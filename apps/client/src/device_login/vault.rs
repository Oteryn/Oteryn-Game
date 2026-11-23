//! OS credential-vault boundary. Ordinary files contain only locks and selection metadata.
use std::sync::Arc;
pub trait Vault: Send + Sync {
    fn available(&self) -> bool;
    fn read(&self) -> Result<Option<Vec<u8>>, String>;
    fn write(&self, bytes: &[u8]) -> Result<(), String>;
    /// A process-only family can retain its retry credential in memory when
    /// the optional persistent backend is unavailable.
    fn write_process(&self, bytes: &[u8]) -> Result<bool, String> {
        if self.available() {
            self.write(bytes)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn clear(&self) -> Result<(), String>;
}
#[cfg(windows)]
#[path = "vault_windows.rs"]
mod windows;
pub fn system(target: String) -> Arc<dyn Vault> {
    #[cfg(windows)]
    {
        Arc::new(windows::WindowsVault(target))
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        Arc::new(Unavailable)
    }
}
#[cfg(not(windows))]
struct Unavailable;
#[cfg(not(windows))]
impl Vault for Unavailable {
    fn available(&self) -> bool {
        false
    }
    fn read(&self) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
    fn write(&self, _: &[u8]) -> Result<(), String> {
        Err("OS credential vault unavailable".into())
    }
    fn clear(&self) -> Result<(), String> {
        Ok(())
    }
}

pub(super) fn revocations(target: String) -> Arc<dyn Vault> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(secret_service::SecretService(target))
    }
    #[cfg(not(target_os = "linux"))]
    {
        system(target)
    }
}

#[cfg(target_os = "linux")]
#[path = "vault_secret_service.rs"]
mod secret_service;
