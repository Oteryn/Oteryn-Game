#[cfg(windows)]
mod login_screen;
#[cfg(windows)]
mod preferences_browser;
#[cfg(windows)]
mod settings_ui;

#[cfg(windows)]
mod client_panels;
#[cfg(windows)]
mod game_ui;
#[cfg(windows)]
mod windows_shell;

type NativeGameplay = (
    oteryn_client::ClientBootstrap,
    oteryn_client::AdmittedSession,
);

/// Native gameplay entry when `OTERYN_PLATFORM_URL` and the other native login settings are set.
/// Only the public outcome is printed: never a token, ticket or grant. An admitted session is
/// returned with the client whose runtime it is bound to.
fn native_entry() -> Result<Option<NativeGameplay>, ()> {
    let config = match oteryn_client::NativeLoginConfig::from_env(|name| std::env::var(name).ok()) {
        Ok(Some(config)) => config,
        Ok(None) => return Ok(None),
        Err(error) => {
            println!("Oteryn: {error}");
            return Err(());
        }
    };
    let client = match oteryn_client::ClientBootstrap::new() {
        Ok(client) => client.with_native_login(config),
        Err(error) => {
            println!("Oteryn: {error}");
            return Err(());
        }
    };
    match client.request_gameplay_entry() {
        Ok(admitted) => {
            println!(
                "Oteryn: admitted to World {} channel {}",
                admitted.world_id(),
                admitted.channel_id()
            );
            Ok(Some((client, admitted)))
        }
        Err(error) => {
            println!("Oteryn: {error}");
            client.shutdown();
            Err(())
        }
    }
}

#[cfg(windows)]
fn main() -> Result<(), windows_shell::ShellError> {
    let entry = match if std::env::args().any(|argument| argument == "--auto-login") {
        native_entry()
    } else {
        Ok(None)
    } {
        Ok(entry) => entry,
        Err(()) => return Ok(()),
    };
    windows_shell::run(entry)
}

#[cfg(not(windows))]
fn main() {
    let entry = match native_entry() {
        Ok(entry) => entry,
        Err(()) => return,
    };
    if let Some((client, _admitted)) = entry {
        client.shutdown();
    }
    println!("Oteryn pre-native client: Windows desktop target only");
}
