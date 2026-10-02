#![allow(clippy::expect_used)]

use super::*;
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::foundation::{CharacterId, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player, wound};
use crate::spell::cast::{CastContext, cast, v1_spell_book};
use crate::spell::combat_batch::{OwnerCombatEffect, SpellOccurrenceBinding};
use oteryn_protocol_oteryn::actor_spell::{SpellCastIntent, SpellTarget};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::num::NonZeroU32;

struct Fixture {
    runtime: ChannelRuntimeV1,
    states: ChannelSpellStates,
    caster: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    next: PlayerSpellState,
    batch: OwnerCombatBatch,
}
fn fixture() -> Fixture {
    let (mut runtime, caster, session) = runtime_with_player(31);
    runtime
        .initialize_movement_test_position(
            caster,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        )
        .expect("position");
    let target = runtime
        .admit_test_creature(MovementLocalPosition {
            x: 11,
            y: 10,
            floor: 7,
        })
        .expect("physical target");
    let mut states = ChannelSpellStates::default();
    let now = SemanticTimeMicros::from_micros(100_000);
    states
        .initialize(
            &runtime,
            caster,
            session,
            crate::spell::cast::CharacterCastFacts { level: 20, ..FACTS },
            (0, 0),
            now,
        )
        .expect("Character-owned actor state");
    wound(&mut states, caster, session, 20);
    let before = states
        .get(&runtime, caster, session)
        .expect("current state");
    let occurrence = AbilityOccurrence::new(
        "spell-cast:atomic",
        RevisionSet::new("rules:1", "content:1", "world:1", "formula:1", "sim:1")
            .expect("revisions"),
    )
    .expect("occurrence");
    let atom = crate::spell::combat_execution::actor_atom(caster);
    let next = cast(
        &v1_spell_book().expect("real V1 book"),
        before,
        &SpellCastIntent {
            spell: NonZeroU32::new(1).expect("cure poison"),
            target: SpellTarget::None,
            aim_at_target: false,
        },
        CastContext {
            caster: &atom,
            owner_scope: "channel-owner",
            occurrence: occurrence.clone(),
            now,
            draw: &mut |_, maximum| maximum,
        },
    )
    .expect("actual common payment");
    let batch = OwnerCombatBatch {
        caster,
        attacker: CharacterId::decode(&[1, 0x90, 0, 0, 0, 40, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 40])
            .expect("character"),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(1).expect("command")),
        occurrence: SpellOccurrenceBinding::from(occurrence),
        binding: b"qualified:atomic-owner-integration".to_vec(),
        anchor: Some(
            next.payment_anchor_from(before)
                .expect("actual cost and cooldowns"),
        ),
        now_ms: 100,
        effects: vec![
            OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: "test:creature".into(),
                    magnitude: 3,
                },
            },
            OwnerCombatEffect {
                target: caster,
                sub_ordinal: 1,
                change: OwnerCombatChange::Heal {
                    target_atom: atom,
                    magnitude: 9,
                },
            },
        ],
        deferred: None,
    };
    Fixture {
        runtime,
        states,
        caster,
        target,
        session,
        next,
        batch,
    }
}
fn target_health(f: &Fixture) -> i64 {
    f.runtime
        .creature_combat_facts(f.target, 100)
        .expect("real creature HP")
        .health
}

#[test]
fn actual_creature_and_player_owners_commit_cost_heal_and_damage_once() {
    let mut f = fixture();
    let paid_mana = f.batch.anchor.as_ref().expect("anchor").paid_mana;
    let staged = f
        .runtime
        .stage_spell_batch(&f.batch)
        .expect("physical stage");
    let proof = stage_player_batch(&f.runtime, &f.states, &f.batch, Some(f.next.clone()))
        .expect("real player stage");
    assert!(
        commit_owner_batch(&mut f.runtime, &mut f.states, staged, Some(proof))
            .expect("one owner turn")
            .applied
    );
    let state = f
        .states
        .get(&f.runtime, f.caster, f.session)
        .expect("actual state");
    assert_eq!(
        (state.revision(), state.vitals().health, state.vitals().mana),
        (2, 29, 90 - paid_mana)
    );
    assert_eq!(target_health(&f), 17);
    let before = state.clone();
    let replay = f
        .runtime
        .stage_spell_batch(&f.batch)
        .expect("retained original batch");
    assert!(
        !commit_owner_batch(&mut f.runtime, &mut f.states, replay, None)
            .expect("receipt replay")
            .applied
    );
    assert_eq!(f.states.get(&f.runtime, f.caster, f.session), Some(&before));
    assert_eq!(target_health(&f), 17);
}

#[test]
fn refused_player_atom_never_commits_an_earlier_creature_effect_or_cost() {
    let mut f = fixture();
    if let OwnerCombatChange::Heal { target_atom, .. } = &mut f.batch.effects[1].change {
        *target_atom = "actor:substituted".into();
    }
    let before = f
        .states
        .get(&f.runtime, f.caster, f.session)
        .expect("before")
        .clone();
    let _staged = f
        .runtime
        .stage_spell_batch(&f.batch)
        .expect("physical preparation is read-only");
    assert!(stage_player_batch(&f.runtime, &f.states, &f.batch, Some(f.next.clone())).is_err());
    assert_eq!(f.states.get(&f.runtime, f.caster, f.session), Some(&before));
    assert_eq!(target_health(&f), 20);
}

#[test]
fn changed_actual_player_predecessor_refuses_the_entire_prepared_transaction() {
    let mut f = fixture();
    let staged = f
        .runtime
        .stage_spell_batch(&f.batch)
        .expect("physical stage");
    let proof = stage_player_batch(&f.runtime, &f.states, &f.batch, Some(f.next.clone()))
        .expect("player stage");
    f.states
        .get_mut(&f.runtime, f.caster, f.session)
        .expect("actual actor")
        .advance_batch_revision()
        .expect("independent owner mutation");
    let current = f
        .states
        .get(&f.runtime, f.caster, f.session)
        .expect("current")
        .clone();
    assert!(matches!(
        commit_owner_batch(&mut f.runtime, &mut f.states, staged, Some(proof)),
        Err(Error::SnapshotChanged)
    ));
    assert_eq!(
        f.states.get(&f.runtime, f.caster, f.session),
        Some(&current)
    );
    assert_eq!(target_health(&f), 20);
}

fn source_light_successor(
    f: &Fixture,
) -> (
    crate::ability::condition::ConditionStore<String>,
    crate::ability::condition::ConditionStore<String>,
) {
    use crate::ability::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
    };
    let source: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
    ))
    .expect("source catalogue");
    let entry = source["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .find(|row| row["bundle"]["spell"]["identity"]["key"] == "candidate:spell/light")
        .expect("real Light source");
    let effect: crate::spell::executable_catalog::EffectProfile =
        serde_json::from_value(entry["dependencies"]["effects"][0].clone()).expect("typed effect");
    let light = effect
        .condition
        .as_ref()
        .expect("light condition")
        .light
        .as_ref()
        .expect("source levels");
    let definition = ConditionDefinition::new(
        &effect.identity.key,
        1,
        ConditionValues::SpellLight {
            duration_ms: effect.duration_ms.expect("source lifetime"),
            level: light.level.try_into().expect("level"),
            color: light.color.try_into().expect("color"),
        },
    )
    .expect("actual condition definition");
    let expected = f
        .states
        .get(&f.runtime, f.caster, f.session)
        .expect("actual predecessor")
        .owned_conditions()
        .clone();
    let mut next = expected.clone();
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([3; 32]);
    let facts = ApplicationFacts {
        now: 100_000,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([4; 16]),
    };
    next.apply(
        &definition,
        Some("actor:source-light".into()),
        ConditionSourceKind::SelfUse,
        &[],
        &facts,
    )
    .expect("real store application");
    (expected, next)
}
#[test]
fn source_conditions_join_real_hp_payment_and_replay_without_second_installation() {
    let mut f = fixture();
    let (expected, next) = source_light_successor(&f);
    f.batch.effects.push(OwnerCombatEffect {
        target: f.caster,
        sub_ordinal: 2,
        change: OwnerCombatChange::PlayerConditions {
            expected: Box::new(expected),
            next: Box::new(next.clone()),
        },
    });
    let staged = f
        .runtime
        .stage_spell_batch(&f.batch)
        .expect("physical batch");
    // The physical owner cannot commit a player condition without the actual
    // player predecessor proof, even with a correctly encoded condition body.
    assert!(commit_owner_batch(&mut f.runtime, &mut f.states, staged, None).is_err());
    assert_eq!(target_health(&f), 20);
    let staged = f
        .runtime
        .stage_spell_batch(&f.batch)
        .expect("unmutated owner");
    let proof = stage_player_batch(&f.runtime, &f.states, &f.batch, Some(f.next.clone()))
        .expect("actual condition predecessor proof");
    assert!(
        commit_owner_batch(&mut f.runtime, &mut f.states, staged, Some(proof))
            .expect("joined commit")
            .applied
    );
    let before = f
        .states
        .get(&f.runtime, f.caster, f.session)
        .expect("committed player")
        .clone();
    assert_eq!(before.owned_conditions(), &next);
    assert_eq!(target_health(&f), 17);
    let replay = f
        .runtime
        .stage_spell_batch(&f.batch)
        .expect("retained exact original");
    assert!(
        !commit_owner_batch(&mut f.runtime, &mut f.states, replay, None)
            .expect("physical receipt")
            .applied
    );
    assert_eq!(f.states.get(&f.runtime, f.caster, f.session), Some(&before));
    assert_eq!(target_health(&f), 17);
}
#[test]
fn source_condition_predecessor_substitution_refuses_entire_hp_and_cost_batch() {
    let mut f = fixture();
    let (_, next) = source_light_successor(&f);
    f.batch.effects.push(OwnerCombatEffect {
        target: f.caster,
        sub_ordinal: 2,
        change: OwnerCombatChange::PlayerConditions {
            expected: Box::new(next.clone()),
            next: Box::new(next),
        },
    });
    let before = f
        .states
        .get(&f.runtime, f.caster, f.session)
        .expect("actual predecessor")
        .clone();
    assert!(stage_player_batch(&f.runtime, &f.states, &f.batch, Some(f.next.clone())).is_err());
    assert_eq!(f.states.get(&f.runtime, f.caster, f.session), Some(&before));
    assert_eq!(target_health(&f), 20);
}
