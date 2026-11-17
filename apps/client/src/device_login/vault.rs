//! OS credential-vault boundary. Ordinary files contain only locks and selection metadata.
use std::sync::Arc;
pub trait Vault: Send + Sync {
    fn available(&self) -> bool;
    fn read(&self) -> Result<Option<Vec<u8>>, String>;
    fn write(&self, bytes: &[u8]) -> Result<(), String>;
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
