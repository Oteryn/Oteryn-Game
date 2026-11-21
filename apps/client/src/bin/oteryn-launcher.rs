//! Stable entry point of an installed client (CLIENT-INSTALLER-0 §2.1): reads `current.txt` next
//! to itself and starts `releases\<current.txt>\oteryn-client.exe` with its own arguments. It never
//! guesses another release. `--after-setup` first waits until the install that started it has
//! released the transaction mutex.
#![windows_subsystem = "windows"]
#[macro_use]
extern crate oteryn_client_ui;

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
            Self::MissingPointer => formatter.write_str(tr!("current.txt is missing")),
            Self::UnreadablePointer(error) => {
                lwrite!(
                    formatter,
                    "current.txt is unreadable: {error}",
                    error = error
                )
            }
            Self::MalformedPointer => {
                formatter.write_str(tr!("current.txt does not name a release"))
            }
            Self::DanglingPointer(release) => {
                lwrite!(
                    formatter,
                    "release {release} named by current.txt is not installed",
                    release = release
                )
            }
            Self::SetupStillRunning => {
                formatter.write_str(tr!("the installer did not finish in time"))
            }
            Self::Os(error) => lwrite!(formatter, "cannot start Oteryn: {error}", error = error),
        }
    }
}

/// The release directory name `current.txt` holds; one trailing line ending is accepted.
fn parse_pointer(contents: &[u8]) -> Result<&str, LaunchError> {
    let contents = contents
        .strip_suffix(b"\r\n")
        .or_else(|| contents.strip_suffix(b"\n"))
        .unwrap_or(contents);
    std::str::from_utf8(contents)
        .ok()
        .filter(|value| oteryn_client::updater::release_directory(value))
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

#[path = "../launcher/package_status.rs"]
mod package_checks;

/// Local diagnosis never downloads or replaces a release behind the installer.
fn installation_report(install_dir: &Path) -> String {
    let status = match active_client(install_dir) {
        Ok(client) => format!(
            "{}\r\n{}: {}",
            lfmt!("Ready to launch: {}", client.display()),
            tr!("Active client"),
            package_checks::status(&client)
        ),
        Err(error) => lfmt!("Installation check failed: {error}", error = error),
    };
    lfmt!(
        "Oteryn installation check\r\nLauncher: {}\r\n{status}\r\nRepair: run the Oteryn installer and choose the same installation directory. Keep the client closed until setup finishes. Updates must come from your operator's trusted release channel.",
        oteryn_client::RELEASE_ID,
        status = status
    )
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
    if arguments
        .iter()
        .any(|argument| argument == "--after-client")
    {
        // The client requests maintenance before exiting; never race its install mutex.
        use std::time::{Duration, Instant};
        let sid = oteryn_client::win_mutex::current_user_sid().map_err(LaunchError::Os)?;
        let name = oteryn_client::win_mutex::client_mutex_name(&sid);
        let deadline = Instant::now() + Duration::from_secs(120);
        while oteryn_client::win_mutex::exists(&name).map_err(LaunchError::Os)? {
            if Instant::now() >= deadline {
                return Err(LaunchError::Os(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "client did not exit in time; close the client and retry maintenance",
                )));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        arguments.retain(|argument| argument != "--after-client");
    }
    let launcher = std::env::current_exe().map_err(LaunchError::Os)?;
    let install_dir = launcher
        .parent()
        .ok_or(LaunchError::Os(io::Error::from(io::ErrorKind::NotFound)))?;
    if let Some(result) = oteryn_client::updater::requested(install_dir, &arguments) {
        let message = result.map_err(|_| {
            LaunchError::Os(io::Error::other(
                "Update refused; trusted update configuration or signed release unavailable",
            ))
        })?;
        oteryn_client::win_mutex::show_error(&message);
        return Ok(());
    }
    if arguments
        .iter()
        .any(|argument| argument == "--check-install")
    {
        oteryn_client::win_mutex::show_error(&installation_report(install_dir));
        return Ok(());
    }
    let client = active_client(install_dir)?;
    std::process::Command::new(client)
        .args(arguments)
        .spawn()
        .map_err(LaunchError::Os)?;
    Ok(())
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    if let Some(path) = oteryn_client::play::PreferenceStore::path()
        && let Ok(store) = oteryn_client::play::PreferenceStore::load(path)
    {
        oteryn_client_ui::localization::set_default(store.preferences().language());
    }
    match launch() {
        Ok(()) => {
            oteryn_client::crash_report::record_launcher_exit_code(0);
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            let message = lfmt!("{error}. Reinstall Oteryn to repair it.", error = error);
            oteryn_client::crash_report::record_startup_failure("launcher", &message);
            oteryn_client::crash_report::record_launcher_exit_code(2);
            leprintln!("Oteryn: {message}", message = message);
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
        && let Some(result) = oteryn_client::updater::requested(
            &directory,
            &std::env::args_os().skip(1).collect::<Vec<_>>(),
        )
    {
        match result {
            Ok(message) => println!("{message}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        return;
    }
    if let Ok(directory) = std::env::current_dir()
        && let Err(error) = active_client(&directory)
    {
        leprintln!("Oteryn: {error}", error = error);
    }
    if std::env::args_os().any(|argument| argument == "--check-install")
        && let Ok(directory) = std::env::current_dir()
    {
        println!("{}", installation_report(&directory));
    }
    lprintln!("Oteryn launcher: Windows desktop target only");
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE: &str = oteryn_client::RELEASE_ID;

    fn install_dir() -> std::io::Result<tempfile::TempDir> {
        let fixture = tempfile::tempdir()?;
        std::fs::create_dir(fixture.path().join("releases"))?;
        Ok(fixture)
    }

    fn install_release(directory: &Path, release: &str) {
        let release_dir = directory.join("releases").join(release);
        assert!(std::fs::create_dir_all(&release_dir).is_ok());
        assert!(std::fs::write(release_dir.join("oteryn-client.exe"), b"").is_ok());
    }

    #[test]
    fn installation_check_explains_repair_and_active_release() -> std::io::Result<()> {
        let fixture = install_dir()?;
        let directory = fixture.path();
        assert!(installation_report(directory).contains("Repair: run the Oteryn installer"));
        install_release(directory, RELEASE);
        assert!(std::fs::write(directory.join("current.txt"), RELEASE).is_ok());
        let report = installation_report(directory);
        assert!(report.contains(RELEASE));
        assert!(report.contains("Active client"));
        Ok(())
    }

    #[test]
    fn missing_pointer_is_refused() -> std::io::Result<()> {
        let fixture = install_dir()?;
        let directory = fixture.path();
        install_release(directory, RELEASE);
        assert!(matches!(
            active_client(directory),
            Err(LaunchError::MissingPointer)
        ));
        Ok(())
    }

    #[test]
    fn malformed_pointers_are_refused() -> std::io::Result<()> {
        let fixture = install_dir()?;
        let directory = fixture.path();
        install_release(directory, RELEASE);
        install_release(directory, "..");
        for contents in [
            String::new(),
            "..".to_owned(),
            format!("{RELEASE}~0"),
            format!("{RELEASE}~01"),
            format!("{RELEASE}~+1"),
            "a".repeat(65),
            "rel..a".to_owned(),
            format!("{RELEASE}~"),
            format!("{RELEASE}~1~2"),
            format!("{RELEASE}~99999999999"),
            format!(" {RELEASE}"),
            format!("{RELEASE}\n\n"),
            format!("{RELEASE}\\..\\x"),
        ] {
            assert!(std::fs::write(directory.join("current.txt"), &contents).is_ok());
            assert!(
                matches!(active_client(directory), Err(LaunchError::MalformedPointer)),
                "{contents:?}"
            );
        }
        assert!(std::fs::write(directory.join("current.txt"), [0xff, 0xfe]).is_ok());
        assert!(matches!(
            active_client(directory),
            Err(LaunchError::MalformedPointer)
        ));
        Ok(())
    }

    #[test]
    fn dangling_pointers_are_refused() -> std::io::Result<()> {
        let fixture = install_dir()?;
        let directory = fixture.path();
        install_release(directory, RELEASE);
        let repaired = format!("{RELEASE}~1");
        assert!(std::fs::write(directory.join("current.txt"), &repaired).is_ok());
        assert!(matches!(
            active_client(directory),
            Err(LaunchError::DanglingPointer(release)) if release == repaired
        ));
        // A release directory without its executable is dangling too.
        assert!(std::fs::create_dir_all(directory.join("releases").join(&repaired)).is_ok());
        assert!(matches!(
            active_client(directory),
            Err(LaunchError::DanglingPointer(_))
        ));
        Ok(())
    }

    #[test]
    fn pointers_to_another_client_version_start_that_release() -> std::io::Result<()> {
        // Release A is active; installing B of another client version failed to activate, so
        // B's launcher runs with current.txt still naming A.
        let fixture = install_dir()?;
        let directory = fixture.path();
        let previous = "9.9.9+dev.ci.g1a2b3c4d5e6f";
        assert!(!previous.starts_with(env!("CARGO_PKG_VERSION")));
        install_release(directory, previous);
        install_release(directory, RELEASE);
        for (contents, release) in [
            (format!("{previous}\r\n"), previous.to_owned()),
            (format!("{previous}~1"), format!("{previous}~1")),
        ] {
            install_release(directory, &release);
            assert!(std::fs::write(directory.join("current.txt"), &contents).is_ok());
            let expected = directory
                .join("releases")
                .join(&release)
                .join("oteryn-client.exe");
            assert!(
                matches!(active_client(directory), Ok(client) if client == expected),
                "{contents:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn valid_pointers_start_exactly_the_named_release() -> std::io::Result<()> {
        let fixture = install_dir()?;
        let directory = fixture.path();
        install_release(directory, RELEASE);
        let repaired = format!("{RELEASE}~2");
        install_release(directory, &repaired);
        install_release(directory, "rel-a");
        for (contents, release) in [
            ("rel-a".to_owned(), "rel-a"),
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
                matches!(active_client(directory), Ok(client) if client == expected),
                "{contents:?}"
            );
        }
        Ok(())
    }
}
