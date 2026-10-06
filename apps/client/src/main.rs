#[cfg(windows)]
mod windows_shell;

/// Native gameplay entry when `OTERYN_PLATFORM_URL` and the other native login settings are set.
/// Only the public outcome is printed: never a token, ticket or grant.
fn native_entry() {
    let config = match oteryn_client::NativeLoginConfig::from_env(|name| std::env::var(name).ok()) {
        Ok(Some(config)) => config,
        Ok(None) => return,
        Err(error) => {
            println!("Oteryn: {error}");
            return;
        }
    };
    let client = match oteryn_client::ClientBootstrap::new() {
        Ok(client) => client.with_native_login(config),
        Err(error) => {
            println!("Oteryn: {error}");
            return;
        }
    };
    match client.request_gameplay_entry() {
        Ok(admitted) => println!(
            "Oteryn: admitted to World {} channel {}",
            admitted.world_id(),
            admitted.channel_id()
        ),
        Err(error) => println!("Oteryn: {error}"),
    }
    client.shutdown();
}

#[cfg(windows)]
fn main() -> Result<(), windows_shell::ShellError> {
    native_entry();
    windows_shell::run()
}

#[cfg(not(windows))]
fn main() {
    native_entry();
    println!("Oteryn pre-native client: Windows desktop target only");
}
