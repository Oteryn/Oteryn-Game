//! Local confirmations and session choices; Platform retains all session authority.
use serde::{Deserialize, Serialize};
use std::{io, path::Path};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MiscellaneousPreferences {
    pub character_sort_column: u8,
    pub character_sort_descending: bool,
    pub stay_logged_in: bool,
    pub remember_device: bool,
    pub quick_login: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_engine: Option<oteryn_renderer::GraphicsEngine>,
    pub confirm_logout: bool,
    pub confirm_exit: bool,
}
impl Default for MiscellaneousPreferences {
    fn default() -> Self {
        Self {
            character_sort_column: 0,
            character_sort_descending: false,
            stay_logged_in: false,
            remember_device: false,
            quick_login: false,
            startup_engine: None,
            confirm_logout: true,
            confirm_exit: true,
        }
    }
}
impl MiscellaneousPreferences {
    pub fn path() -> Option<std::path::PathBuf> {
        super::PreferenceStore::path().map(|p| p.with_extension("session-options.json"))
    }
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| Self::read(&p).ok())
            .unwrap_or_default()
    }
    pub fn read(path: &std::path::Path) -> std::io::Result<Self> {
        if path.with_added_extension("opt-out").try_exists()? {
            match crate::atomic_file::try_lock(path) {
                Ok(_lock) => Self::replay_opt_out(path)?,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    let mut options = Self::read_sidecar(path)?;
                    options.remember_device = false;
                    return Ok(options);
                }
                Err(error) => return Err(error),
            }
        }
        Self::read_sidecar(path)
    }

    fn read_sidecar(path: &Path) -> io::Result<Self> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(4097)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 4096 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid session options size",
            ));
        }
        let options: Self = serde_json::from_slice(&bytes)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        if options.character_sort_column > 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid character sort column",
            ));
        }
        Ok(options)
    }
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let _lock = crate::atomic_file::lock(path)?;
        self.publish(path)
    }

    pub(super) fn publish_edits(
        &self,
        before: Self,
        path: &std::path::Path,
    ) -> std::io::Result<()> {
        let mut active = Self::read_for_edit(path)?;
        let Self {
            character_sort_column,
            character_sort_descending,
            stay_logged_in,
            remember_device,
            quick_login,
            startup_engine,
            confirm_logout,
            confirm_exit,
        } = *self;
        if before.character_sort_column != character_sort_column {
            active.character_sort_column = character_sort_column;
        }
        if before.character_sort_descending != character_sort_descending {
            active.character_sort_descending = character_sort_descending;
        }
        if before.stay_logged_in != stay_logged_in {
            active.stay_logged_in = stay_logged_in;
        }
        if before.remember_device != remember_device {
            active.remember_device = remember_device;
        }
        if before.startup_engine != startup_engine {
            active.startup_engine = startup_engine;
        }
        if before.quick_login != quick_login {
            active.quick_login = quick_login;
        }
        if before.confirm_logout != confirm_logout {
            active.confirm_logout = confirm_logout;
        }
        if before.confirm_exit != confirm_exit {
            active.confirm_exit = confirm_exit;
        }
        active.publish(path)
    }

    #[cfg(any(bevy_shell, test))]
    pub(crate) fn disable_remember_device_at(path: &std::path::Path) -> std::io::Result<()> {
        Self::record_opt_out(path)?;
        Self::try_replay_opt_out(path)
    }

    #[cfg(any(bevy_shell, test))]
    pub(super) fn record_opt_out(path: &Path) -> io::Result<()> {
        crate::atomic_file::publish_durable(&path.with_added_extension("opt-out"), b"false")
    }

    #[cfg(any(bevy_shell, test))]
    pub(super) fn try_replay_opt_out(path: &Path) -> io::Result<()> {
        match crate::atomic_file::try_lock(path) {
            Ok(_lock) => Self::replay_opt_out(path),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(error),
        }
    }

    // The caller holds the publication lock. Keep the intent until the merged
    // sidecar is durable, so an interrupted replay can safely run again.
    fn replay_opt_out(path: &Path) -> io::Result<()> {
        if path.with_added_extension("opt-out").try_exists()? {
            Self::read_for_edit(path)?.publish(path)?;
        }
        Ok(())
    }

    fn read_for_edit(path: &Path) -> io::Result<Self> {
        match Self::read_sidecar(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                crate::atomic_file::preserve_malformed(path)?;
                Ok(Self::default())
            }
            result => result,
        }
    }

    fn publish(&self, path: &std::path::Path) -> std::io::Result<()> {
        // Under the sidecar lock a pending opt-out wins every merge and survives a failed write.
        let marker = path.with_added_extension("opt-out");
        let pending = marker.try_exists()?;
        let mut merged = *self;
        if pending {
            merged.remember_device = false;
        }
        crate::atomic_file::publish_durable(path, &serde_json::to_vec(&merged)?)?;
        if pending {
            std::fs::remove_file(marker)?;
        }
        Ok(())
    }
}
impl super::SettingsState {
    pub fn request_native_sign_out(&mut self) {
        self.native_sign_out = true;
    }
    pub fn take_native_sign_out(&mut self) -> bool {
        std::mem::take(&mut self.native_sign_out)
    }
    /// Queues a Platform web page (support tickets, cookie policy) for the system browser.
    pub fn request_web_link(&mut self, link: crate::web_links::WebLink) {
        self.web_link = Some(link);
    }
    pub fn take_web_link(&mut self) -> Option<crate::web_links::WebLink> {
        self.web_link.take()
    }
}
