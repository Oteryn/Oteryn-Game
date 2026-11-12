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
        lwrite!(formatter, "native login setting {} is missing", self.0)
    }
}

impl std::error::Error for NativeLoginSettingError {}

/// Native login settings with an optional character preselection.
#[derive(Clone)]
pub struct NativeLoginConfig {
    pub device_sessions: Arc<crate::device_login::DeviceLogin>,
    pub platform_url: String,
    pub gateway_url: String,
    pub oauth_client_id: String,
    /// Legacy optional world hint; owner character membership selects the world.
    pub world_ref: String,
    /// Optional canonical lowercase UUIDv7 preselection.
    pub character_id: String,
    /// The only trust root the game endpoint's certificate may chain to (PEM or DER).
    pub dev_root: PathBuf,
    pub client_build: String,
    /// Local preference owner selected by the host, including an explicit isolated path.
    pub preference_path: Option<PathBuf>,
    pub callback_timeout: Duration,
    /// Backoff unit of every retry; one second in production.
    pub retry_unit: Duration,
    pub open_browser: BrowserOpener,
    pub select_character: crate::character_selection::CharacterSelector,
    pub training_queue: crate::character_selection::TrainingQueue,
    pub account_services: crate::account_services::Presenter,
    pub coin_shop: crate::coin_shop::Presenter,
    pub premium: crate::premium::Presenter,
}

impl Debug for NativeLoginConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct(tr!("NativeLoginConfig"))
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
        let gateway_url = setting("OTERYN_GATEWAY_URL")?;
        let oauth_client_id = setting("OTERYN_OAUTH_CLIENT_ID")?;
        let device_sessions = Arc::new(crate::device_login::DeviceLogin::configured(
            &platform_url,
            &oauth_client_id,
            lookup("OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED").as_deref() == Some("true"),
        ));
        Ok(Some(Self {
            device_sessions,
            platform_url,
            gateway_url,
            oauth_client_id,
            world_ref: lookup("OTERYN_WORLD").unwrap_or_default(),
            character_id: lookup("OTERYN_CHARACTER_ID").unwrap_or_default(),
            dev_root: PathBuf::from(setting("OTERYN_DEV_ROOT")?),
            client_build: CLIENT_BUILD.to_owned(),
            preference_path: crate::play::PreferenceStore::path(),
            callback_timeout: DEFAULT_CALLBACK_TIMEOUT,
            retry_unit: Duration::from_secs(1),
            open_browser: Arc::new(open_system_browser),
            select_character: Arc::new(crate::character_selection::select),
            training_queue: Default::default(),
            account_services: Arc::new(crate::account_services::present),
            coin_shop: Arc::new(crate::coin_shop::present),
            premium: Arc::new(crate::premium::present),
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

impl NativeLoginConfig {
    pub(crate) async fn forget_devices(&self) {
        use oteryn_platform_client::{PlatformClientConfig, gameplay_entry::login_client};
        if let (Ok(platform), Ok(gateway)) = (
            PlatformClientConfig::new(&self.platform_url),
            PlatformClientConfig::new(&self.gateway_url),
        ) && let Ok(client) =
            login_client(&platform, &gateway, &self.oauth_client_id, self.retry_unit)
        {
            self.device_sessions.sign_out(&client).await;
        }
    }
}
