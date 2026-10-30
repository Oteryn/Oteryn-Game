#[cfg(windows)]
mod windows_shell;

/// Build the process runtime once; the window owns every native entry attempt.
fn native_client() -> Option<oteryn_client::ClientBootstrap> {
    let config = match oteryn_client::NativeLoginConfig::from_env(|name| std::env::var(name).ok()) {
        Ok(Some(config)) => config,
        Ok(None) => return None,
        Err(error) => {
            println!("Oteryn: {error}");
            return None;
        }
    };
    match oteryn_client::ClientBootstrap::new() {
        Ok(client) => Some(client.with_native_login(config)),
        Err(error) => {
            println!("Oteryn: {error}");
            None
        }
    }
}

/// Holds `Global\OterynClient-<SID>` for the process lifetime and refuses to start while an
/// install or uninstall holds the transaction mutex (CLIENT-INSTALLER-0 §2.1).
#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let _instance = match oteryn_client::win_mutex::acquire_client_instance() {
        Ok(instance) => instance,
        Err(error) => {
            eprintln!("Oteryn: {error}");
            return std::process::ExitCode::from(3);
        }
    };
    match windows_shell::run(native_client()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error:?}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() {
    if let Some(client) = native_client() {
        let mut entry = oteryn_client::reentry::Reentry::new(client);
        entry.request();
        while entry.poll().is_none() {
            std::thread::park_timeout(std::time::Duration::from_millis(10));
        }
        entry.shutdown();
    }
    println!("Oteryn pre-native client: Windows desktop target only");
}
