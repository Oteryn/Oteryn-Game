#[cfg(windows)]
mod windows_shell;

/// Native gameplay entry when `OTERYN_PLATFORM_URL` and the other native login settings are set.
/// Only the public outcome is printed: never a token, ticket or grant. An admitted session is
/// returned with the client whose runtime it is bound to.
fn native_entry() -> Option<(
    oteryn_client::ClientBootstrap,
    oteryn_client::AdmittedSession,
)> {
    let config = match oteryn_client::NativeLoginConfig::from_env(|name| std::env::var(name).ok()) {
        Ok(Some(config)) => config,
        Ok(None) => return None,
        Err(error) => {
            println!("Oteryn: {error}");
            return None;
        }
    };
    let client = match oteryn_client::ClientBootstrap::new() {
        Ok(client) => client.with_native_login(config),
        Err(error) => {
            println!("Oteryn: {error}");
            return None;
        }
    };
    match client.request_gameplay_entry() {
        Ok(admitted) => {
            println!(
                "Oteryn: admitted to World {} channel {}",
                admitted.world_id(),
                admitted.channel_id()
            );
            Some((client, admitted))
        }
        Err(error) => {
            println!("Oteryn: {error}");
            client.shutdown();
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
    match windows_shell::run(native_entry()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error:?}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() {
    if let Some((client, _admitted)) = native_entry() {
        client.shutdown();
    }
    println!("Oteryn pre-native client: Windows desktop target only");
}
