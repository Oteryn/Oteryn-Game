use oteryn_game_server::{GAMEPLAY_UNAVAILABLE_REASON, bootstrap_smoke};
use std::ffi::OsStr;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    // `serve --config <path>` starts one GameNode (OPS-NODE-BOOT-01).
    if arguments.first().map(|argument| argument.as_os_str()) == Some(OsStr::new("serve")) {
        return serve(&arguments[1..]);
    }
    let smoke = std::env::args_os().any(|argument| argument == OsStr::new("--smoke"));
    if smoke {
        return match bootstrap_smoke() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("game-server bootstrap smoke failed: {error}");
                ExitCode::from(1)
            }
        };
    }

    eprintln!("Oteryn Game Server gameplay unavailable: {GAMEPLAY_UNAVAILABLE_REASON}");
    ExitCode::from(2)
}

fn serve(arguments: &[std::ffi::OsString]) -> ExitCode {
    let [flag, path] = arguments else {
        eprintln!("usage: oteryn-game-server serve --config <path>");
        return ExitCode::from(2);
    };
    if flag != OsStr::new("--config") {
        eprintln!("usage: oteryn-game-server serve --config <path>");
        return ExitCode::from(2);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => return ExitCode::from(19),
    };
    match runtime.block_on(oteryn_game_server::node::serve::run(std::path::Path::new(
        path,
    ))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("oteryn-game-server event=boot_failed reason=\"{error}\"");
            ExitCode::from(error.exit_code())
        }
    }
}
