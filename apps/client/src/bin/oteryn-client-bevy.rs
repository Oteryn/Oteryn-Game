//! Opt-in production composition; the legacy client entry point remains intact.
fn main() -> std::process::ExitCode {
    oteryn_client::crash_report::install_hook();
    #[cfg(windows)]
    let _instance = match oteryn_client::win_mutex::acquire_client_instance() {
        Ok(instance) => instance,
        Err(error) => {
            eprintln!("Oteryn: {error}");
            return std::process::ExitCode::from(3);
        }
    };
    let requested = std::env::args_os().any(|arg| arg == "--check-packages");
    match oteryn_client::package_integrity::verify_current(requested) {
        Ok(report) if requested => {
            println!("{report}");
            return std::process::ExitCode::SUCCESS;
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("Oteryn installation check failed: {error}");
            #[cfg(windows)]
            if !requested {
                oteryn_client::package_integrity::show_repair_required(&error);
            }
            return std::process::ExitCode::FAILURE;
        }
    }
    match oteryn_client::bevy_shell::run(std::env::args_os().skip(1)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Oteryn Bevy host: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
