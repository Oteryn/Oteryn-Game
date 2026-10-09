#[cfg(windows)]
mod actor_hud;
#[cfg(windows)]
mod login_screen;
#[cfg(windows)]
mod preferences_browser;
#[cfg(windows)]
mod reference_dialogs;
#[cfg(windows)]
mod settings_ui;

#[cfg(windows)]
mod action_bar_ui;
#[cfg(windows)]
mod client_chrome;
#[cfg(windows)]
mod client_panels;
#[cfg(windows)]
mod game_ui;
#[cfg(windows)]
mod panel_icons;
#[cfg(windows)]
mod panel_views;
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
    let entry = match if std::env::args().any(|argument| argument == "--auto-login") {
        native_entry()
    } else {
        Ok(None)
    } {
        Ok(entry) => entry,
        Err(()) => return std::process::ExitCode::FAILURE,
    };
    match windows_shell::run(entry) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error:?}");
            std::process::ExitCode::FAILURE
        }
    }
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
