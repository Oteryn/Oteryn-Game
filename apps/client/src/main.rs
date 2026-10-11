#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod actor_hud;
#[cfg(windows)]
mod login_backdrop;
#[cfg(windows)]
mod login_screen;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod options_export;
#[cfg(any(windows, test))]
mod preferences_browser;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod reference_dialogs;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod settings_audit;
#[cfg(test)]
mod settings_semantic_contract;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod settings_ui;

#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod action_bar_ui;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod client_chrome;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod client_panels;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod game_ui;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod mouse_cursor;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
mod panel_icons;
#[cfg(any(windows, test))]
#[cfg_attr(test, allow(dead_code))]
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
    let audit = match settings_audit::parse_settings_audit_request(std::env::args_os().skip(1)) {
        Ok(audit) => audit,
        Err(error) => {
            eprintln!("Oteryn: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let _instance = match oteryn_client::win_mutex::acquire_client_instance() {
        Ok(instance) => instance,
        Err(error) => {
            eprintln!("Oteryn: {error}");
            return std::process::ExitCode::from(3);
        }
    };
    let entry =
        match if audit.is_none() && std::env::args().any(|argument| argument == "--auto-login") {
            native_entry()
        } else {
            Ok(None)
        } {
            Ok(entry) => entry,
            Err(()) => return std::process::ExitCode::FAILURE,
        };
    match windows_shell::run(entry, audit) {
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
