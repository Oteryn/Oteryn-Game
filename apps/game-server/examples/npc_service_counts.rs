//! Read-only production catalogue census for tools/contracts/refresh_npc_service_counts.py.
use oteryn_game_server::content::*;
use std::{ffi::OsStr, path::PathBuf};

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let project = capture_world_project(
        &root,
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let catalogue =
        NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)
            .map_err(|error| format!("{error:?}"))?;
    let model = catalogue
        .service_model_for_project()
        .map_err(|error| format!("{error:?}"))?;
    println!(
        "NPC_SERVICE_COUNTS {}",
        serde_json::json!({
            "npcs": model.npcs().len(), "held_npcs": model.held_npcs().len(),
            "services": model.services().len(), "offers": model.admitted_offer_count(),
            "routes": model.admitted_route_count(), "replies": model.generated_reply_npc_count(),
            "held": model.held_offer_counts(), "source_tree_digest": model.source_tree_digest(),
            "service_keys": model.services().keys().map(|r| r.key.as_str()).collect::<Vec<_>>(),
        })
    );
    Ok(())
}
