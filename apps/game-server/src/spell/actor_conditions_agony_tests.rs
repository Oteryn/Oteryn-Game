//! CONDITIONS-0 §3.5: the mana shield does not absorb an `agony` tick (MAGIC-COND-1).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::super::Vocation;
use super::super::cast::CharacterCastFacts;
use super::super::combat_batch::ManaShieldState;
use super::*;

fn shielded() -> PlayerSpellState {
    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Sorcerer,
            level: 100,
            magic_level: 10,
            max_health: 1000,
            max_mana: 500,
            max_soul: 100,
        },
        0,
        0,
    )
    .unwrap();
    apply_combat_change(
        &mut state,
        &OwnerCombatChange::ManaShield(ManaShieldState {
            capacity: 300,
            expires_ms: 2000,
        }),
        0,
    )
    .unwrap();
    state
}

#[test]
fn agony_hit_bypasses_the_mana_shield_and_other_hits_still_use_it() {
    let state = shielded();
    let (health, mana) = (state.health, state.mana);

    let (agony, damage) = stage_agony_hit(&state, 100, 1_000_000).unwrap();
    assert_eq!(damage.health_after, health - 100);
    assert_eq!(agony.health, health - 100);
    assert_eq!(agony.mana, mana);
    assert_eq!(agony.conditions.mana_shield_at(1_000_000), Some(300));

    let (hit, _) = stage_creature_hit(&state, 100, 1_000_000).unwrap();
    assert_eq!(hit.health, health);
    assert_eq!(hit.mana, mana - 100);
    assert_eq!(hit.conditions.mana_shield_at(1_000_000), Some(200));
}

#[test]
fn lethal_agony_hit_ends_every_condition_like_any_other_hit() {
    let state = shielded();
    let (dead, damage) = stage_agony_hit(&state, state.health, 1_000_000).unwrap();
    assert_eq!(damage.health_after, 0);
    assert_eq!(dead.health, 0);
    assert_eq!(dead.mana, state.mana);
    assert!(dead.conditions.instances().is_empty());
}
