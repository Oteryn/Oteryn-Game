use oteryn_game_server::{GAMEPLAY_UNAVAILABLE_REASON, bootstrap_smoke};
use std::ffi::OsStr;
use std::process::ExitCode;

fn main() -> ExitCode {
    use oteryn_game_server::node::serve::{LogKind, begin_process};
    let process = "oteryn-game-server";
    let version = env!("CARGO_PKG_VERSION");
    if begin_process(process, version, LogKind::SpecInvalid.code()).is_err() {
        return ExitCode::from(2);
    }
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.first().map(|argument| argument.as_os_str()) == Some(OsStr::new("--version")) {
        println!("{process} {}", oteryn_error_codes::build_id(version));
        return ExitCode::SUCCESS;
    }
    // `serve --config <path>` starts one GameNode (OPS-NODE-BOOT-01).
    if arguments.first().map(|argument| argument.as_os_str()) == Some(OsStr::new("serve")) {
        return serve(&arguments[1..]);
    }
    if arguments.first().map(|argument| argument.as_os_str()) == Some(OsStr::new("npc-import")) {
        return import_npc_data(&arguments[1..]);
    }
    let smoke = std::env::args_os().any(|argument| argument == OsStr::new("--smoke"));
    if smoke {
        return match bootstrap_smoke() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                diagnose(
                    "smoke_failed",
                    &format!("game-server bootstrap smoke failed: {error}"),
                );
                ExitCode::from(1)
            }
        };
    }

    diagnose("gameplay_unavailable", GAMEPLAY_UNAVAILABLE_REASON);
    ExitCode::from(2)
}

/// One uncoded error line of the server process; the free text goes in `detail`.
fn diagnose(event: &str, detail: &str) {
    use oteryn_error_codes::{Level, Line};
    oteryn_game_server::node::serve::emit(
        "oteryn-game-server",
        &Line::new(Level::Error, "main", event).detail(detail),
    );
}

fn serve(arguments: &[std::ffi::OsString]) -> ExitCode {
    let (flag, path, npc_data) = match arguments {
        [flag, path] => (flag, path, None),
        [flag, path, project_flag, project, digest_flag, digest]
            if project_flag == OsStr::new("--npc-data-project")
                && digest_flag == OsStr::new("--npc-data-sha256") =>
        {
            let Some(digest) = digest.to_str() else {
                return ExitCode::from(2);
            };
            (flag, path, Some((std::path::Path::new(project), digest)))
        }
        _ => {
            diagnose(
                "usage_invalid",
                "usage: oteryn-game-server serve --config <path> [--npc-data-project <path> --npc-data-sha256 <sha256>]",
            );
            return ExitCode::from(2);
        }
    };
    if flag != OsStr::new("--config") {
        diagnose(
            "usage_invalid",
            "usage: oteryn-game-server serve --config <path>",
        );
        return ExitCode::from(2);
    }
    // `block_on` keeps the future it polls on the calling thread's stack. The node's
    // supervision future (gameplay, control and every scheduler loop pinned side by side) is
    // large in a debug build and overflowed the 8 MiB main thread at admission. Box the future
    // and poll it on a dedicated thread, and run the workers, with an explicit stack.
    const NODE_STACK_BYTES: usize = 64 << 20;
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(NODE_STACK_BYTES)
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => return ExitCode::from(19),
    };
    let path = std::path::PathBuf::from(path);
    let npc_data = npc_data.map(|(project, digest)| (project.to_path_buf(), digest.to_owned()));
    let served = std::thread::Builder::new()
        .name("game-node".into())
        .stack_size(NODE_STACK_BYTES)
        .spawn(move || {
            runtime.block_on(Box::pin(
                oteryn_game_server::node::serve::run_with_npc_data_project(
                    &path,
                    npc_data
                        .as_ref()
                        .map(|(project, digest)| (project.as_path(), digest.as_str())),
                ),
            ))
        });
    let Ok(served) = served.map(std::thread::JoinHandle::join) else {
        return ExitCode::from(19);
    };
    match served {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(error)) => {
            oteryn_game_server::node::serve::report_boot_failure(&error);
            ExitCode::from(error.exit_code())
        }
        Err(_) => ExitCode::from(19),
    }
}

fn import_npc_data(arguments: &[std::ffi::OsString]) -> ExitCode {
    let [root_flag, root, digest_flag, digest] = arguments else {
        diagnose(
            "usage_invalid",
            "usage: oteryn-game-server npc-import --project-root <path> --expected-tree-sha256 <sha256>",
        );
        return ExitCode::from(2);
    };
    if root_flag != OsStr::new("--project-root")
        || digest_flag != OsStr::new("--expected-tree-sha256")
    {
        return ExitCode::from(2);
    }
    let Some(digest) = digest.to_str() else {
        return ExitCode::from(2);
    };
    match oteryn_game_server::content::load_data_only_npc_catalogue(
        std::path::Path::new(root),
        digest,
    ) {
        Ok(catalogue) => {
            println!(
                "{}",
                serde_json::json!({
                    "status": "IMPORTED_DATA_ONLY",
                    "project_revision": catalogue.project_revision(),
                    "source_tree_sha256": catalogue.source_tree_digest(),
                    "npcs": catalogue.npc_count(),
                    "dialogues": catalogue.dialogue_count(),
                    "services": catalogue.service_count(),
                    "profiles": catalogue.profile_count(),
                    "spawned_actors": 0,
                    "activated_services": 0
                })
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            diagnose(
                "npc_import_failed",
                &format!("NPC data import failed: {error}"),
            );
            ExitCode::from(19)
        }
    }
}
