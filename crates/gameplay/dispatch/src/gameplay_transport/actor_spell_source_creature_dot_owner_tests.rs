#![allow(clippy::expect_used, clippy::panic)]
use super::*;
use crate::spell::combat_batch::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use {
    crate::foundation::{
        ApplicationFacts, CharacterId, CommandId, CommandRef, ConditionDefinition,
        ConditionSourceKind, ConditionValues, DotElement, MovementLocalPosition,
    },
    oteryn_gameplay_content::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags,
        CreatureResistance,
    },
};
fn fixture() -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    ExactActorRef,
    GameSessionId,
) {
    fixture_with_phase(true)
}
fn fixture_with_phase(
    attackable: bool,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    ExactActorRef,
    GameSessionId,
) {
    let (mut runtime, source, session) = super::tests::runtime_with_player(0x41);
    runtime
        .initialize_first_entry_position(source)
        .expect("actual pinned player");
    let policy = CompiledCreaturePolicy {
        definition_key: "creature:source-healing-target".into(),
        definition_revision: "test-source-1".into(),
        display_name: "source healing target".into(),
        maximum_health: 20,
        base_speed: 100,
        outfit_look_type: 1,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: vec![],
        defense: Some(1),
        armor: Some(0),
        mitigation: Some(CreatureExactRatio {
            numerator: 0,
            denominator: 1,
        }),
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![CreatureResistance {
            damage_type: "fire".into(),
            percent: CreatureExactRatio {
                numerator: 100,
                denominator: 1,
            },
        }],
        flags: CreatureFlags {
            attackable,
            illusionable: false,
            health_hidden: false,
        },
        preferred_distance: Some(1),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
    };
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                vec![policy],
            )
            .expect("actual loader policy"),
        )
        .expect("fullpin");
    let target = runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        })
        .expect("physical actor under actual pin");
    runtime
        .install_creature_policy(target, "creature:source-healing-target")
        .expect("current native decoded profile");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            source,
            session,
            super::tests::FACTS,
            (0, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("actual canonical PlayerSpellState");
    let root = GameplayDecisionRoot::from_bytes([8; 32]);
    // TEST_PARAMETERS: Attributes is a native Player combat modifier.
    // Source-player lowering intentionally does not accept this native kind.
    let definition = oteryn_gameplay_foundation::foundation::condition::ConditionDefinition::new(
        "test.source.attacker.buff",
        1,
        oteryn_gameplay_domain::domain::condition::ConditionValues::Attributes {
            duration_ms: 5000,
            critical_chance_percent: 0,
            critical_extra_percentage_points: 0,
            damage_dealt_percent: 50,
            incoming_reduction_percent: 0,
        },
    )
    .expect("typed native Player combat buff");
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 100,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
    };
    let native_facts = oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
        now: facts.now,
        base_speed: facts.base_speed,
        mana_shield_capacity: facts.mana_shield_capacity,
        target_reentry_protected: facts.target_reentry_protected,
        source_reentry_protected: facts.source_reentry_protected,
        target_is_player: true,
        decision_root: &root,
        occurrence: facts.occurrence,
    };
    let next = states
        .get(&runtime, source, session)
        .expect("sole canonical owner")
        .stage_owned_test_conditions(&[definition], &native_facts)
        .expect("actual native Player buff owner");
    assert!(states.commit(&runtime, source, session, next));
    let wound = OwnerCombatBatch {
        caster: source,
        attacker: CharacterId::decode(&[1, 0x90, 0, 0, 0, 5, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 5])
            .expect("test character"),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(1).expect("command")),
        occurrence: SpellOccurrenceBinding {
            id: "source-dot-wound".into(),
            revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
        },
        binding: b"actual-source-wound".to_vec(),
        anchor: None,
        now_us: None,
        now_ms: 0,
        effects: vec![OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: runtime
                    .creature_spell_target_atom(target)
                    .expect("actual target identity"),
                magnitude: 15,
            },
        }],
        deferred: None,
    };
    let staged = runtime
        .stage_spell_batch(&wound)
        .expect("native wound stage");
    runtime.commit_spell_batch(staged).expect("native HP");
    let before = runtime.companion_snapshot(target).expect("actual target");
    let mut next = before.state.clone();
    let dot = ConditionDefinition::new(
        "test.source.player.dot",
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Fire,
            total_min: 30,
            total_max: 30,
            per_tick: 10,
            interval_ms: 1000,
            delayed: true,
        },
    )
    .expect("typed source DOT");
    let facts = ApplicationFacts {
        target_is_player: false,
        occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        ..facts
    };
    next.conditions
        .apply(&dot, Some(source), ConditionSourceKind::Player, &[], &facts)
        .expect("actual source in native Creature store");
    runtime
        .compare_companion_state(&before, next)
        .expect("current source install");
    (runtime, states, source, target, session)
}
#[test]
fn current_player_buff_comes_from_canonical_owner_dot_heals_before_scaled_damage_and_replays() {
    let (mut runtime, states, source, target, session) = fixture();
    assert!(
        runtime
            .actor_conditions(source, Some(session))
            .expect("native player compatibility store")
            .instances()
            .is_empty()
    );
    let receipt = states
        .tick_source_creature_conditions(
            &mut runtime,
            target,
            crate::foundation::TickFacts::default(),
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
        )
        .expect("actual owned periodic handler");
    assert_eq!(
        (receipt.health_before, receipt.health_after),
        (5, 10),
        "map10 before50% source buff then damage5"
    );
    let retry = states
        .tick_source_creature_conditions(
            &mut runtime,
            target,
            crate::foundation::TickFacts::default(),
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
        )
        .expect("same turn retry");
    assert!(retry.ticks.is_empty());
    assert_eq!(retry.health_after, 10);
}
#[test]
fn retired_exact_attacker_cannot_heal_and_pz_tick_uses_real_owner_store_without_hp() {
    let (mut runtime, states, source, target, _) = fixture();
    runtime
        .remove_test_actor(source)
        .expect("retire source generation");
    let first = states
        .tick_source_creature_conditions(
            &mut runtime,
            target,
            crate::foundation::TickFacts {
                in_protection_zone: true,
                standing_on_field: None,
            },
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
        )
        .expect("qualified PZ suppression");
    assert_eq!((first.health_before, first.health_after), (5, 5));
    let second = states
        .tick_source_creature_conditions(
            &mut runtime,
            target,
            crate::foundation::TickFacts::default(),
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("retired attacker resolved absent");
    assert_eq!(
        (second.health_before, second.health_after),
        (5, 0),
        "no fake attacker buff/healing; residual damage remains"
    );
}
#[test]
fn stale_lineage_content_pin_refuses_before_hp_and_condition_cursor_publication() {
    let (mut runtime, states, source, target, session) = fixture();
    let before = runtime.companion_snapshot(target).expect("current target");
    let mut next = before.state.clone();
    next.conditions
        .remove_type(crate::foundation::ConditionType::DamageOverTime(
            DotElement::Fire,
        ));
    let root = GameplayDecisionRoot::from_bytes([12; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 100,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: false,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([3; 16]),
    };
    let definition = ConditionDefinition::new(
        "test.source.pin-negative",
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Fire,
            total_min: 10,
            total_max: 10,
            per_tick: 10,
            interval_ms: 1000,
            delayed: true,
        },
    )
    .expect("typed source recipe");
    let lineage = crate::foundation::condition::ConditionLineage {
        source_character: [3; 16],
        source_session: *session.as_bytes(),
        source_lease_generation: 1,
        source_actor_placement: source.placement_identity(),
        source_scope_generation: source.scope_generation().get(),
        source_content_digest: [9; 32],
        source_catalog_digest: [8; 32],
        source_creation_command: 7,
    };
    next.conditions
        .apply_with_lineage(
            &definition,
            Some(source),
            ConditionSourceKind::Player,
            &[],
            &facts,
            &lineage,
        )
        .expect("retained historical source receipt");
    runtime
        .compare_companion_state(&before, next)
        .expect("real owner source store");
    let before = runtime.companion_snapshot(target).expect("full preimage");
    assert!(
        states
            .tick_source_creature_conditions(
                &mut runtime,
                target,
                crate::foundation::TickFacts::default(),
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000)
            )
            .is_err()
    );
    assert_eq!(
        runtime
            .companion_snapshot(target)
            .expect("untouched HP/store"),
        before
    );
}

#[test]
fn current_nonattackable_phase_does_not_trigger_pre_immunity_healing() {
    let (mut runtime, states, _, target, _) = fixture_with_phase(false);
    let receipt = states
        .tick_source_creature_conditions(
            &mut runtime,
            target,
            crate::foundation::TickFacts::default(),
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
        )
        .expect("actual nonattackable phase");
    assert_eq!(
        (receipt.health_before, receipt.health_after),
        (5, 5),
        "phaseattackability precedes healingMap; damageimmunity differs"
    );
}
