//! Handoff from the refused client to its pinned updater and the launcher.
use super::pinned_root;
#[cfg(windows)]
use super::{Action, launcher::LAUNCHER_EXECUTABLE, maintain_release, release_directory};
use oteryn_platform_client::native_login::UpdatePointer;
use oteryn_platform_client::updates::UpdateError;
#[cfg(windows)]
use std::{
    path::Path,
    time::{Duration, Instant},
};

/// Use the running build's pinned root for the gate update.
pub(crate) fn start_login_update(update: &UpdatePointer) -> Result<(), UpdateError> {
    let channel = update.update_channel.as_str();
    require_update_root(pinned_root(channel))?;
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|_| UpdateError::Configuration)?;
        login_install_directory(&exe).ok_or(UpdateError::Configuration)?;
        crate::build_info::updater_release(channel).ok_or(UpdateError::Configuration)?;
        std::process::Command::new(exe)
            .arg("--login-update")
            .arg(channel)
            .arg(
                update
                    .minimum_release_sequence
                    .map_or_else(String::new, |minimum| minimum.to_string()),
            )
            .arg(&update.manual_download_url)
            .spawn()
            .map_err(|_| UpdateError::Unavailable)?;
        Ok(())
    }
    #[cfg(not(windows))]
    Err(UpdateError::Unavailable)
}

pub(super) fn require_update_root(root: &[u8]) -> Result<(), UpdateError> {
    if root.is_empty() {
        return Err(UpdateError::Configuration);
    }
    Ok(())
}

#[cfg(windows)]
fn login_install_directory(exe: &Path) -> Option<&Path> {
    let slot = exe.parent()?;
    let releases = slot.parent()?;
    let directory = releases.parent()?;
    (release_directory(slot.file_name()?.to_str()?)
        && releases.file_name()? == "releases"
        && directory.join("current.txt").is_file()
        && directory.join(LAUNCHER_EXECUTABLE).is_file())
    .then_some(directory)
}

/// Handle a gate-triggered updater subprocess before acquiring the client-instance mutex.
#[cfg(windows)]
pub fn login_update_requested() -> Option<std::process::ExitCode> {
    let mut args = std::env::args_os().skip(1);
    if args.next()?.as_os_str() != "--login-update" {
        return None;
    }
    let mut run = || {
        let [channel, minimum, url] = [args.next()?, args.next()?, args.next()?];
        if args.next().is_some() {
            return None;
        }
        let (channel, minimum, url) = (channel.to_str()?, minimum.to_str()?, url.to_str()?);
        if !url::Url::parse(url).is_ok_and(|url| {
            url.scheme() == "https"
                && url.host().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        }) {
            return None;
        }
        let minimum = if minimum.is_empty() {
            None
        } else {
            Some(minimum.parse().ok()?)
        };
        if let Some(path) = crate::play::PreferenceStore::path()
            && let Ok(store) = crate::play::PreferenceStore::load(path)
        {
            oteryn_client_ui::localization::set_default(store.preferences().language());
        }
        let exe = std::env::current_exe().ok()?;
        let directory = login_install_directory(&exe)?;
        let result = (|| {
            let release =
                crate::build_info::updater_release(channel).ok_or(UpdateError::Configuration)?;
            let sid = crate::win_mutex::current_user_sid().map_err(|_| UpdateError::Unavailable)?;
            let name = crate::win_mutex::client_mutex_name(&sid);
            wait_for_client_mutex(&name, Duration::from_secs(300))?;
            let runtime = tokio::runtime::Runtime::new().map_err(|_| UpdateError::Unavailable)?;
            let message = runtime.block_on(maintain_release(
                directory,
                release.update_channel().as_str(),
                Action::Update,
                Some((release, minimum)),
            ))?;
            // The old client exited and maintain_release released the setup mutex.
            crate::bevy_shell::maintenance::login_report(directory, message);
            std::process::Command::new(directory.join(LAUNCHER_EXECUTABLE))
                .spawn()
                .map_err(|_| UpdateError::Unavailable)?;
            Ok::<(), UpdateError>(())
        })();
        if result.is_ok() {
            Some(std::process::ExitCode::SUCCESS)
        } else {
            crate::bevy_shell::maintenance::login_report(
                directory,
                lfmt!(
                    "Automatic update could not finish. Download Oteryn at {url}",
                    url = url
                ),
            );
            Some(std::process::ExitCode::FAILURE)
        }
    };
    Some(run().unwrap_or(std::process::ExitCode::FAILURE))
}

#[cfg(windows)]
fn wait_for_client_mutex(name: &str, timeout: Duration) -> Result<(), UpdateError> {
    let deadline = Instant::now() + timeout;
    while crate::win_mutex::exists(name).map_err(|_| UpdateError::Unavailable)? {
        if Instant::now() >= deadline {
            return Err(UpdateError::Unavailable);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn login_updater_waits_for_the_client_lifetime_mutex() -> Result<(), Box<dyn std::error::Error>>
    {
        let name = format!("Local\\OterynLoginUpdateTest-{}", std::process::id());
        let client = crate::win_mutex::create_exclusive(&name)?;
        assert_eq!(
            wait_for_client_mutex(&name, Duration::ZERO),
            Err(UpdateError::Unavailable)
        );
        drop(client);
        assert_eq!(wait_for_client_mutex(&name, Duration::ZERO), Ok(()));
        Ok(())
    }
}
