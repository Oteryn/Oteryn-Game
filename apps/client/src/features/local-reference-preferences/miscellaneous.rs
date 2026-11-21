//! Local confirmations and session choices; Platform retains all session authority.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MiscellaneousPreferences {
    pub stay_logged_in: bool,
    pub remember_device: bool,
    pub quick_login: bool,
    pub confirm_logout: bool,
    pub confirm_exit: bool,
}
impl Default for MiscellaneousPreferences {
    fn default() -> Self {
        Self {
            stay_logged_in: false,
            remember_device: false,
            quick_login: false,
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
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(4097)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 4096 {
            return Err(std::io::Error::other("Invalid session options size"));
        }
        serde_json::from_slice(&bytes).map_err(std::io::Error::other)
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
        use std::io::ErrorKind;
        let mut active = match Self::read(path) {
            Err(error) if error.kind() == ErrorKind::NotFound => Self::default(),
            result => result?,
        };
        let Self {
            stay_logged_in,
            remember_device,
            quick_login,
            confirm_logout,
            confirm_exit,
        } = *self;
        if before.stay_logged_in != stay_logged_in {
            active.stay_logged_in = stay_logged_in;
        }
        if before.remember_device != remember_device {
            active.remember_device = remember_device;
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

    fn publish(&self, path: &std::path::Path) -> std::io::Result<()> {
        crate::atomic_file::replace(path, &serde_json::to_vec(self)?, true)
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
