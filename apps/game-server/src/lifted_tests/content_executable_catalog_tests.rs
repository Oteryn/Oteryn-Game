#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use crate::content::spell_data::executable_catalog::compile;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SAMPLE: &[u8] = include_bytes!(
    "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
);

#[test]
fn native_wheel_guard_uses_owned_facts_before_any_plan_or_draw() {
    let digest: String = Sha256::digest(SAMPLE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let catalog = compile(SAMPLE, &digest).unwrap();
    let definition = catalog
        .entries
        .iter()
        .find(|entry| {
            entry.native.is_some() && entry.header.requirements.wheel_unlock == Some(true)
        })
        .unwrap()
        .definition
        .as_ref()
        .unwrap();
    let caster = crate::spell::CasterState {
        vocation: *definition.vocations.iter().next().unwrap(),
        level: 999,
        magic_level: 999,
        premium: crate::spell::CasterPremium::Current,
        mana: 1_000_000,
        max_mana: 1_000_000,
        soul: 10_000,
        learned: BTreeSet::new(),
        attack_skill: 100,
        attack_value: 100,
        attack_factor: 1.0,
        shielding_skill: 100,
        melee_weapon: true,
        shield_defense: Some(100),
        harmony_multiplier: crate::spell::harmony::HarmonyMultiplier::ONE,
    };
    let position = crate::spell::chain::TilePosition {
        x: 0,
        y: 0,
        floor: 0,
    };
    let facts = crate::spell::OperationalCastFacts {
        caster_position: position,
        target_position: Some(position),
        target: None,
        line_of_sight_clear: Some(true),
        direction_available: true,
        wheel_unlocked: None,
        in_protection_zone: false,
        target_tile_solid: Some(false),
        target_tile_creature: Some(false),
    };
    let cooldowns = crate::spell::Cooldowns::default();
    let result = crate::spell::resolve_native_cast(
        definition,
        &caster,
        &cooldowns,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        &facts,
        crate::spell::native::Facts::Food,
        &mut |_, _| panic!("a rejected header cannot draw"),
    );
    assert_eq!(
        result.unwrap_err(),
        crate::spell::CastRejection::WheelUnlockRequired
    );
    assert!(cooldowns.canonical_deadlines().is_empty());
    assert_eq!(caster.mana, 1_000_000);
}
