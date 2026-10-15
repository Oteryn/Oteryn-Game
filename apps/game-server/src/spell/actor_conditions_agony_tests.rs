//! CONDITIONS-0 §3.5: the mana shield does not absorb an `agony` tick, and the condition
//! cycle deals only a non-lethal creature agony hit itself (MAGIC-COND-1).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::super::Vocation;
use super::super::cast::CharacterCastFacts;
use super::super::combat_batch::ManaShieldState;
use super::*;
use crate::ability::condition::DotElement;

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

fn with_agony(source: &str, kind: ConditionSourceKind, total: u32) -> PlayerSpellState {
    let mut state = shielded();
    let root = GameplayDecisionRoot::from_bytes([3; 32]);
    let definition = ConditionDefinition::new(
        "cond.agony",
        1,
        ConditionValues::Agony {
            total_min: total,
            total_max: total,
            per_tick: total,
            interval_ms: 2_000,
            delayed: false,
        },
    )
    .unwrap();
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([4; 16]),
    };
    state
        .conditions
        .apply(&definition, Some(source.to_owned()), kind, &[], &facts)
        .unwrap();
    state
}

const OPEN: TickFacts = TickFacts {
    in_protection_zone: false,
    standing_on_field: None,
};

#[test]
fn the_cycle_deals_a_non_lethal_creature_agony_tick_past_the_mana_shield() {
    let mut state = with_agony("creature:test", ConditionSourceKind::Creature, 100);
    let (health, mana) = (state.health, state.mana);
    assert!(tick(&mut state, 0, OPEN).unwrap().is_empty());
    assert_eq!(state.health, health - 100);
    assert_eq!(state.mana, mana);
    assert!(
        state
            .conditions
            .get(crate::ability::condition::ConflictKey::Agony)
            .is_none()
    );
}

#[test]
fn the_cycle_hands_lethal_or_non_creature_agony_to_the_combat_owner() {
    let lethal = shielded().health;
    for mut state in [
        with_agony("creature:test", ConditionSourceKind::Creature, lethal),
        with_agony("player:test", ConditionSourceKind::Player, 100),
    ] {
        let health = state.health;
        let combat = tick(&mut state, 0, OPEN).unwrap();
        assert!(matches!(
            combat.as_slice(),
            [ConditionTick {
                kind: TickKind::Agony { refused: false, .. },
                ..
            }]
        ));
        assert_eq!(state.health, health);
    }
}

#[test]
fn the_cycle_uses_up_a_refused_agony_tick_in_a_protection_zone() {
    let mut state = with_agony("creature:test", ConditionSourceKind::Creature, 100);
    let health = state.health;
    let pz = TickFacts {
        in_protection_zone: true,
        standing_on_field: Some(DotElement::Fire),
    };
    assert!(tick(&mut state, 0, pz).unwrap().is_empty());
    assert_eq!(state.health, health);
    assert!(
        state
            .conditions
            .get(crate::ability::condition::ConflictKey::Agony)
            .is_none()
    );
    assert!(has_periodic_damage(&with_agony(
        "creature:test",
        ConditionSourceKind::Creature,
        100
    )));
}
