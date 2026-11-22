use crate::CLIENT_BUILD;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

const HELPER_OBSERVATION: Duration = Duration::from_secs(3);
const DEFAULT_CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

use oteryn_foundation::CancellationToken;

pub type BrowserOpener = Arc<dyn Fn(&str) -> bool + Send + Sync>;

#[derive(Default)]
struct BrowserWorkers {
    shutdown: CancellationToken,
    handles: Mutex<Vec<std::thread::JoinHandle<()>>>,
}

#[derive(Clone)]
pub(crate) struct SystemBrowser {
    opener: BrowserOpener,
    workers: Arc<BrowserWorkers>,
}

impl Default for SystemBrowser {
    fn default() -> Self {
        let workers = Arc::<BrowserWorkers>::default();
        let owner = workers.clone();
        Self {
            opener: Arc::new(move |url| deliver_system_url(url, &owner, None, None)),
            workers,
        }
    }
}

impl SystemBrowser {
    pub(crate) fn shutdown(&self) {
        self.workers.shutdown.cancel();
        let handles = std::mem::take(
            &mut *self
                .workers
                .handles
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        for handle in handles {
            while !handle.is_finished() {
                if std::time::Instant::now() >= deadline {
                    return;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            let _ = handle.join();
        }
    }
}

/// One admission attempt or Store session. A replacement gets a fresh epoch.
#[derive(Default)]
pub(crate) struct BrowserEpoch {
    ended: CancellationToken,
    delivery: Mutex<Delivery>,
}

#[derive(Default)]
struct Delivery {
    callbacks: usize,
    helpers: Vec<mpsc::Receiver<()>>,
}

impl BrowserEpoch {
    pub(crate) fn opener(self: &Arc<Self>, config: &NativeLoginConfig) -> BrowserOpener {
        let epoch = self.clone();
        if Arc::ptr_eq(&config.open_browser, &config.system_browser.opener) {
            let workers = config.system_browser.workers.clone();
            return Arc::new(move |url| {
                deliver_system_url(url, &workers, None, Some(epoch.clone()))
            });
        }
        let browser = config.open_browser.clone();
        Arc::new(move |url| {
            {
                let mut delivery = epoch
                    .delivery
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                if epoch.ended.is_cancelled() {
                    return false;
                }
                delivery.callbacks += 1;
            }
            let delivered = browser(url);
            let mut delivery = epoch
                .delivery
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            delivery.callbacks -= 1;
            delivered && !epoch.ended.is_cancelled()
        })
    }

    pub(crate) fn close(&self) {
        let mut delivery = self.delivery.lock().unwrap_or_else(PoisonError::into_inner);
        if self.ended.is_cancelled() {
            return;
        }
        self.ended.cancel();
        delivery.prune();
        if delivery.callbacks > 0 || !delivery.helpers.is_empty() {
            eprintln!("Browser delivery cancelled: URL may already have been delivered.");
        }
        let helpers = std::mem::take(&mut delivery.helpers);
        drop(delivery);
        let deadline = std::time::Instant::now() + HELPER_OBSERVATION + Duration::from_secs(2);
        for helper in helpers {
            let _ =
                helper.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()));
        }
    }
}

impl Delivery {
    fn prune(&mut self) {
        self.helpers
            .retain(|helper| matches!(helper.try_recv(), Err(mpsc::TryRecvError::Empty)));
    }
}

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
    pub(crate) session_options: Option<crate::play::MiscellaneousPreferences>,
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
    pub(crate) system_browser: SystemBrowser,
    pub select_character: crate::character_selection::CharacterSelector,
    pub training_queue: crate::character_selection::TrainingQueue,
    pub account_services: crate::account_services::Presenter,
    pub coin_shop: crate::coin_shop::Presenter,
    pub premium: crate::premium::Presenter,
    /// chsw (capability 512): after the server released the session for a Channel switch, the
    /// next entry takes this character into this Channel of the same World without the picker.
    pub channel_switch: Option<(String, String)>,
    pub(crate) require_character_picker: bool,
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
    /// `OTERYN_WORLD`, `OTERYN_CHARACTER_ID` and `OTERYN_DEV_ROOT`. The root is required
    /// for gameplay TLS after a grant. Native login is off
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
        let system_browser = SystemBrowser::default();
        Ok(Some(Self {
            session_options: None,
            device_sessions,
            platform_url,
            gateway_url,
            oauth_client_id,
            world_ref: lookup("OTERYN_WORLD").unwrap_or_default(),
            character_id: lookup("OTERYN_CHARACTER_ID").unwrap_or_default(),
            dev_root: PathBuf::from(lookup("OTERYN_DEV_ROOT").unwrap_or_default()),
            client_build: CLIENT_BUILD.to_owned(),
            preference_path: crate::play::PreferenceStore::path(),
            callback_timeout: DEFAULT_CALLBACK_TIMEOUT,
            retry_unit: Duration::from_secs(1),
            open_browser: system_browser.opener.clone(),
            system_browser,
            select_character: Arc::new(crate::character_selection::select),
            training_queue: Default::default(),
            account_services: Arc::new(crate::account_services::present),
            coin_shop: Arc::new(crate::coin_shop::present),
            premium: Arc::new(crate::premium::present),
            channel_switch: None,
            require_character_picker: false,
        }))
    }
}

fn deliver_system_url(
    url: &str,
    workers: &BrowserWorkers,
    cancel: Option<&CancellationToken>,
    epoch: Option<Arc<BrowserEpoch>>,
) -> bool {
    let platform = if cfg!(windows) {
        BrowserPlatform::Windows
    } else if cfg!(target_os = "macos") {
        BrowserPlatform::MacOs
    } else {
        BrowserPlatform::Unix
    };
    let mut command = browser_command(platform, url);
    let cancel = cancel.cloned();
    let (result, delivery) = std::sync::mpsc::channel();
    // Register before shutdown can take the handles. Delivery itself runs outside the lock.
    let mut handles = workers
        .handles
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if workers.shutdown.is_cancelled() {
        return false;
    }
    handles.retain(|handle| !handle.is_finished());
    let shutdown = workers.shutdown.clone();
    let worker = std::thread::Builder::new()
        .name("browser-helper".into())
        .spawn(move || {
            let mut gate = epoch.as_ref().map(|scope| {
                scope
                    .delivery
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
            });
            let ended = || {
                epoch
                    .as_ref()
                    .is_some_and(|scope| scope.ended.is_cancelled())
            };
            let cancelled = || {
                shutdown.is_cancelled()
                    || cancel.as_ref().is_some_and(CancellationToken::is_cancelled)
            };
            if cancelled() || ended() {
                return;
            }
            let Ok(mut child) = command
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            else {
                return;
            };
            // Spawn and registration are atomic with closing the epoch. Disconnect
            // only after the child exits or is killed and reaped.
            let _completion = gate.as_mut().map(|delivery| {
                delivery.prune();
                let (done, finished) = mpsc::channel();
                delivery.helpers.push(finished);
                done
            });
            drop(gate);
            let deadline = std::time::Instant::now() + HELPER_OBSERVATION;
            let mut pending = Some(result);
            while !cancelled() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if let Some(result) = pending {
                            let _ = result.send(status.success() && !ended());
                        }
                        return;
                    }
                    Err(_) => break,
                    Ok(None) => {
                        if std::time::Instant::now() >= deadline
                            && let Some(result) = pending.take()
                        {
                            if ended() {
                                break;
                            }
                            let _ = result.send(true);
                        }
                        if pending.is_none() && ended() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        });
    let Ok(worker) = worker else { return false };
    handles.push(worker);
    drop(handles);
    delivery.recv().unwrap_or(false)
}

enum BrowserPlatform {
    Windows,
    MacOs,
    Unix,
}

fn browser_command(platform: BrowserPlatform, url: &str) -> std::process::Command {
    let mut command = match platform {
        BrowserPlatform::Windows => std::process::Command::new("rundll32"),
        BrowserPlatform::MacOs => std::process::Command::new("open"),
        BrowserPlatform::Unix => std::process::Command::new("xdg-open"),
    };
    if matches!(platform, BrowserPlatform::Windows) {
        command.arg("url.dll,FileProtocolHandler");
    }
    command.arg(url);
    command
}

impl NativeLoginConfig {
    pub(crate) fn bind_system_browser(&mut self, system: SystemBrowser) {
        if Arc::ptr_eq(&self.open_browser, &self.system_browser.opener) {
            self.open_browser = system.opener.clone();
        }
        self.system_browser = system;
    }

    pub(crate) fn scoped_browser(&self, cancel: CancellationToken) -> BrowserOpener {
        if Arc::ptr_eq(&self.open_browser, &self.system_browser.opener) {
            let workers = self.system_browser.workers.clone();
            Arc::new(move |url| deliver_system_url(url, &workers, Some(&cancel), None))
        } else {
            Arc::clone(&self.open_browser)
        }
    }

    pub(crate) async fn forget_devices(&self) -> bool {
        use oteryn_platform_client::{PlatformClientConfig, gameplay_entry::login_client};
        if let (Ok(platform), Ok(gateway)) = (
            PlatformClientConfig::new(&self.platform_url),
            PlatformClientConfig::new(&self.gateway_url),
        ) && let Ok(client) =
            login_client(&platform, &gateway, &self.oauth_client_id, self.retry_unit)
        {
            self.device_sessions.finish_sign_out(&client).await.is_ok()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod browser_command_tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn browser_helper_delivery_and_reaping() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;
        const ROOT: &str = "OTERYN_BROWSER_DELIVERY_TEST";
        let Some(root) = std::env::var_os(ROOT) else {
            let dir = tempfile::tempdir()?;
            let helper = dir.path().join("xdg-open");
            std::fs::write(
                &helper,
                "#!/bin/sh\ncase $1 in fail) exit 1;; esac\necho $$ > \"$OTERYN_BROWSER_DELIVERY_TEST/pid\"\nexec sleep \"$1\"\n",
            )?;
            std::fs::set_permissions(helper, std::fs::Permissions::from_mode(0o755))?;
            let name = std::thread::current().name().ok_or("test name")?.to_owned();
            let status = std::process::Command::new(std::env::current_exe()?)
                .args(["--exact", &name, "--nocapture"])
                .env(ROOT, dir.path())
                .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
                .status()?;
            assert!(status.success());
            return Ok(());
        };
        let system = SystemBrowser::default();
        assert!(!(system.opener)("fail"));
        let cancel = CancellationToken::new();
        for delay in ["4", "60"] {
            assert!(if delay == "4" {
                (system.opener)(delay)
            } else {
                deliver_system_url(delay, &system.workers, Some(&cancel), None)
            });
            let pid = std::fs::read_to_string(PathBuf::from(&root).join("pid"))?;
            let process = PathBuf::from("/proc").join(pid.trim());
            assert!(process.exists());
            if delay == "60" {
                cancel.cancel();
            }
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while process.exists() && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(!process.exists());
        }
        let config = NativeLoginConfig::from_env(|name| match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some("https://example.invalid".into()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("browser-test".into()),
            _ => None,
        })?
        .ok_or("missing native config")?;
        let client = crate::ClientBootstrap::new()?.with_native_login(config);
        let browser = client
            .native_login
            .as_ref()
            .ok_or("missing native config")?
            .scoped_browser(client.runtime.cancellation());
        let entry = crate::reentry::Reentry::new(client);
        assert!(browser("60"));
        let pid = std::fs::read_to_string(PathBuf::from(root).join("pid"))?;
        let process = PathBuf::from("/proc").join(pid.trim());
        assert!(process.exists());
        entry.shutdown();
        assert!(!process.exists(), "browser helper survived shutdown");
        Ok(())
    }

    #[test]
    fn browser_commands_keep_the_url_in_one_argument_without_a_shell() {
        let url = "https://example.invalid/authorize?state=a%20b&other=$(false)";
        for (platform, program, prefix) in [
            (
                BrowserPlatform::Windows,
                "rundll32",
                Some("url.dll,FileProtocolHandler"),
            ),
            (BrowserPlatform::MacOs, "open", None),
            (BrowserPlatform::Unix, "xdg-open", None),
        ] {
            let command = browser_command(platform, url);
            assert_eq!(command.get_program(), program);
            let arguments: Vec<_> = command.get_args().collect();
            if let Some(prefix) = prefix {
                assert_eq!(arguments, [prefix, url]);
            } else {
                assert_eq!(arguments, [url]);
            }
        }
    }
}
