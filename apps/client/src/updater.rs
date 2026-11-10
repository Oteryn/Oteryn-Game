//! T005 maintenance caller. Only a build-pinned TUF target may install bytes.
mod install;
#[cfg(windows)]
mod window;
pub use install::verify_archive;
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
/// Shared CLI/native-window caller; operations serialize across processes in this installation.
pub async fn maintain(
    directory: &Path,
    channel: &str,
    action: Action,
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
