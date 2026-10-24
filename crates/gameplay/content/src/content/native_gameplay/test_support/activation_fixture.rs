//! Reused native activation fixture and LOOP-MONSTER source-profile variants.
use super::*;

fn manifest_input() -> NativeGameplayInput {
    let manifest = Path::new(if env!("CARGO_PKG_NAME") == "oteryn-game-server" {
        env!("CARGO_MANIFEST_DIR")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../../apps/game-server")
    })
    .join("../../content/spells.manifest.json");
    NativeGameplayInput::from_manifest(&manifest).unwrap()
}
fn activate(
    world: oteryn_protocol_oteryn::WorldId,
    supplied: &NativeGameplayInput,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let room = crate::content::qualify_selected_native_gameplay_room(world, supplied).unwrap();
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
        supplied,
    )
    .unwrap();
    (controller, pin)
}

pub fn activated_with_loot_tables(
    world: oteryn_protocol_oteryn::WorldId,
    source_world: Option<&[u8]>,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let mut supplied = manifest_input();
    assert!(
        supplied.loot_tables.is_some(),
        "the manifest pins loot_tables"
    );
    supplied.source_world = source_world.map(pinned);
    activate(world, &supplied)
}

pub fn activated_loop_monster(
    world: oteryn_protocol_oteryn::WorldId,
    summons: bool,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    activate(world, &loop_monster_input(summons))
}
fn loop_monster_input(summons: bool) -> NativeGameplayInput {
    let mut supplied = manifest_input();
    supplied.source_world = Some(pinned(include_bytes!(
        "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
    )));
    if summons {
        let mut creatures: CreatureProfilesDocument =
            serde_json::from_slice(&supplied.creature_profiles.bytes).unwrap();
        // Reuse one real admitted ranged Spell body rather than inventing executable effects.
        let source = creatures
            .records
            .iter()
            .find(|r| !r.source_item_abilities.is_empty())
            .expect("source Item spell closure");
        let ability = source.source_item_abilities[0].clone();
        let Some(ProjectV2AuthoringProfile {
            data: ProjectV2AuthoringProfileData::Behavior(source_behavior),
            ..
        }) = &source.behavior
        else {
            panic!("source Behavior");
        };
        let mut attack = source_behavior
            .attacks
            .iter()
            .find(|entry| entry.ability == ability.target)
            .expect("source attack")
            .clone();
        attack.interval_ms = 1000;
        attack.chance_ppm = 1_000_000;
        let rat = creatures
            .records
            .iter_mut()
            .find(|r| r.profile.target.key == "oteryn:creature.rat")
            .expect("admitted rat");
        let child = rat.profile.target.clone();
        if let ProjectV2AuthoringProfileData::Creature(profile) = &mut rat.profile.data {
            profile.abilities = vec![ability.target.clone()];
        }
        rat.source_item_abilities = vec![ability];
        rat.monster_melee = None;
        let Some(ProjectV2AuthoringProfile {
            data: ProjectV2AuthoringProfileData::Behavior(behavior),
            ..
        }) = &mut rat.behavior
        else {
            panic!("rat Behavior closure");
        };
        behavior.attacks = vec![attack];
        behavior.defenses.clear();
        behavior.summons = Some(serde_json::from_value(serde_json::json!({"max_summons":2,"entries":[{"creature":child,"interval_ms":1000,"chance_ppm":1000000,"count":2}]})).unwrap());
        supplied.creature_profiles = pinned(&serde_json::to_vec(&creatures).unwrap());
    }
    supplied
}

pub fn loop_monster_inner_digest(pin: &crate::content::NativeEntryContentPin) -> [u8; 32] {
    let compiled = pin.qualified_room().compiled();
    let decoded = crate::content::native_source_world_carrier::decode(
        &compiled.server_artifact,
        &compiled.client_artifact,
    )
    .expect("source outer envelope");
    sha256(decoded.baseline_server)
}

pub fn activated_loop_monster_missing_child(
    world: oteryn_protocol_oteryn::WorldId,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let mut supplied = loop_monster_input(true);
    let mut creatures: CreatureProfilesDocument =
        serde_json::from_slice(&supplied.creature_profiles.bytes).unwrap();
    let child = creatures
        .records
        .iter_mut()
        .find(|r| r.profile.target.key == "oteryn:creature.snake")
        .expect("admitted child");
    child.behavior = None;
    child.monster_melee = None;
    child.source_item_abilities.clear();
    let child = child.profile.target.clone();
    let rat = creatures
        .records
        .iter_mut()
        .find(|r| r.profile.target.key == "oteryn:creature.rat")
        .expect("admitted parent");
    let Some(ProjectV2AuthoringProfile {
        data: ProjectV2AuthoringProfileData::Behavior(behavior),
        ..
    }) = &mut rat.behavior
    else {
        panic!("parent Behavior");
    };
    behavior.summons.as_mut().expect("summons").entries[0].creature = child;
    supplied.creature_profiles = pinned(&serde_json::to_vec(&creatures).unwrap());
    activate(world, &supplied)
}
