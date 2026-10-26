use super::*;
use crate::content::native_gameplay::{ItemProfilesDocument, NativeGameplayInput};
use crate::durability::character_equipment::EquippedItem;
use crate::durability::item_mint::TypedDefinitionRef;
use oteryn_gameplay_content::content::item_abilities::{AbilitySuppression, ItemAbility};
use std::error::Error;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn input() -> Result<NativeGameplayInput, Box<dyn Error>> {
    let mut input = NativeGameplayInput::from_manifest(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json"),
    )?;
    // These fixtures exercise the production item profiles in the accepted entry room.
    // Compiling the unrelated source-world carrier in every ability case adds contention
    // to the concurrent PostgreSQL owners without exercising another equipment path.
    input.source_world = None;
    Ok(input)
}

fn native(input: &NativeGameplayInput) -> Result<NativeGameplayState, Box<dyn Error>> {
    let world = crate::foundation::WorldId::decode(&[
        1, 0x90, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
    ])?;
    let (controller, _) = activated(input, world)?;
    Ok(controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native state")?
        .clone())
}

fn activated(
    input: &NativeGameplayInput,
    world: crate::foundation::WorldId,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use crate::content::{
        ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
        activate_native_entry_room_with_gameplay,
    };
    let room = crate::content::qualify_selected_native_gameplay_room(world, input)?;
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
        input,
    )
    .map_err(debug)?;
    Ok((controller, pin))
}

fn snapshot(content: &NativeGameplayState, key: &str, slot: u8) -> EquipmentSnapshot {
    EquipmentSnapshot {
        character: [1; 16],
        content_digest: content.source_digest(),
        character_revision: 1,
        revision: 1,
        combat_mode: None,
        items: vec![EquippedItem {
            tier: 0,
            slot,
            item_instance_id: [2; 16],
            quantity: 1,
            state_revision: 1,
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            },
        }],
    }
}

fn fixture_input(
    modifiers: serde_json::Value,
    speed: Option<i32>,
) -> Result<NativeGameplayInput, Box<dyn Error>> {
    let mut input = input()?;
    let pin = input.item_profiles.as_mut().ok_or("profiles")?;
    let mut profiles: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    let record = profiles["records"]
        .as_array_mut()
        .ok_or("records")?
        .iter_mut()
        .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i2854")
        .ok_or("backpack")?;
    record["semantics"]["skill_modifiers"] = serde_json::json!({"state":"KNOWN","value":{"modifiers":{"state":"KNOWN","value":modifiers}}});
    record["attributes"]["speed_bonus"] = serde_json::to_value(speed)?;
    pin.bytes = serde_json::to_vec(&profiles)?;
    use sha2::{Digest, Sha256};
    pin.sha256 = Sha256::digest(&pin.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(input)
}

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn debug(error: impl std::fmt::Debug) -> Box<dyn Error> {
    format!("{error:?}").into()
}

#[path = "equipment_effects_owner_tests/ability_tests.rs"]
mod ability_tests;
#[path = "equipment_effects_owner_tests/armor_tests.rs"]
mod armor_tests;
#[path = "equipment_effects_owner_tests/committed_moves_tests.rs"]
mod committed_moves_tests;
#[path = "equipment_effects_owner_tests/defence_tests.rs"]
mod defence_tests;
#[path = "equipment_effects_owner_tests/first_entry_tests.rs"]
mod first_entry_tests;
#[path = "equipment_effects_owner_tests/offence_tests.rs"]
mod offence_tests;
#[path = "equipment_effects_owner_tests/postgres_scenario.rs"]
mod postgres_scenario;
