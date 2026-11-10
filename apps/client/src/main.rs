#[macro_use]
extern crate oteryn_client_ui;
#[cfg(windows)]
mod windows_shell;

/// Build the process runtime once; the window owns every native entry attempt.
fn native_client() -> Option<oteryn_client::ClientBootstrap> {
    match oteryn_client::ClientBootstrap::new() {
        Ok(client) => Some(client),
        Err(error) => {
            lprintln!("Oteryn: {error}", error = error);
            None
        }
    }
}

/// Installation checks finish before a login runtime or presentation package can be loaded.
fn installation_check() -> Option<std::process::ExitCode> {
    let requested = std::env::args_os().any(|argument| argument == "--check-packages");
    #[cfg(windows)]
    let smoke = std::env::args_os().any(|argument| argument == "--smoke");
    match oteryn_client::package_integrity::verify_current(requested) {
        Ok(report) if requested => {
            println!("{report}");
            Some(std::process::ExitCode::SUCCESS)
        }
        Ok(_) => None,
        Err(error) => {
            eprintln!("Oteryn installation check failed: {error}");
            #[cfg(windows)]
            if !requested && !smoke {
                oteryn_client::package_integrity::show_repair_required(&error);
            }
            Some(std::process::ExitCode::FAILURE)
        }
    }
}

/// Holds `Global\OterynClient-<SID>` for the process lifetime and refuses to start while an
/// install or uninstall holds the transaction mutex (CLIENT-INSTALLER-0 §2.1).
#[cfg(windows)]
fn main() -> std::process::ExitCode {
    oteryn_client::crash_report::install_hook();
    let _instance = match oteryn_client::win_mutex::acquire_client_instance() {
        Ok(instance) => instance,
        Err(error) => {
            leprintln!("Oteryn: {error}", error = error);
            return std::process::ExitCode::from(3);
        }
    };
    if let Some(result) = installation_check() {
        return result;
    }
    if std::env::args().any(|arg| arg == "--watch") {
        return match oteryn_client::cast_watch::run() {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                leprintln!("Oteryn Watch: {error}", error = error);
                std::process::ExitCode::FAILURE
            }
        };
    }

    match windows_shell::run(native_client()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            oteryn_client::crash_report::record_startup_failure(
                "client",
                &format!("windows shell failed: {error}"),
            );
            leprintln!("Error: {error:?}", error = error);
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    oteryn_client::crash_report::install_hook();
    if let Some(result) = installation_check() {
        return result;
    }
    if std::env::args().any(|arg| arg == "--watch") {
        leprintln!("Oteryn Watch uses the native Windows desktop shell");
        return std::process::ExitCode::SUCCESS;
    }

    if let Some(client) = native_client()
        && let Ok(Some(config)) =
            oteryn_client::NativeLoginConfig::from_env(|name| std::env::var(name).ok())
    {
        let mut entry = oteryn_client::reentry::Reentry::new(client.with_native_login(config));
        entry.request();
        while entry.poll().is_none() {
            std::thread::park_timeout(std::time::Duration::from_millis(10));
        }
        entry.shutdown();
    }
    lprintln!("Oteryn pre-native client: Windows desktop target only");
    std::process::ExitCode::SUCCESS
}
