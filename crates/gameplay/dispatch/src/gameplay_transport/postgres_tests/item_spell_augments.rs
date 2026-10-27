//! SPELL-AUGMENT-1: framed ordinary cast with a real PostgreSQL equipment read.
use super::{TestResult, debug};
use crate::content::*;
use crate::foundation::WorldId;
use sha2::{Digest, Sha256};

#[test]
fn item_spell_augments_postgres_equipped_healing_cooldown_unequip_and_replay() -> TestResult {
    super::parameter_spell::scenario(4)
}

#[test]
fn item_spell_augments_postgres_damage_critical_leech_and_replay() -> TestResult {
    super::parameter_spell::scenario(5)
}

pub(super) fn activate(
    world: WorldId,
) -> TestResult<(ContentActivationController, NativeEntryContentPin)> {
    let mut input = native_gameplay::NativeGameplayInput::from_manifest(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json"),
    )?;
    let bytes = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
    );
    let pinned = |bytes: Vec<u8>| native_gameplay::PinnedGameplayBytes {
        sha256: Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        bytes,
    };
    input.source_world = Some(pinned(bytes.to_vec()));
    input
        .build_training
        .as_mut()
        .ok_or("training")?
        .content_revision = "content-1".into();
    let pin = input.item_profiles.as_mut().ok_or("item profiles")?;
    let mut document: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    let item = document["records"]
        .as_array_mut()
        .ok_or("records")?
        .iter_mut()
        .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i2854")
        .ok_or("backpack")?;
    // Qualified synthetic values isolate the executable field. XML unit conversion has its
    // own pinned-source tests; no production Item value is asserted by this fixture.
    item["attributes"]["spell_augments"] = serde_json::json!([
        {"spell":"light healing","kind":"base_healing","value":100000},
        {"spell":"light healing","kind":"cooldown","value":100000},
        {"spell":"intense healing","kind":"base_healing","value":100000},
        {"spell":"energy strike","kind":"base_damage","value":10000},
        {"spell":"energy strike","kind":"critical_hit_chance","value":10000},
        {"spell":"energy strike","kind":"critical_extra_damage","value":10000},
        {"spell":"energy strike","kind":"life_leech","value":10000},
        {"spell":"energy strike","kind":"mana_leech","value":10000}
    ]);
    *pin = pinned(serde_json::to_vec(&document)?);
    let room = qualify_selected_native_gameplay_room(world, &input).map_err(debug)?;
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
        &input,
    )
    .map_err(debug)?;
    Ok((controller, pin))
}
