//! Persistent user consent for shared native confirmation dialogs.
use oteryn_client_ui::modal_dialogs::DismissedNotices;
use std::{
    io,
    path::Path,
    sync::{Mutex, PoisonError},
};
static SAVE: Mutex<()> = Mutex::new(());

pub(in crate::play) fn should_ask(path: &Path, key: &str) -> io::Result<bool> {
    DismissedNotices::load(path)
        .map(|value| !value.contains(key))
        .map_err(|error| io::Error::other(error.to_string()))
}
pub(in crate::play) fn record(
    path: &Path,
    key: &str,
    accepted: bool,
    dont_show_again: bool,
) -> io::Result<bool> {
    if !accepted {
        return Ok(false);
    }
    if dont_show_again {
        // Merge the latest choices under a process lock; two native windows cannot lose each other's dismissals.
        let _save = SAVE.lock().unwrap_or_else(PoisonError::into_inner);
        let mut value =
            DismissedNotices::load(path).map_err(|error| io::Error::other(error.to_string()))?;
        if !value.dismiss(key) {
            return Err(io::Error::other("Invalid confirmation preference"));
        }
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        value.save(path)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_confirmation_cancel_never_persists_and_confirm_survives_reload()
    -> Result<(), Box<dyn std::error::Error>> {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("cg-modal-confirm-{}-{suffix}", std::process::id()));
        let path = directory.join("notices.json");
        assert!(should_ask(&path, "mail-post:v1")?);
        assert!(!record(&path, "mail-post:v1", false, true)?);
        assert!(!path.exists());
        assert!(record(&path, "mail-post:v1", true, true)?);
        assert!(!should_ask(&path, "mail-post:v1")?);
        assert!(should_ask(&path, "mail-post:v2")?);
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }
}
