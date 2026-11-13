#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixture construction failures are assertions"
)]

use super::*;
use crate::foundation::{ConditionSourceKind, ConditionValues};
use crate::gameplay_transport::actor_spell::tests;
use oteryn_gameplay_domain::domain::condition::ConflictKey;
use oteryn_gameplay_progression::charm_effects::CharmEffectOutcome;

fn facts(root: &GameplayDecisionRoot, now: u64, ordinal: u8) -> ApplicationFacts<'_> {
    ApplicationFacts {
        now,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: root,
        occurrence: DecisionOccurrenceId::from_bytes([ordinal; 16]),
    }
}

fn canonical_hex_case() {
    let (mut runtime, player, session) = tests::runtime_with_player(83);
    runtime.initialize_first_entry_position(player).unwrap();
    let position = runtime.read_actor_position(player).unwrap().position();
    let source = runtime
        .admit_source_pinned_lab_creature(
            crate::foundation::MovementLocalPosition {
                x: position.x + 1,
                y: position.y,
                floor: position.floor,
            },
            "test:creature",
            20,
        )
        .unwrap();
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            player,
            session,
            tests::FACTS,
            (0, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .unwrap();
    // The Soulpit Oppressor applies this exact family and values to the actor slot.
    let hex = ConditionDefinition::new(
        "oteryn:condition.soulpit.intense_hex",
        1,
        ConditionValues::IntenseHex {
            duration_ms: 3_000,
            damage_dealt_percent: 50,
            healing_received_percent: 50,
        },
    )
    .unwrap();
    let root = GameplayDecisionRoot::from_bytes([83; 32]);
    let apply_hex = |runtime: &mut ChannelRuntimeV1, now: u64, ordinal: u8| {
        let plan = runtime.prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::Apply {
                definition: &hex,
                source: ConditionSource {
                    actor: source,
                    session: None,
                    kind: ConditionSourceKind::Creature,
                },
                immunities: &[],
                facts: facts(&root, now, ordinal),
            },
            SemanticTimeMicros::from_micros(now),
        );
        match plan {
            Ok(plan) => runtime
                .commit_actor_condition(&plan, SemanticTimeMicros::from_micros(now))
                .map(|_| ()),
            Err(error) => Err(error),
        }
    };
    apply_hex(&mut runtime, 0, 1).unwrap();
    let wrong_session = tests::runtime_with_player(84).2;
    let unchanged = runtime
        .actor_conditions(player, Some(session))
        .unwrap()
        .clone();
    assert!(!states.apply_player_charm_condition(
        &mut runtime,
        player,
        wrong_session,
        CharmEffectOutcome::CleanseOneNegativeCondition,
        &root,
        DecisionOccurrenceId::from_bytes([2; 16]),
        1_000,
    ));
    assert_eq!(
        runtime.actor_conditions(player, Some(session)).unwrap(),
        &unchanged
    );
    assert!(states.apply_player_charm_condition(
        &mut runtime,
        player,
        session,
        CharmEffectOutcome::CleanseOneNegativeCondition,
        &root,
        DecisionOccurrenceId::from_bytes([2; 16]),
        1_000
    ));
    let store = runtime.actor_conditions(player, Some(session)).unwrap();
    assert!(store.get(ConflictKey::IntenseHex).is_none());
    assert_eq!(
        store.cleanse_immunity_remaining(ConflictKey::IntenseHex, 1_000),
        11_000_000
    );
    assert!(!states.apply_player_charm_condition(
        &mut runtime,
        player,
        session,
        CharmEffectOutcome::CleanseOneNegativeCondition,
        &root,
        DecisionOccurrenceId::from_bytes([2; 16]),
        1_001
    ));
    assert_eq!(
        apply_hex(&mut runtime, 11_000_999, 3),
        Err(ConditionOwnerError::Condition(
            oteryn_gameplay_domain::domain::condition::ConditionRefusal::Immune
        ))
    );
    apply_hex(&mut runtime, 11_001_000, 4).unwrap();
    assert!(states.apply_player_charm_condition(
        &mut runtime,
        player,
        session,
        CharmEffectOutcome::CleanseOneNegativeCondition,
        &root,
        DecisionOccurrenceId::from_bytes([5; 16]),
        11_001_000
    ));
    assert!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .get(ConflictKey::IntenseHex)
            .is_none()
    );
}

#[test]
fn charm_cleanse_reaches_canonical_boss_hex_and_retains_immunity_and_replay_fence() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(canonical_hex_case)
        .unwrap()
        .join()
        .unwrap();
}

fn mixed_store_case() {
    let mut selected = [false; 2];
    for tag in 1..=16_u8 {
        let (mut runtime, player, session) = tests::runtime_with_player(tag);
        runtime.initialize_first_entry_position(player).unwrap();
        let position = runtime.read_actor_position(player).unwrap().position();
        let source = runtime
            .admit_source_pinned_lab_creature(
                crate::foundation::MovementLocalPosition {
                    x: position.x + 1,
                    y: position.y,
                    floor: position.floor,
                },
                "test:creature",
                20,
            )
            .unwrap();
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                player,
                session,
                tests::FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        let root = GameplayDecisionRoot::from_bytes([tag; 32]);
        let hex = ConditionDefinition::new(
            "oteryn:condition.soulpit.intense_hex",
            1,
            ConditionValues::IntenseHex {
                duration_ms: 3_000,
                damage_dealt_percent: 50,
                healing_received_percent: 50,
            },
        )
        .unwrap();
        let apply = runtime
            .prepare_actor_condition(
                player,
                Some(session),
                ActorConditionTransition::Apply {
                    definition: &hex,
                    source: ConditionSource {
                        actor: source,
                        session: None,
                        kind: ConditionSourceKind::Creature,
                    },
                    immunities: &[],
                    facts: facts(&root, 0, 1),
                },
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        runtime
            .commit_actor_condition(&apply, SemanticTimeMicros::from_micros(0))
            .unwrap();
        let poison = ConditionDefinition::new(
            "test.poison",
            1,
            ConditionValues::DamageOverTime {
                element: oteryn_gameplay_domain::domain::condition::DotElement::Poison,
                total_min: 20,
                total_max: 20,
                per_tick: 10,
                interval_ms: 1_000,
                delayed: true,
            },
        )
        .unwrap();
        let before = states
            .get(&runtime, player, session)
            .unwrap()
            .owned_conditions()
            .clone();
        let mut after = before.clone();
        after
            .apply(
                &poison,
                Some("test:creature".into()),
                ConditionSourceKind::Creature,
                &[],
                &facts(&root, 0, 1),
            )
            .unwrap();
        states
            .get_mut(&runtime, player, session)
            .unwrap()
            .apply_batch_conditions(&before, &after)
            .unwrap();
        let occurrence = DecisionOccurrenceId::from_bytes([2; 16]);
        let index = usize::try_from(
            oteryn_simulation_determinism::deterministic_decision_u64(
                &root,
                occurrence,
                oteryn_gameplay_foundation::foundation::condition::COND_CLEANSE_PICK,
                0,
            )
            .unwrap()
                % 2,
        )
        .unwrap();
        selected[index] = true;
        assert!(states.apply_player_charm_condition(
            &mut runtime,
            player,
            session,
            CharmEffectOutcome::CleanseOneNegativeCondition,
            &root,
            occurrence,
            1_000
        ));
        let canonical = runtime.actor_conditions(player, Some(session)).unwrap();
        let legacy = states
            .get(&runtime, player, session)
            .unwrap()
            .owned_conditions();
        assert_eq!(canonical.instances().len() + legacy.instances().len(), 1);
        assert_eq!(canonical.get(ConflictKey::IntenseHex).is_none(), index == 0);
        let native_before = canonical.clone();
        let legacy_before = legacy.clone();
        assert!(!states.apply_player_charm_condition(
            &mut runtime,
            player,
            session,
            CharmEffectOutcome::CleanseOneNegativeCondition,
            &root,
            occurrence,
            1_001
        ));
        assert_eq!(
            runtime.actor_conditions(player, Some(session)).unwrap(),
            &native_before
        );
        assert_eq!(
            states
                .get(&runtime, player, session)
                .unwrap()
                .owned_conditions(),
            &legacy_before
        );
    }
    assert_eq!(selected, [true, true]);
}

#[test]
fn charm_cleanse_single_named_draw_selects_one_across_both_stores_and_replay_cannot_fall_through() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(mixed_store_case)
        .unwrap()
        .join()
        .unwrap();
}
