use oteryn_game_server::{GAMEPLAY_UNAVAILABLE_REASON, bootstrap_smoke};
use std::ffi::OsStr;
use std::process::ExitCode;

fn main() -> ExitCode {
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
