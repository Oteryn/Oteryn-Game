#![allow(clippy::unwrap_used)]
use super::*;
use crate::spell::Vocation;
use crate::spell::cast::CharacterCastFacts;
use crate::spell::combat_batch::ManaShieldState;

fn actor() -> PlayerSpellState {
    PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Knight,
            level: 100,
            magic_level: 10,
            max_health: 1000,
            max_mana: 101,
            max_soul: 100,
        },
        0,
        0,
    )
    .unwrap()
}

#[test]
fn energy_ring_hit_spends_mana_with_hp_spill_and_one_revision() {
    let mut state = actor();
    state.install_equipped_mana_shield(true);
    let before = state.clone();
    let (next, damage) = stage_creature_hit(&state, 53, 0).unwrap();
    assert_eq!((next.health, next.mana), (997, 1));
    assert_eq!(damage.health_after, 997);
    assert_eq!(next.revision(), before.revision() + 1);
    assert_eq!(state, before, "staging does not mutate the predecessor");
    let (next, _) = stage_creature_hit(&next, 5, 0).unwrap();
    assert_eq!((next.health, next.mana), (992, 1));
}

#[test]
fn energy_ring_spell_precedes_ring_then_next_hit_uses_ring() {
    let mut state = actor();
    state.install_equipped_mana_shield(true);
    apply_combat_change(
        &mut state,
        &OwnerCombatChange::ManaShield(ManaShieldState {
            capacity: 3,
            expires_ms: 1000,
        }),
        0,
    )
    .unwrap();
    let (next, _) = stage_creature_hit(&state, 5, 0).unwrap();
    assert_eq!((next.health, next.mana), (998, 98));
    assert_eq!(next.conditions.mana_shield_at(0), None);
    let (next, _) = stage_creature_hit(&next, 5, 0).unwrap();
    assert_eq!((next.health, next.mana), (998, 88));
}

#[test]
fn energy_ring_zero_capacity_spell_does_not_block_equipment_absorption() {
    let mut state = actor();
    state.install_equipped_mana_shield(true);
    apply_definition(
        &mut state,
        definition(
            "spell.empty_shield",
            ConditionValues::ManaShield { duration_ms: 1000 },
        )
        .unwrap(),
        0,
        0,
    )
    .unwrap();
    let (next, _) = stage_creature_hit(&state, 5, 0).unwrap();
    assert_eq!((next.health, next.mana), (1000, 91));
}

#[test]
fn energy_ring_unequipped_and_agony_hits_do_not_absorb() {
    let mut state = actor();
    state.install_equipped_mana_shield(true);
    let (agony, _) = stage_agony_hit(&state, 5, 0).unwrap();
    assert_eq!((agony.health, agony.mana), (995, 101));
    state.install_equipped_mana_shield(false);
    let (next, _) = stage_creature_hit(&state, 5, 0).unwrap();
    assert_eq!((next.health, next.mana), (995, 101));
}
