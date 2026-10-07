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

#[cfg(windows)]
fn main() -> Result<(), windows_shell::ShellError> {
    windows_shell::run(native_entry())
}

#[cfg(not(windows))]
fn main() {
    if let Some((client, _admitted)) = native_entry() {
        client.shutdown();
    }
    println!("Oteryn pre-native client: Windows desktop target only");
}
