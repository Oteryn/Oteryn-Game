use crate::CLIENT_BUILD;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

pub type BrowserOpener = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// A required native login setting that is absent or empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeLoginSettingError(pub &'static str);

impl Display for NativeLoginSettingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "native login setting {} is missing", self.0)
    }
}

impl std::error::Error for NativeLoginSettingError {}

/// Native login settings (character mode 33a: the character comes from configuration).
#[derive(Clone)]
pub struct NativeLoginConfig {
    pub platform_url: String,
    pub gateway_url: String,
    pub oauth_client_id: String,
    /// The directory `world_ref` that must be listed before any credential is requested.
    pub world_ref: String,
    /// Canonical lowercase UUIDv7.
    pub character_id: String,
    /// The only trust root the game endpoint's certificate may chain to (PEM or DER).
    pub dev_root: PathBuf,
    pub client_build: String,
    pub callback_timeout: Duration,
    /// Backoff unit of every retry; one second in production.
    pub retry_unit: Duration,
    pub open_browser: BrowserOpener,
}

impl Debug for NativeLoginConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeLoginConfig")
            .field("platform_url", &self.platform_url)
            .field("gateway_url", &self.gateway_url)
            .field("world_ref", &self.world_ref)
            .field("dev_root", &self.dev_root)
            .finish_non_exhaustive()
    }
}

impl NativeLoginConfig {
    /// Reads `OTERYN_PLATFORM_URL`, `OTERYN_GATEWAY_URL`, `OTERYN_OAUTH_CLIENT_ID`,
    /// `OTERYN_WORLD`, `OTERYN_CHARACTER_ID` and `OTERYN_DEV_ROOT`. Native login is off
    /// (`Ok(None)`) while `OTERYN_PLATFORM_URL` is unset.
    pub fn from_env(
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, NativeLoginSettingError> {
        let setting = |name: &'static str| {
            lookup(name)
                .filter(|value| !value.is_empty())
                .ok_or(NativeLoginSettingError(name))
        };
        let Ok(platform_url) = setting("OTERYN_PLATFORM_URL") else {
            return Ok(None);
        };
        Ok(Some(Self {
            platform_url,
            gateway_url: setting("OTERYN_GATEWAY_URL")?,
            oauth_client_id: setting("OTERYN_OAUTH_CLIENT_ID")?,
            world_ref: setting("OTERYN_WORLD")?,
            character_id: setting("OTERYN_CHARACTER_ID")?,
            dev_root: PathBuf::from(setting("OTERYN_DEV_ROOT")?),
            client_build: CLIENT_BUILD.to_owned(),
            callback_timeout: DEFAULT_CALLBACK_TIMEOUT,
            retry_unit: Duration::from_secs(1),
            open_browser: Arc::new(open_system_browser),
        }))
    }
}

/// Opens `url` in the system browser without waiting for it.
pub fn open_system_browser(url: &str) -> bool {
    #[cfg(windows)]
    let mut command = {
        let mut command = std::process::Command::new("rundll32");
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|mut child| {
            std::thread::spawn(move || child.wait());
        })
        .is_ok()
}
