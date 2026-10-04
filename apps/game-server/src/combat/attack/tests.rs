//! ATTACK-1a unit tests against the ATTACK-0 numbers (§2, §4, §5).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::constants::CHECKED_IN_JSON;
use super::*;
use crate::spell::formula::{FormulaInputs, Input};
use oteryn_simulation_determinism::SemanticTimeMicros;

fn table() -> &'static AttackConstants {
    AttackConstants::checked_in().expect("the checked-in constants table parses")
}

fn ms(value: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(value * 1_000)
}

fn attack_inputs(
    level: u32,
    attack_skill: u32,
    attack_value: u32,
    mode: FightMode,
) -> FormulaInputs {
    FormulaInputs {
        level,
        magic_level: 0,
        base_power: None,
        attack_skill,
        attack_value,
        attack_factor: table().fight_mode(mode).attack_factor,
        shielding_skill: 0,
        shield_defense: None,
    }
}

fn defence_inputs(skill: u32, value: u32) -> FormulaInputs {
    FormulaInputs {
        level: 1,
        magic_level: 0,
        base_power: None,
        attack_skill: 0,
        attack_value: 0,
        attack_factor: 1.0,
        shielding_skill: skill,
        shield_defense: Some(value),
    }
}

fn facts() -> TargetFacts {
    TargetFacts {
        present_and_visible: true,
        is_creature: true,
        alive: true,
        same_floor: true,
        distance: 1,
        attacker_in_protection_zone: false,
        target_in_protection_zone: false,
        attacker_reentry_protected: false,
    }
}

// --- constants ---

#[test]
fn checked_in_table_carries_the_decision_values() {
    let table = table();
    assert_eq!(table.attack_interval_ms, 2_000);
    assert_eq!(table.attack_interval_micros(), 2_000_000);
    assert_eq!(table.in_fight_ms, 60_000);
    assert_eq!(table.starting_skill, 10);
    let factors: Vec<f64> = [
        FightMode::Offensive,
        FightMode::Balanced,
        FightMode::Defensive,
    ]
    .into_iter()
    .map(|mode| table.fight_mode(mode).attack_factor)
    .collect();
    assert_eq!(factors, [1.0, 0.75, 0.5]);
    assert_eq!(
        table
            .fight_mode(FightMode::Offensive)
            .defence_factor_after_swing,
        0.5
    );
    assert_eq!(
        table
            .fight_mode(FightMode::Balanced)
            .defence_factor_after_swing,
        0.75
    );
    assert_eq!(
        table
            .fight_mode(FightMode::Defensive)
            .defence_factor_after_swing,
        1.0
    );
    assert_eq!((table.fist.attack, table.fist.defence), (7, 7));
    assert_eq!(table.player_melee.damage_coefficient, 0.085);
    assert_eq!(table.defence.scaling_shield, 0.16);
    assert_eq!(table.defence.scaling_weapon, 0.146);
    assert_eq!(table.defence.scaling_fist, 0.15);
    assert_eq!(table.block.refill_ms, 1_000);
    assert_eq!(table.block.max_blocks, 2);
    assert_eq!(FightMode::default(), FightMode::Balanced);
}

#[test]
fn table_parse_rejects_bad_tables() {
    let base: serde_json::Value = serde_json::from_str(CHECKED_IN_JSON).unwrap();
    let mutate = |pointer: &str, value: serde_json::Value| {
        let mut copy = base.clone();
        *copy.pointer_mut(pointer).unwrap() = value;
        AttackConstants::parse(&copy.to_string())
    };
    assert!(AttackConstants::parse(&base.to_string()).is_some());
    assert!(mutate("/schema", "OTERYN_ATTACK_CONSTANTS/v2".into()).is_none());
    assert!(mutate("/parity", "PARITY_MATCHED".into()).is_none());
    assert!(mutate("/attack_interval_ms", 0.into()).is_none());
    assert!(mutate("/in_fight_ms", 0.into()).is_none());
    assert!(mutate("/block/refill_ms", 0.into()).is_none());
    assert!(mutate("/block/initial_blocks", 3.into()).is_none());
    assert!(mutate("/fight_modes/balanced/attack_factor", 1.5.into()).is_none());
    assert!(
        mutate(
            "/fight_modes/offensive/defence_factor_after_swing",
            0.into()
        )
        .is_none()
    );
    assert!(mutate("/defence/scaling_fist", (-0.15).into()).is_none());
    assert!(mutate("/defence/skill_divisor", 0.into()).is_none());
    assert!(mutate("/creature_melee/attack_coefficient", 0.into()).is_none());
    let mut extra = base.clone();
    extra["unknown"] = 1.into();
    assert!(AttackConstants::parse(&extra.to_string()).is_none());
}

// --- formulas ---

#[test]
fn fist_damage_matches_canary_per_fight_mode() {
    let fist = player_fist_formula(table());
    assert!(!fist.reads(Input::AttackValue));
    let cases = [
        (8, 10, FightMode::Offensive, 7),
        (8, 10, FightMode::Balanced, 5),
        (8, 10, FightMode::Defensive, 4),
        (100, 50, FightMode::Offensive, 50),
    ];
    for (level, skill, mode, maximum) in cases {
        // The attack value input is ignored: fists always hit with attack 7.
        let inputs = attack_inputs(level, skill, 999, mode);
        assert_eq!(
            fist.bounds(&inputs),
            Ok((0, maximum)),
            "{level} {skill} {mode:?}"
        );
    }
}

#[test]
fn melee_damage_matches_canary() {
    let melee = player_melee_formula(table());
    assert!(melee.reads(Input::AttackValue));
    assert_eq!(
        melee.bounds(&attack_inputs(100, 100, 50, FightMode::Offensive)),
        Ok((0, 445))
    );
    assert_eq!(
        melee.bounds(&attack_inputs(300, 120, 45, FightMode::Balanced)),
        Ok((0, 404))
    );
}

#[test]
fn melee_level_term_follows_the_official_curve() {
    let fist = player_fist_formula(table());
    // Up to level 500 the curve equals Canary's level / 5.
    assert_eq!(
        fist.bounds(&attack_inputs(500, 10, 0, FightMode::Offensive)),
        Ok((0, 106))
    );
    // Above it the official curve (116 at 600) replaces level / 5 (120).
    assert_eq!(
        fist.bounds(&attack_inputs(600, 10, 0, FightMode::Offensive)),
        Ok((0, 122))
    );
}

#[test]
fn creature_melee_matches_get_max_melee_damage() {
    let creature = creature_melee_formula(table());
    for (skill, attack, maximum) in [(10, 5, 5), (30, 40, 80), (70, 100, 400), (50, 25, 75)] {
        let inputs = attack_inputs(1, skill, attack, FightMode::Offensive);
        assert_eq!(
            creature.bounds(&inputs),
            Ok((0, maximum)),
            "{skill} {attack}"
        );
    }
    assert!(!creature.reads(Input::Level));
    assert!(!creature.reads(Input::AttackFactor));
}

#[test]
fn defence_matches_canary_per_source() {
    let table = table();
    let fist = defence_formula(
        table,
        DefenceSource::Fist,
        defence_mode_factor(table, FightMode::Balanced, true),
        1.0,
    );
    assert_eq!(fist.bounds(&defence_inputs(10, 7)), Ok((1, 3)));
    let shield = defence_formula(
        table,
        DefenceSource::Shield,
        defence_mode_factor(table, FightMode::Defensive, true),
        1.0,
    );
    assert_eq!(shield.bounds(&defence_inputs(50, 20)), Ok((23, 47)));
    let weapon = defence_formula(
        table,
        DefenceSource::Weapon,
        defence_mode_factor(table, FightMode::Offensive, true),
        1.0,
    );
    assert_eq!(weapon.bounds(&defence_inputs(100, 30)), Ok((29, 59)));
    let creature = defence_formula(table, DefenceSource::Creature, 1.0, 1.0);
    assert_eq!(creature.bounds(&defence_inputs(30, 25)), Ok((18, 36)));
}

#[test]
fn defence_mode_factor_applies_only_after_a_recent_swing() {
    let table = table();
    for mode in [
        FightMode::Offensive,
        FightMode::Balanced,
        FightMode::Defensive,
    ] {
        assert_eq!(defence_mode_factor(table, mode, false), 1.0);
    }
    assert_eq!(defence_mode_factor(table, FightMode::Offensive, true), 0.5);
    assert_eq!(defence_mode_factor(table, FightMode::Balanced, true), 0.75);
    assert_eq!(defence_mode_factor(table, FightMode::Defensive, true), 1.0);
}

#[test]
fn zero_skill_defence_is_one_or_two() {
    let table = table();
    assert_eq!(
        zero_skill_defence_bounds(table, FightMode::Offensive),
        (0, 1)
    );
    assert_eq!(
        zero_skill_defence_bounds(table, FightMode::Balanced),
        (0, 1)
    );
    assert_eq!(
        zero_skill_defence_bounds(table, FightMode::Defensive),
        (1, 2)
    );
}

#[test]
fn armor_reduction_matches_canary() {
    let table = table();
    assert_eq!(armor_reduction_bounds(table, 0), (0, 0));
    for armor in 1..=3 {
        assert_eq!(armor_reduction_bounds(table, armor), (1, 1));
    }
    assert_eq!(armor_reduction_bounds(table, 4), (2, 3));
    assert_eq!(armor_reduction_bounds(table, 5), (2, 3));
    assert_eq!(armor_reduction_bounds(table, 10), (5, 9));
    assert_eq!(armor_reduction_bounds(table, 11), (5, 9));
}

// --- target validity ---

#[test]
fn admission_refuses_the_section_3_targets() {
    assert_eq!(facts().admit(), Ok(()));
    let out_of_range = TargetFacts {
        distance: 5,
        same_floor: false,
        ..facts()
    };
    assert_eq!(
        out_of_range.admit(),
        Ok(()),
        "range is a swing wait, not a refusal"
    );
    let cases = [
        (
            TargetFacts {
                present_and_visible: false,
                ..facts()
            },
            TargetRefusal::NotVisible,
        ),
        (
            TargetFacts {
                is_creature: false,
                ..facts()
            },
            TargetRefusal::NotACreature,
        ),
        (
            TargetFacts {
                alive: false,
                ..facts()
            },
            TargetRefusal::Dead,
        ),
        (
            TargetFacts {
                target_in_protection_zone: true,
                ..facts()
            },
            TargetRefusal::TargetInProtectionZone,
        ),
        (
            TargetFacts {
                attacker_in_protection_zone: true,
                ..facts()
            },
            TargetRefusal::AttackerInProtectionZone,
        ),
        (
            TargetFacts {
                attacker_reentry_protected: true,
                ..facts()
            },
            TargetRefusal::ReentryProtected,
        ),
    ];
    for (facts, refusal) in cases {
        assert_eq!(facts.admit(), Err(refusal));
        let mut state = AttackState::<u32, u8>::default();
        assert_eq!(state.set_target(1, 0, &facts), Err(refusal));
        assert_eq!(state.target(), None);
    }
}

#[test]
fn validity_waits_or_clears_per_section_4() {
    assert_eq!(facts().validity(), TargetValidity::Valid);
    let same_tile = TargetFacts {
        distance: 0,
        ..facts()
    };
    assert_eq!(same_tile.validity(), TargetValidity::Valid);
    let cases = [
        (
            TargetFacts {
                distance: 2,
                ..facts()
            },
            TargetValidity::Wait(SwingWait::OutOfRange),
        ),
        (
            TargetFacts {
                same_floor: false,
                ..facts()
            },
            TargetValidity::Wait(SwingWait::OtherFloor),
        ),
        (
            TargetFacts {
                attacker_in_protection_zone: true,
                ..facts()
            },
            TargetValidity::Wait(SwingWait::AttackerInProtectionZone),
        ),
        (
            TargetFacts {
                target_in_protection_zone: true,
                ..facts()
            },
            TargetValidity::Wait(SwingWait::TargetInProtectionZone),
        ),
        (
            TargetFacts {
                attacker_reentry_protected: true,
                ..facts()
            },
            TargetValidity::Wait(SwingWait::ReentryProtected),
        ),
        (
            TargetFacts {
                present_and_visible: false,
                ..facts()
            },
            TargetValidity::Clear(TargetCleared::Vanished),
        ),
        (
            TargetFacts {
                alive: false,
                ..facts()
            },
            TargetValidity::Clear(TargetCleared::Dead),
        ),
        (
            TargetFacts {
                is_creature: false,
                ..facts()
            },
            TargetValidity::Clear(TargetCleared::NotACreature),
        ),
    ];
    for (facts, validity) in cases {
        assert_eq!(facts.validity(), validity);
    }
}

// --- auto-attack timer ---

fn swing_at(poll: SwingPoll<u32, u8>) -> (u64, u32, u8, u64) {
    match poll {
        SwingPoll::Swing(swing) => (
            swing.at.get() / 1_000,
            swing.target,
            swing.lineage,
            swing.sequence,
        ),
        other => panic!("expected a swing, got {other:?}"),
    }
}

#[test]
fn swings_every_two_seconds_with_a_stable_sequence() {
    let interval = table().attack_interval_micros();
    let mut state = AttackState::<u32, u8>::default();
    assert_eq!(
        state.poll_swing(ms(0), interval, &facts()),
        SwingPoll::NoTarget
    );
    state.set_target(7, 3, &facts()).unwrap();
    assert_eq!(
        swing_at(state.poll_swing(ms(100), interval, &facts())),
        (100, 7, 3, 0)
    );
    assert_eq!(
        state.poll_swing(ms(2_099), interval, &facts()),
        SwingPoll::NotDue { due: ms(2_100) }
    );
    assert_eq!(
        swing_at(state.poll_swing(ms(2_100), interval, &facts())),
        (2_100, 7, 3, 1)
    );
    assert_eq!(
        swing_at(state.poll_swing(ms(4_100), interval, &facts())),
        (4_100, 7, 3, 2)
    );
}

#[test]
fn a_stall_yields_one_swing_and_the_next_interval_counts_from_execution() {
    let interval = table().attack_interval_micros();
    let mut state = AttackState::<u32, u8>::default();
    state.set_target(7, 0, &facts()).unwrap();
    assert_eq!(swing_at(state.poll_swing(ms(0), interval, &facts())).3, 0);
    // Five intervals missed: exactly one swing, no backlog.
    assert_eq!(
        swing_at(state.poll_swing(ms(10_500), interval, &facts())),
        (10_500, 7, 0, 1)
    );
    assert_eq!(
        state.poll_swing(ms(10_500), interval, &facts()),
        SwingPoll::NotDue { due: ms(12_500) }
    );
    assert_eq!(
        state.poll_swing(ms(12_000), interval, &facts()),
        SwingPoll::NotDue { due: ms(12_500) }
    );
    assert_eq!(
        swing_at(state.poll_swing(ms(12_500), interval, &facts())).3,
        2
    );
}

#[test]
fn changing_target_keeps_the_interval() {
    let interval = table().attack_interval_micros();
    let mut state = AttackState::<u32, u8>::default();
    state.set_target(1, 0, &facts()).unwrap();
    swing_at(state.poll_swing(ms(0), interval, &facts()));
    state.set_target(2, 1, &facts()).unwrap();
    assert_eq!(
        state.poll_swing(ms(500), interval, &facts()),
        SwingPoll::NotDue { due: ms(2_000) }
    );
    state.clear_target();
    state.set_target(3, 2, &facts()).unwrap();
    assert_eq!(
        swing_at(state.poll_swing(ms(2_000), interval, &facts())),
        (2_000, 3, 2, 1)
    );
}

#[test]
fn waiting_target_is_held_and_cleared_target_is_dropped() {
    let interval = table().attack_interval_micros();
    let mut state = AttackState::<u32, u8>::default();
    state.set_target(9, 0, &facts()).unwrap();
    let far = TargetFacts {
        distance: 3,
        ..facts()
    };
    assert_eq!(
        state.poll_swing(ms(0), interval, &far),
        SwingPoll::Waiting(SwingWait::OutOfRange)
    );
    assert_eq!(state.target(), Some(9));
    // Back in range: the swing happens at once, no interval was spent while waiting.
    assert_eq!(
        swing_at(state.poll_swing(ms(700), interval, &facts())).0,
        700
    );
    let dead = TargetFacts {
        alive: false,
        ..facts()
    };
    assert_eq!(
        state.poll_swing(ms(2_700), interval, &dead),
        SwingPoll::Cleared(TargetCleared::Dead)
    );
    assert_eq!(state.target(), None);
    assert_eq!(
        state.poll_swing(ms(2_700), interval, &facts()),
        SwingPoll::NoTarget
    );
}

#[test]
fn swung_within_interval_tracks_the_last_swing() {
    let interval = table().attack_interval_micros();
    let mut state = AttackState::<u32, u8>::default();
    assert!(!state.swung_within_interval(ms(0), interval));
    state.set_target(1, 0, &facts()).unwrap();
    swing_at(state.poll_swing(ms(1_000), interval, &facts()));
    assert!(state.swung_within_interval(ms(2_999), interval));
    assert!(!state.swung_within_interval(ms(3_000), interval));
}

// --- in-fight deadline ---

#[test]
fn in_fight_deadline_runs_sixty_seconds_after_the_last_hit() {
    let in_fight = table().in_fight_micros();
    let mut state = AttackState::<u32, u8>::default();
    assert!(!state.in_fight(ms(0)));
    state.record_hit(ms(1_000), in_fight);
    assert_eq!(state.in_fight_until(), Some(ms(61_000)));
    assert!(state.in_fight(ms(60_999)));
    assert!(!state.in_fight(ms(61_000)));
    // A later hit refreshes it; an earlier timestamp never shortens it.
    state.record_hit(ms(30_000), in_fight);
    assert_eq!(state.in_fight_until(), Some(ms(90_000)));
    state.record_hit(ms(5_000), in_fight);
    assert_eq!(state.in_fight_until(), Some(ms(90_000)));
}

#[test]
fn clearing_the_target_keeps_the_in_fight_deadline() {
    let mut state = AttackState::<u32, u8>::default();
    state.set_target(1, 0, &facts()).unwrap();
    state.record_hit(ms(0), table().in_fight_micros());
    state.clear_target();
    assert_eq!(state.target(), None);
    assert!(state.in_fight(ms(59_999)));
}

// --- block budget ---

#[test]
fn block_budget_refills_one_per_second_up_to_two() {
    let block = table().block;
    let mut budget = BlockBudget::new(&block, ms(0));
    assert_eq!(budget.blocks(), 0);
    assert!(!budget.try_block(&block, ms(999)));
    assert!(budget.try_block(&block, ms(1_000)));
    assert!(!budget.try_block(&block, ms(1_500)));
    // 10 s idle: capped at two.
    budget.advance_to(&block, ms(11_000));
    assert_eq!(budget.blocks(), 2);
    assert!(budget.try_block(&block, ms(11_000)));
    assert!(budget.try_block(&block, ms(11_000)));
    assert!(
        !budget.try_block(&block, ms(11_000)),
        "a third hit meets armor only"
    );
    assert!(budget.try_block(&block, ms(12_000)));
}

#[test]
fn block_budget_keeps_partial_progress_and_ignores_time_regression() {
    let block = table().block;
    let mut budget = BlockBudget::new(&block, ms(0));
    budget.advance_to(&block, ms(600));
    budget.advance_to(&block, ms(1_200));
    assert_eq!(budget.blocks(), 1);
    budget.advance_to(&block, ms(500));
    assert_eq!(budget.blocks(), 1);
    budget.advance_to(&block, ms(2_000));
    assert_eq!(budget.blocks(), 2);
}
