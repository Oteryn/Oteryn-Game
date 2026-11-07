//! Offline catalogue adapter; producer.py owns validation, canonicalization and digests.
use oteryn_gameplay_content::content::{
    capture_world_project, npc_catalogue_preproduction_limits, public_content_view,
};
use std::{io::Write, path::Path};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [root, revision, generated_at] = args.as_slice() else {
        return Err("usage: oteryn-game-platform-catalog <project-root> <source-revision|--inventory> <generated-at>".into());
    };
    let root = Path::new(root);
    let parent = root
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let basename = root.file_name().ok_or("project root basename absent")?;
    let project = capture_world_project(parent, basename, npc_catalogue_preproduction_limits())?;
    let view = public_content_view(&project)?;
    let value = if revision == "--inventory" {
        serde_json::to_value(view.field_fingerprints())?
    } else {
        view.native_input(revision, generated_at)
    };
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, &value)?;
    out.write_all(b"\n")?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "catalogue adapter refused: {}",
                error.to_string().chars().take(512).collect::<String>()
            );
            std::process::ExitCode::FAILURE
        }
    }
}
