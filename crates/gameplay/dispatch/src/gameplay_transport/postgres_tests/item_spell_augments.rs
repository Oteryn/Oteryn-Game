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

#[test]
fn dmg2_postgres_equipment_spell_leech_without_augment_and_replay() -> TestResult {
    super::parameter_spell::scenario(6)
}

pub(super) fn activate(
    world: WorldId,
    equipment_leech: bool,
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
    let key = if equipment_leech {
        "oteryn:item.tibia.i6529"
    } else {
        "oteryn:item.tibia.i2854"
    };
    let item = document["records"]
        .as_array_mut()
        .ok_or("records")?
        .iter_mut()
        .find(|r| r["production_definition"]["production_key"] == key)
        .ok_or("fixture item")?;
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
    if equipment_leech {
        // No spell augment: the real equipment read must supply both leech rates.
        item["attributes"]["spell_augments"] = serde_json::json!([]);
        let modifiers = ["LIFE_LEECH_AMOUNT", "MANA_LEECH_AMOUNT"].map(|kind| {
            serde_json::json!({
                "kind": kind,
                "parameter": {"state":"KNOWN", "value": {
                    "kind":"RATIONAL_PERCENT", "value":{"numerator":100,"denominator":1}
                }},
                "evaluation_phase":{"state":"UNKNOWN"},
                "priority":{"state":"UNKNOWN"},
                "target_domain":{"state":"UNKNOWN"}
            })
        });
        item["semantics"]["skill_modifiers"] = serde_json::json!({
            "state":"KNOWN", "value":{"modifiers":{"state":"KNOWN", "value":modifiers}}
        });
    }
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
