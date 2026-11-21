//! T005 maintenance caller. Only a build-pinned TUF target may install bytes.
mod install;
mod launcher;
mod login;
pub use install::verify_archive;
#[cfg(windows)]
pub use login::login_update_requested;
pub(crate) use login::start_login_update;
use oteryn_platform_client::native_login::ReleaseIdentity;
use oteryn_platform_client::updates::{UpdateClient, UpdateConfig, UpdateError};
use std::{io::{self, Read}, path::Path};

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
const MAX_SLOT_LENGTH: usize = 64;

pub(crate) fn read_release_pointer(directory: &Path) -> io::Result<String> {
    const MAX_POINTER_BYTES: u64 = 1024;
    let mut current = String::new();
    std::fs::File::open(directory.join("current.txt"))?
        .take(MAX_POINTER_BYTES + 1)
        .read_to_string(&mut current)?;
    if current.len() as u64 > MAX_POINTER_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "release pointer is too large"));
    }
    Ok(current)
}

/// A release slot may name an opaque Platform release or a manual installer release.
pub fn release_directory(value: &str) -> bool {
    release_id_from_slot(value).is_some()
}

fn release_id_from_slot(value: &str) -> Option<&str> {
    let (id, slot) = value
        .split_once('~')
        .map_or((value, None), |(id, slot)| (id, Some(slot)));
    (value.len() <= MAX_SLOT_LENGTH
        && !id.is_empty()
        && !id.starts_with('.')
        && !id.ends_with('.')
        && !id.contains("..")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.+".contains(&byte))
        && slot.is_none_or(|slot| {
            !slot.is_empty()
                && !slot.starts_with('0')
                && slot.bytes().all(|byte| byte.is_ascii_digit())
                && slot.parse::<u32>().is_ok()
        }))
    .then_some(id)
}
pub(crate) fn compiled_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}
pub(crate) fn compiled_arch() -> &'static str {
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
    maintain_release(directory, channel, action, None).await
}

async fn maintain_release(
    directory: &Path,
    channel: &str,
    action: Action,
    gate: Option<(ReleaseIdentity, Option<u64>)>,
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
    let _maintenance = acquire_maintenance()?;
    let update = UpdateClient::check(config, &directory.join(".updates")).await?;
    if let Some((refused, minimum)) = gate {
        update.require_login_update(minimum, refused)?;
    }
    let current = read_release_pointer(directory).unwrap_or_default();
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
        if release_id_from_slot(current) != Some(&update.policy.current_release_id) {
            return Err(UpdateError::Unavailable);
        }
        verify_archive(&package, &directory.join("releases").join(current))?;
        return Ok("Verified installed release against the current signed package.".into());
    }
    let launcher_path = directory.join(launcher::LAUNCHER_EXECUTABLE);
    let launcher = cfg!(windows).then_some(launcher_path.as_path());
    let installed = install::install(
        &package,
        directory,
        &update.policy.current_release_id,
        launcher,
    )?;
    if gate.is_some() {
        return Ok(lfmt!(
            "Installed verified release {installed}. Oteryn will restart when you close this message.",
            installed = installed
        ));
    }
    Ok(lfmt!(
        "Installed verified release {installed}. Start the launcher to play.",
        installed = installed
    ))
}
/// Refuses maintenance while setup or the client runs (the install/client mutex handshake).
#[cfg(windows)]
fn acquire_maintenance() -> Result<crate::win_mutex::NamedMutex, UpdateError> {
    let sid = crate::win_mutex::current_user_sid().map_err(|_| UpdateError::Unavailable)?;
    let name = crate::win_mutex::setup_mutex_name(&sid);
    let guard = crate::win_mutex::create_exclusive(&name).map_err(|_| UpdateError::Unavailable)?;
    if crate::win_mutex::exists(&crate::win_mutex::client_mutex_name(&sid))
        .map_err(|_| UpdateError::Unavailable)?
    {
        return Err(UpdateError::Unavailable);
    }
    Ok(guard)
}
/// Returns true when a maintenance command was consumed, without starting gameplay.
pub fn requested(
    directory: &Path,
    args: &[std::ffi::OsString],
) -> Option<Result<String, UpdateError>> {
    let channel = if args.iter().any(|a| a == "--beta") {
        "beta"
    } else {
        "stable"
    };
    let action = if args.iter().any(|a| a == "--check-updates") {
        Action::Check
    } else if args.iter().any(|a| a == "--verify-release") {
        Action::Verify
    } else if args.iter().any(|a| a == "--repair") {
        Action::Repair
    } else if args.iter().any(|a| a == "--update") {
        Action::Update
    } else {
        #[cfg(bevy_shell)]
        if args.iter().any(|a| a == "--updates") {
            return Some(crate::bevy_shell::maintenance::run(
                directory, args, channel,
            ));
        }
        return None;
    };
    Some(
        tokio::runtime::Runtime::new()
            .map_err(|_| UpdateError::Unavailable)
            .and_then(|runtime| runtime.block_on(maintain(directory, channel, action))),
    )
}

#[cfg(test)]
mod install_tests;

#[cfg(test)]
#[test]
fn oversized_release_pointer_is_rejected() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("current.txt"), vec![b' '; 1025])?;
    assert!(matches!(read_release_pointer(root.path()), Err(error) if error.kind() == io::ErrorKind::InvalidData));
    Ok(())
}
