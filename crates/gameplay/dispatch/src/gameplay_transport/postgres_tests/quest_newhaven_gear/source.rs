//! Same production native activation as the existing combat owner fixture.
use super::TestResult;
use crate::content::native_gameplay::{NativeGameplayInput, PinnedGameplayBytes};
use crate::content::{
    ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
    activate_native_entry_room_with_gameplay,
};
use crate::foundation::WorldId;
use sha2::{Digest, Sha256};

pub(super) fn generation(
    world: WorldId,
    partial: bool,
) -> TestResult<(
    ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/spells.manifest.json");
    let mut supplied =
        NativeGameplayInput::from_manifest(&manifest).map_err(super::super::fixture::debug)?;
    let bytes = include_bytes!(
        "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
    )
    .to_vec();
    supplied.source_world = Some(pinned(bytes));
    supplied
        .build_training
        .as_mut()
        .ok_or("training")?
        .content_revision = "content-1".into();
    if partial {
        let profiles = supplied.item_profiles.as_mut().ok_or("profiles")?;
        let mut document: serde_json::Value = serde_json::from_slice(&profiles.bytes)?;
        let records = document["records"].as_array_mut().ok_or("records")?;
        let staff = records
            .iter_mut()
            .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i51119")
            .ok_or("real staff profile")?;
        staff["admission"]["materializable"] = serde_json::Value::Bool(false);
        *profiles = pinned(serde_json::to_vec(&document)?);
    }
    let room = crate::content::qualify_selected_native_gameplay_room(world, &supplied)
        .map_err(super::super::fixture::debug)?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = ContentActivationController::new();
    let pin = activate_native_entry_room_with_gameplay(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &supplied,
    )
    .map_err(super::super::fixture::debug)?;
    Ok((controller, pin))
}

fn pinned(bytes: Vec<u8>) -> PinnedGameplayBytes {
    let sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    PinnedGameplayBytes { bytes, sha256 }
}
