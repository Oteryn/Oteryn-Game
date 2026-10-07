//! Stable entry point of an installed client (CLIENT-INSTALLER-0 §2.1): reads `current.txt` next
//! to itself and starts `releases\<current.txt>\oteryn-client.exe` with its own arguments. It never
//! guesses another release. `--after-setup` first waits until the install that started it has
//! released the transaction mutex.
#![windows_subsystem = "windows"]

use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};

/// Why no release can be started.
#[derive(Debug)]
enum LaunchError {
    MissingPointer,
    UnreadablePointer(io::Error),
    MalformedPointer,
    DanglingPointer(String),
    #[cfg_attr(not(windows), allow(dead_code))]
    SetupStillRunning,
    #[cfg_attr(not(windows), allow(dead_code))]
    Os(io::Error),
}

impl Display for LaunchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPointer => formatter.write_str("current.txt is missing"),
            Self::UnreadablePointer(error) => {
                write!(formatter, "current.txt is unreadable: {error}")
            }
            Self::MalformedPointer => formatter.write_str("current.txt does not name a release"),
            Self::DanglingPointer(release) => {
                write!(
                    formatter,
                    "release {release} named by current.txt is not installed"
                )
            }
            Self::SetupStillRunning => formatter.write_str("the installer did not finish in time"),
            Self::Os(error) => write!(formatter, "cannot start Oteryn: {error}"),
        }
    }
}

/// `<release_id>` or `<release_id>~<n>`, `n` a decimal integer from 1 without leading zeros.
fn is_release_directory_name(value: &str) -> bool {
    let (release_id, suffix) = match value.split_once('~') {
        Some((release_id, suffix)) => (release_id, Some(suffix)),
        None => (value, None),
    };
    let suffix_valid = suffix.is_none_or(|suffix| {
        !suffix.is_empty()
            && !suffix.starts_with('0')
            && suffix.bytes().all(|digit| digit.is_ascii_digit())
            && suffix.parse::<u32>().is_ok()
    });
    suffix_valid && oteryn_client::is_valid_release_id(release_id)
}

/// The release directory name `current.txt` holds; one trailing line ending is accepted.
fn parse_pointer(contents: &[u8]) -> Result<&str, LaunchError> {
    let contents = contents
        .strip_suffix(b"\r\n")
        .or_else(|| contents.strip_suffix(b"\n"))
        .unwrap_or(contents);
    std::str::from_utf8(contents)
        .ok()
        .filter(|value| is_release_directory_name(value))
        .ok_or(LaunchError::MalformedPointer)
}

/// The executable of the active release under `install_dir`.
fn active_client(install_dir: &Path) -> Result<PathBuf, LaunchError> {
    let contents = match std::fs::read(install_dir.join("current.txt")) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(LaunchError::MissingPointer);
        }
        Err(error) => return Err(LaunchError::UnreadablePointer(error)),
    };
    let release = parse_pointer(&contents)?;
    let client = install_dir
        .join("releases")
        .join(release)
        .join("oteryn-client.exe");
    if client.is_file() {
        Ok(client)
    } else {
        Err(LaunchError::DanglingPointer(release.to_owned()))
    }
}

#[cfg(windows)]
fn wait_for_setup_exit() -> Result<(), LaunchError> {
    use oteryn_client::win_mutex;
    use std::time::{Duration, Instant};
    const SETUP_EXIT_TIMEOUT: Duration = Duration::from_secs(120);
    let sid = win_mutex::current_user_sid().map_err(LaunchError::Os)?;
    let name = win_mutex::setup_mutex_name(&sid);
    let deadline = Instant::now() + SETUP_EXIT_TIMEOUT;
    while win_mutex::exists(&name).map_err(LaunchError::Os)? {
        if Instant::now() >= deadline {
            return Err(LaunchError::SetupStillRunning);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

#[cfg(windows)]
fn launch() -> Result<(), LaunchError> {
    let mut arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "--after-setup")
    {
        arguments.remove(0);
        wait_for_setup_exit()?;
    }
    let launcher = std::env::current_exe().map_err(LaunchError::Os)?;
    let install_dir = launcher
        .parent()
        .ok_or(LaunchError::Os(io::Error::from(io::ErrorKind::NotFound)))?;
    let client = active_client(install_dir)?;
    std::process::Command::new(client)
        .args(arguments)
        .spawn()
        .map_err(LaunchError::Os)?;
    Ok(())
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    match launch() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let message = format!("{error}. Reinstall Oteryn to repair it.");
            eprintln!("Oteryn: {message}");
            // The launcher has no console, so the user sees the error only in a dialog. A
            // `--smoke` run is unattended and must not block on one.
            if !std::env::args_os().any(|argument| argument == "--smoke") {
                oteryn_client::win_mutex::show_error(&message);
            }
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(not(windows))]
fn main() {
    if let Ok(directory) = std::env::current_dir()
        && let Err(error) = active_client(&directory)
    {
        eprintln!("Oteryn: {error}");
    }
    println!("Oteryn launcher: Windows desktop target only");
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE: &str = oteryn_client::RELEASE_ID;

    fn install_dir(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("oteryn-launcher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        assert!(std::fs::create_dir_all(directory.join("releases")).is_ok());
        directory
    }

    fn install_release(directory: &Path, release: &str) {
        let release_dir = directory.join("releases").join(release);
        assert!(std::fs::create_dir_all(&release_dir).is_ok());
        assert!(std::fs::write(release_dir.join("oteryn-client.exe"), b"").is_ok());
    }

    #[test]
    fn missing_pointer_is_refused() {
        let directory = install_dir("missing");
        install_release(&directory, RELEASE);
        assert!(matches!(
            active_client(&directory),
            Err(LaunchError::MissingPointer)
        ));
    }

    #[test]
    fn malformed_pointers_are_refused() {
        let directory = install_dir("malformed");
        install_release(&directory, RELEASE);
        install_release(&directory, "..");
        for contents in [
            String::new(),
            "..".to_owned(),
            format!("{RELEASE}~0"),
            format!("{RELEASE}~01"),
            format!("{RELEASE}~"),
            format!("{RELEASE}~1~2"),
            format!("{RELEASE}~99999999999"),
            format!(" {RELEASE}"),
            format!("{RELEASE}\n\n"),
            format!("{RELEASE}\\..\\x"),
            "not-a-release".to_owned(),
        ] {
            assert!(std::fs::write(directory.join("current.txt"), &contents).is_ok());
            assert!(
                matches!(
                    active_client(&directory),
                    Err(LaunchError::MalformedPointer)
                ),
                "{contents:?}"
            );
        }
        assert!(std::fs::write(directory.join("current.txt"), [0xff, 0xfe]).is_ok());
        assert!(matches!(
            active_client(&directory),
            Err(LaunchError::MalformedPointer)
        ));
    }

    #[test]
    fn dangling_pointers_are_refused() {
        let directory = install_dir("dangling");
        install_release(&directory, RELEASE);
        let repaired = format!("{RELEASE}~1");
        assert!(std::fs::write(directory.join("current.txt"), &repaired).is_ok());
        assert!(matches!(
            active_client(&directory),
            Err(LaunchError::DanglingPointer(release)) if release == repaired
        ));
        // A release directory without its executable is dangling too.
        assert!(std::fs::create_dir_all(directory.join("releases").join(&repaired)).is_ok());
        assert!(matches!(
            active_client(&directory),
            Err(LaunchError::DanglingPointer(_))
        ));
    }

    #[test]
    fn valid_pointers_start_exactly_the_named_release() {
        let directory = install_dir("valid");
        install_release(&directory, RELEASE);
        let repaired = format!("{RELEASE}~2");
        install_release(&directory, &repaired);
        for (contents, release) in [
            (RELEASE.to_owned(), RELEASE),
            (format!("{RELEASE}\r\n"), RELEASE),
            (format!("{repaired}\n"), repaired.as_str()),
        ] {
            assert!(std::fs::write(directory.join("current.txt"), &contents).is_ok());
            let expected = directory
                .join("releases")
                .join(release)
                .join("oteryn-client.exe");
            assert!(
                matches!(active_client(&directory), Ok(client) if client == expected),
                "{contents:?}"
            );
        }
    }
}
