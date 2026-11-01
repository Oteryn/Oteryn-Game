//! Crystal 201189f condition.cpp:552-580,827-889 saves each condition's signed
//! application delta and removes that same delta on refresh or expiry.

use super::{
    ApplicationFacts, ConditionBuffDelta, ConditionDefinition, ConditionRefusal,
    ConditionSourceKind, ConditionStore, ConditionValues, TickFacts,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

const SECOND: u64 = 1_000_000;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn facts(now: u64, root: &GameplayDecisionRoot) -> ApplicationFacts<'_> {
    ApplicationFacts {
        now,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: root,
        occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
    }
}

fn attributes(percent: u32, duration_ms: u32) -> TestResult<ConditionDefinition> {
    ConditionDefinition::new(
        "soulpit.test.attributes",
        1,
        ConditionValues::Attributes {
            duration_ms,
            critical_chance_percent: 30,
            critical_extra_percentage_points: 25,
            damage_dealt_percent: percent,
            incoming_reduction_percent: 15,
        },
    )
    .ok_or_else(|| "valid source condition definition expected".into())
}

fn hex(duration_ms: u32) -> TestResult<ConditionDefinition> {
    ConditionDefinition::new(
        "soulpit.test.intense_hex",
        1,
        ConditionValues::IntenseHex {
            duration_ms,
            damage_dealt_percent: 50,
            healing_received_percent: 50,
        },
    )
    .ok_or_else(|| "valid source condition definition expected".into())
}

fn apply(
    store: &mut ConditionStore<u32>,
    definition: &ConditionDefinition,
    now: u64,
    root: &GameplayDecisionRoot,
) -> TestResult {
    store
        .apply(
            definition,
            Some(7),
            ConditionSourceKind::SelfUse,
            &[],
            &facts(now, root),
        )
        .map_err(|error| format!("{error:?}"))?;
    Ok(())
}

#[test]
fn soulpit_hex_keeps_its_saved_delta_after_swift_foot_expires() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut store = ConditionStore::new();
    // Swift Foot is 70% for 10 seconds; Hex is 50% for 3 seconds.
    apply(&mut store, &attributes(70, 10_000)?, 0, &root)?;
    apply(&mut store, &hex(3_000)?, 9 * SECOND, &root)?;
    assert_eq!(store.attribute_damage_delta_at(9 * SECOND), -30);
    assert_eq!(
        store.intense_hex_delta_at(9 * SECOND),
        ConditionBuffDelta {
            damage_dealt: -35,
            healing_received: -50,
        }
    );
    assert_eq!(store.damage_dealt_percent_at(10 * SECOND - 1, 0), Some(35));
    // Expired Attributes disappear from the view before a cleanup pass. Hex
    // retains -35, so expiry restores 30 points rather than recomputing 50%.
    assert_eq!(store.damage_dealt_percent_at(10 * SECOND, 0), Some(65));
    assert_eq!(store.healing_received_percent_at(10 * SECOND), 50);
    assert!(store.take_due(10 * SECOND, TickFacts::default()).is_empty());
    assert_eq!(store.damage_dealt_percent_at(10 * SECOND, 0), Some(65));
    assert_eq!(store.instances().len(), 1);
    assert_eq!(store.damage_dealt_percent_at(12 * SECOND, 0), Some(100));
    assert_eq!(store.healing_received_percent_at(12 * SECOND), 100);
    Ok(())
}

#[test]
fn soulpit_hex_then_swift_saves_swifts_delta_from_the_already_reduced_buff() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut store = ConditionStore::new();
    apply(&mut store, &hex(3_000)?, 0, &root)?;
    apply(&mut store, &attributes(70, 10_000)?, SECOND, &root)?;
    assert_eq!(store.intense_hex_delta_at(SECOND).damage_dealt, -50);
    assert_eq!(store.attribute_damage_delta_at(SECOND), -15);
    assert_eq!(store.damage_dealt_percent_at(SECOND, 0), Some(35));
    assert_eq!(store.damage_dealt_percent_at(3 * SECOND, 0), Some(85));
    assert_eq!(store.damage_dealt_percent_at(11 * SECOND, 0), Some(100));
    Ok(())
}

#[test]
fn soulpit_hex_truncates_the_negative_delta_and_preserves_other_attributes() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut store = ConditionStore::new();
    apply(&mut store, &attributes(71, 10_000)?, 0, &root)?;
    apply(&mut store, &hex(3_000)?, SECOND, &root)?;
    // Source truncates -35.5 toward zero, then adds -35 to 71.
    assert_eq!(store.damage_dealt_percent_at(SECOND, 0), Some(36));
    assert_eq!(
        store.attributes_at(SECOND),
        Some(ConditionValues::Attributes {
            duration_ms: 10_000,
            critical_chance_percent: 30,
            critical_extra_percentage_points: 25,
            damage_dealt_percent: 36,
            incoming_reduction_percent: 15,
        })
    );
    assert_eq!(store.instances().len(), 2);
    assert_eq!(store.damage_dealt_percent_at(4 * SECOND, 0), Some(71));
    Ok(())
}

#[test]
fn soulpit_hex_refresh_removes_only_its_old_delta_before_recalculating() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut store = ConditionStore::new();
    apply(&mut store, &attributes(70, 10_000)?, 0, &root)?;
    apply(&mut store, &hex(3_000)?, 9 * SECOND, &root)?;
    apply(&mut store, &hex(3_000)?, 9 * SECOND + SECOND / 2, &root)?;
    assert_eq!(store.damage_dealt_percent_at(10 * SECOND - 1, 0), Some(35));
    assert_eq!(store.damage_dealt_percent_at(10 * SECOND, 0), Some(65));
    // Attributes have now expired. Refresh removes -35, observes 100, and
    // saves -50. Keeping or multiplying the old Hex delta gives a wrong result.
    apply(&mut store, &hex(3_000)?, 10 * SECOND + SECOND / 2, &root)?;
    assert_eq!(store.damage_dealt_percent_at(12 * SECOND, 0), Some(50));
    assert_eq!(store.intense_hex_delta_at(12 * SECOND).damage_dealt, -50);
    assert_eq!(
        store.damage_dealt_percent_at(13 * SECOND + SECOND / 2, 0),
        Some(100)
    );
    Ok(())
}

#[test]
fn soulpit_hex_attributes_refresh_and_replacement_retain_independent_history() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut initial = ConditionStore::new();
    apply(&mut initial, &attributes(70, 10_000)?, 0, &root)?;
    apply(&mut initial, &hex(3_000)?, SECOND, &root)?;

    let mut refreshed = initial.clone();
    apply(&mut refreshed, &attributes(70, 10_000)?, 2 * SECOND, &root)?;
    // Remove the old -30 from 35, then trunc(65 * -0.3) is -19.
    assert_eq!(refreshed.attribute_damage_delta_at(2 * SECOND), -19);
    assert_eq!(refreshed.damage_dealt_percent_at(2 * SECOND, 0), Some(46));
    assert_eq!(refreshed.damage_dealt_percent_at(4 * SECOND, 0), Some(81));

    let mut replaced = initial;
    apply(&mut replaced, &attributes(50, 10_000)?, 2 * SECOND, &root)?;
    assert_eq!(replaced.attribute_damage_delta_at(2 * SECOND), -32);
    assert_eq!(replaced.damage_dealt_percent_at(2 * SECOND, 0), Some(33));
    assert_eq!(replaced.damage_dealt_percent_at(4 * SECOND, 0), Some(68));
    Ok(())
}

#[test]
fn soulpit_hex_refresh_after_later_attributes_uses_their_saved_delta() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut store = ConditionStore::new();
    apply(&mut store, &hex(3_000)?, 0, &root)?;
    apply(&mut store, &attributes(70, 2_000)?, SECOND, &root)?;
    apply(&mut store, &hex(3_000)?, 2 * SECOND, &root)?;
    // Remove the old -50 from 35, then trunc(85 * -0.5) is -42.
    assert_eq!(store.intense_hex_delta_at(2 * SECOND).damage_dealt, -42);
    assert_eq!(store.damage_dealt_percent_at(2 * SECOND, 0), Some(43));
    assert_eq!(store.damage_dealt_percent_at(3 * SECOND, 0), Some(58));
    assert_eq!(store.damage_dealt_percent_at(5 * SECOND, 0), Some(100));
    Ok(())
}

#[test]
fn soulpit_hex_combines_actual_split_stores_without_counting_a_mirror_twice() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut player = ConditionStore::new();
    let mut actor = ConditionStore::new();
    apply(&mut player, &attributes(70, 10_000)?, 0, &root)?;
    actor
        .apply_with_external_damage_buff(
            &hex(3_000)?,
            Some(7),
            ConditionSourceKind::SelfUse,
            &[],
            &facts(9 * SECOND, &root),
            player.attribute_damage_delta_at(9 * SECOND),
        )
        .map_err(|error| format!("{error:?}"))?;
    let canonical = actor.intense_hex_delta_at(9 * SECOND);
    assert_eq!(
        player.damage_dealt_percent_with_hex_at(9 * SECOND, canonical),
        Some(35)
    );
    assert_eq!(
        player.healing_received_percent_with_hex_at(9 * SECOND, canonical),
        Some(50)
    );
    let mut mirrored = player.clone();
    apply(&mut mirrored, &hex(3_000)?, 9 * SECOND, &root)?;
    assert_eq!(
        mirrored.damage_dealt_percent_with_hex_at(9 * SECOND, canonical),
        Some(35)
    );
    assert_eq!(
        mirrored.healing_received_percent_with_hex_at(9 * SECOND, canonical),
        Some(50)
    );
    assert_eq!(
        player.damage_dealt_percent_with_hex_at(10 * SECOND, canonical),
        Some(65)
    );
    assert_eq!(
        mirrored.damage_dealt_percent_with_hex_at(10 * SECOND, canonical),
        Some(65)
    );
    Ok(())
}

#[test]
fn soulpit_hex_is_observed_when_swift_is_applied_in_the_other_real_store() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut actor = ConditionStore::new();
    let mut player = ConditionStore::new();
    apply(&mut actor, &hex(3_000)?, 0, &root)?;
    player
        .apply_with_external_damage_buff(
            &attributes(70, 10_000)?,
            Some(7),
            ConditionSourceKind::SelfUse,
            &[],
            &facts(SECOND, &root),
            actor.intense_hex_delta_at(SECOND).damage_dealt,
        )
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(player.attribute_damage_delta_at(SECOND), -15);
    assert_eq!(
        player.damage_dealt_percent_with_hex_at(SECOND, actor.intense_hex_delta_at(SECOND)),
        Some(35)
    );
    assert_eq!(
        player.damage_dealt_percent_with_hex_at(3 * SECOND, actor.intense_hex_delta_at(3 * SECOND)),
        Some(85)
    );
    Ok(())
}

#[test]
fn soulpit_hex_signed_total_is_not_clamped_when_an_enhancement_expires() -> TestResult {
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut store = ConditionStore::new();
    apply(&mut store, &attributes(300, 1_000)?, 0, &root)?;
    apply(&mut store, &hex(3_000)?, 0, &root)?;
    assert_eq!(store.intense_hex_delta_at(0).damage_dealt, -150);
    assert_eq!(store.damage_dealt_percent_at(0, 0), Some(150));
    // Creature::setBuff adds signed deltas without a clamp (creature.hpp:261-270).
    assert_eq!(store.damage_dealt_percent_at(SECOND, 0), Some(-50));
    assert_eq!(store.damage_dealt_percent_at(3 * SECOND, 0), Some(100));
    Ok(())
}

#[test]
fn soulpit_hex_invalid_zero_values_and_shorter_refresh_cannot_change_a_live_slot() -> TestResult {
    for (damage, healing, duration) in [
        (0, 50, 3_000),
        (50, 0, 3_000),
        (101, 50, 3_000),
        (50, 101, 3_000),
        (50, 50, 0),
    ] {
        assert!(
            ConditionDefinition::new(
                "soulpit.test.invalid_hex",
                1,
                ConditionValues::IntenseHex {
                    duration_ms: duration,
                    damage_dealt_percent: damage,
                    healing_received_percent: healing,
                },
            )
            .is_none()
        );
    }
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let mut store = ConditionStore::new();
    apply(&mut store, &attributes(70, 10_000)?, 0, &root)?;
    apply(&mut store, &hex(3_000)?, SECOND, &root)?;
    let before = store.clone();
    assert_eq!(
        store.apply(
            &hex(1_000)?,
            Some(7),
            ConditionSourceKind::SelfUse,
            &[],
            &facts(SECOND + SECOND / 2, &root),
        ),
        Err(ConditionRefusal::KeptCurrent)
    );
    assert_eq!(store, before);
    Ok(())
}
