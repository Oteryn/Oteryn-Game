//! Actual node manifest loader and activation; no test input sections are cleared.
use super::*;
use sha2::{Digest, Sha256};
pub(in crate::gameplay_transport) fn repository_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        if env!("CARGO_PKG_NAME") == "oteryn-game-server" {
            "../.."
        } else {
            "../../.."
        },
    )
}
pub(in crate::gameplay_transport) fn activate(
    world: crate::foundation::WorldId,
    artifact_export_env: &str,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    let manifest = repository_root().join("content/spells.manifest.json");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(&manifest)
        .map_err(debug)?;
    assert!(
        input.source_world.is_some(),
        "real node manifest retains its pinned source_world"
    );
    let room =
        crate::content::qualify_selected_native_gameplay_room(world, &input).map_err(debug)?;
    let production_client_sha: [u8; 32] = Sha256::digest(&room.compiled().client_artifact).into();
    assert_eq!(
        production_client_sha,
        room.compiled().client_digest(),
        "independent actual production compiled bytes"
    );
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: production_client_sha,
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = crate::content::ContentActivationController::new();
    let pin = crate::content::activate_native_entry_room_with_gameplay(
        &mut controller,
        &crate::content::NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &input,
    )
    .map_err(debug)?;
    assert_eq!(
        controller
            .active()
            .ok_or("actual activated production generation")?
            .identity()
            .client_artifact_digest(),
        production_client_sha,
        "the runtime activation digest equals independent production compiled client bytes, not a wire header echo"
    );
    if let Some(path) = std::env::var_os(artifact_export_env) {
        std::fs::write(path, &room.compiled().client_artifact)?;
    }
    Ok((controller, pin))
}
