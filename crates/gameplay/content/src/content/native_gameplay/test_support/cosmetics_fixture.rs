//! Test-only access to the existing authored fixture and production decoder.
use super::*;
use serde_json::Value;

pub fn cosmetics_test_packet() -> Value {
    let root = Path::new(if env!("CARGO_PKG_NAME") == "oteryn-game-server" {
        env!("CARGO_MANIFEST_DIR")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../../apps/game-server")
    })
    .join("../..");
    let source = root.join("imports/cipsoft-appearances/outfits");
    let manifest = std::fs::read(source.join("manifest.json")).unwrap();
    let parsed: Value = serde_json::from_slice(&manifest).unwrap();
    let shards: Vec<_> = parsed["files"].as_array().unwrap().iter().map(|pin| {
        let name = pin["path"].as_str().unwrap();
        serde_json::json!({"path":name,"json":std::fs::read_to_string(source.join(name)).unwrap()})
    }).collect();
    let authoring: Value = serde_json::from_slice(include_bytes!(
        "../../../../../../../tools/content-schema/cosmetics-authoring/synthetic-authoring.json"
    ))
    .unwrap();
    serde_json::json!({"schema":"OTERYN_NATIVE_COSMETICS/v1","availability":"authored",
        "authoring":authoring,"source_manifest":String::from_utf8(manifest).unwrap(),
        "source_shards":shards})
}

pub fn decode_cosmetics(bytes: &[u8]) -> Result<NativeCosmeticsView, ContentError> {
    super::super::native_cosmetics::decode(bytes)?.ok_or_else(|| invalid("fixture unavailable"))
}

pub fn activated_with_optional_cosmetics(
    world: oteryn_protocol_oteryn::WorldId,
    source_world: Option<&[u8]>,
    cosmetics: Option<&[u8]>,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let manifest = Path::new(if env!("CARGO_PKG_NAME") == "oteryn-game-server" {
        env!("CARGO_MANIFEST_DIR")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../../apps/game-server")
    })
    .join("../../content/spells.manifest.json");
    let mut supplied = NativeGameplayInput::from_manifest(&manifest).unwrap();
    assert!(
        supplied.loot_tables.is_some(),
        "the manifest pins loot_tables"
    );
    supplied.source_world = source_world.map(pinned);
    if let Some(bytes) = cosmetics {
        supplied.cosmetics = pinned(bytes);
    }
    let room = crate::content::qualify_selected_native_gameplay_room(world, &supplied).unwrap();
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
    .unwrap();
    (controller, pin)
}
