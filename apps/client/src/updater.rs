//! T005 maintenance caller. Only a build-pinned TUF target may install bytes.
mod install;
#[cfg(windows)]
mod window;
pub use install::verify_archive;
use oteryn_platform_client::native_login::UpdatePointer;
use oteryn_platform_client::updates::{UpdateClient, UpdateConfig, UpdateError};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Check,
    Verify,
    Update,
    Repair,
}
pub fn pinned_root(channel: &str) -> &'static [u8] {
    match channel {
        "stable" => include_bytes!(concat!(env!("OUT_DIR"), "/update-root-stable.json")),
        "beta" => include_bytes!(concat!(env!("OUT_DIR"), "/update-root-beta.json")),
        _ => &[],
    }
}
/// A release pointer may name the accepted opaque Platform release ID or an installer slot.
pub fn release_directory(value: &str) -> bool {
    let (id, slot) = value
        .split_once('~')
        .map_or((value, None), |(a, b)| (a, Some(b)));
    !id.is_empty()
        && id.len() <= 160
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.+".contains(&b))
        && slot.is_none_or(|s| !s.is_empty() && !s.starts_with('0') && s.parse::<u32>().is_ok())
}
fn compiled_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}
fn compiled_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x86_64"
    }
}
/// Use the running build's pinned root even when its launcher predates this gate.
pub(crate) fn start_login_update(update: &UpdatePointer) -> Result<(), UpdateError> {
    let channel = update.update_channel.as_str();
    require_update_root(pinned_root(channel))?;
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|_| UpdateError::Configuration)?;
        login_install_directory(&exe).ok_or(UpdateError::Configuration)?;
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

fn require_update_root(root: &[u8]) -> Result<(), UpdateError> {
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
        && directory.join("oteryn-launcher.exe").is_file())
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
        let minimum = if minimum.is_empty() {
            None
        } else {
            Some(minimum.parse().ok()?)
        };
        let result = (|| {
            let exe = std::env::current_exe().map_err(|_| UpdateError::Configuration)?;
            let directory = login_install_directory(&exe).ok_or(UpdateError::Configuration)?;
            window::wait_for_client_exit()?;
            let runtime = tokio::runtime::Runtime::new().map_err(|_| UpdateError::Unavailable)?;
            runtime.block_on(maintain_with_minimum(
                directory,
                channel,
                Action::Update,
                minimum,
            ))
        })();
        if let Ok(message) = result {
            crate::win_mutex::show_error(&message);
            Some(std::process::ExitCode::SUCCESS)
        } else {
            crate::win_mutex::show_error(&format!(
                "Automatic update could not finish. Download Oteryn at {url}"
            ));
            Some(std::process::ExitCode::FAILURE)
        }
    };
    Some(run().unwrap_or(std::process::ExitCode::FAILURE))
}

/// Shared CLI/native-window caller; operations serialize across processes in this installation.
pub async fn maintain(
    directory: &Path,
    channel: &str,
    action: Action,
) -> Result<String, UpdateError> {
    maintain_with_minimum(directory, channel, action, None).await
}

async fn maintain_with_minimum(
    directory: &Path,
    channel: &str,
    action: Action,
    minimum_release_sequence: Option<u64>,
) -> Result<String, UpdateError> {
    let config = UpdateConfig::new(
        channel,
        pinned_root(channel),
        compiled_platform(),
        compiled_arch(),
    )?;
    std::fs::create_dir_all(directory.join(".updates")).map_err(|_| UpdateError::Storage)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(directory.join(".updates/lock"))
        .map_err(|_| UpdateError::Storage)?;
    lock.try_lock().map_err(|_| UpdateError::Unavailable)?;
    #[cfg(windows)]
    let _maintenance = window::acquire_maintenance()?;
    let update = UpdateClient::check(config, &directory.join(".updates")).await?;
    update.require_minimum(minimum_release_sequence)?;
    let current = std::fs::read_to_string(directory.join("current.txt")).unwrap_or_default();
    if action == Action::Check {
        return Ok(format!(
            "Version: {}\nRelease: {}\nUpdate: {}\nInstalled: {}",
            update.policy.current_version_display,
            update.policy.current_release_id,
            update.policy.update_mode,
            current.trim()
        ));
    }
    if fs2::available_space(directory).map_err(|_| UpdateError::Storage)? < update.length {
        return Err(UpdateError::Storage);
    }
    let package = directory.join(".updates/package.zip");
    let _ = std::fs::remove_file(package.with_extension("partial"));
    update.download(&package).await?;
    if action == Action::Verify {
        let current = current.trim();
        if !release_directory(current)
            || current.split('~').next() != Some(&update.policy.current_release_id)
        {
            return Err(UpdateError::Unavailable);
        }
        verify_archive(&package, &directory.join("releases").join(current))?;
        return Ok("Verified installed release against the current signed package.".into());
    }
    let installed = install::install(&package, directory, &update.policy.current_release_id)?;
    Ok(format!(
        "Installed verified release {installed}. Start the launcher to play."
    ))
}
/// Returns true when a maintenance command was consumed, without starting gameplay.
pub fn requested(
    directory: &Path,
    args: &[std::ffi::OsString],
) -> Option<Result<String, UpdateError>> {
    let action = if args.iter().any(|a| a == "--check-updates") {
        Action::Check
    } else if args.iter().any(|a| a == "--verify-release") {
        Action::Verify
    } else if args.iter().any(|a| a == "--repair") {
        Action::Repair
    } else if args.iter().any(|a| a == "--update") {
        Action::Update
    } else {
        #[cfg(windows)]
        if args.iter().any(|a| a == "--updates") {
            return Some(window::show(directory));
        }
        return None;
    };
    let channel = if args.iter().any(|a| a == "--beta") {
        "beta"
    } else {
        "stable"
    };
    Some(
        tokio::runtime::Runtime::new()
            .map_err(|_| UpdateError::Unavailable)
            .and_then(|runtime| runtime.block_on(maintain(directory, channel, action))),
    )
}

#[cfg(test)]
mod install_tests;
