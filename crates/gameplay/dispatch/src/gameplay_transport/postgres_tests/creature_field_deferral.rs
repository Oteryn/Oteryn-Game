//! Creature field entry and periodic HP through the SQL-backed map pass.
use super::fixture::{Harness, debug, fence, id};
use super::loop_monster_fixtures::fixture::{cell, dependencies};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::native_world_items::{SpellGroundTarget, qualify_source_field_chain};
use crate::durability::spell_item_transaction::{
    apply_spell_items_in_transaction, assert_spell_item_authority_in_transaction,
    assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
};
use crate::durability::spell_items_abi::{
    CastCostBinding, QualifiedItemDefinition, SourceCasterOrigin, SpellItemOperation,
    SpellItemTransactionOutcome, SpellItemTransactionRequest,
};
use crate::durability::{DurabilityError, spell_owner_commit::SpellLanePermit};
use crate::foundation::{
    ActivityTransferMember, ActorRef, ChannelRuntimeV1, CharacterId, CharacterLease, CommandId,
    CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
};
use crate::gameplay_transport::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use crate::movement::monster_map::CurrentMonsterMap;
use crate::spell::stance_execution::PreparedStance;
use oteryn_gameplay_domain::domain::{
    carrier_values::CarrierError,
    condition::{ConditionValues, DotElement},
};
use oteryn_gameplay_entities::carrier::encounter_access::{
    EncounterHealProjection, EncounterHooks,
};
use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
use oteryn_gameplay_entities::carrier::{
    ChannelActorCarrier, NamespaceContinuityGuard,
    runtime_actor_source_reservation::{
        ScopeCreatureReservationProof, SourceActorReservation, SourceActorReservationProof,
        source_reservation_seal,
    },
};
use oteryn_gameplay_entities::{
    AttackerCommand, CreatureDeathOccurrenceKey, OwnerDamageCommand, OwnerDamageResult,
};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, TickFacts,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};

const SECOND: u64 = 1_000_000;
const CYCLOPS: &str = "oteryn:creature.cyclops";

#[test]
fn immediate_field_entry_commits_with_a_reserved_source() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::ReservedSource)
}

#[test]
fn hooked_guardian_keeps_due_damage_until_unprotected() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Hooked)
}

#[test]
fn pending_fire_caster_does_not_hold_another_players_due_poison() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::ReadyPoison)
}

#[test]
fn pending_stance_without_carrier_reservation_holds_due_damage() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::PendingStance)
}

#[test]
fn deferred_periodic_turn_consumes_two_ticks_and_same_time_replay_consumes_none() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Replay)
}

#[test]
fn deferred_entries_are_pruned_after_target_death_and_despawn() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::TargetRemoved)
}

#[test]
fn deferred_entry_is_pruned_when_a_companion_owner_changes_scope() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::ScopeChange)
}

#[test]
fn reserved_target_that_steps_off_takes_one_field_application() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::TargetMoves)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    ReservedSource,
    Hooked,
    ReadyPoison,
    PendingStance,
    Replay,
    TargetRemoved,
    ScopeChange,
    TargetMoves,
}

fn run(admin: String, case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let executor =
                crate::bestiary_postgres_harness::runtime().map_err(|error| error.to_string())?;
            executor
                .block_on(async {
                    let harness = Harness::create_with_admission_seed(
                        admin,
                        "creature_field_deferral",
                        false,
                        false,
                    )
                    .await?;
                    let result = exercise(&harness, case).await;
                    let cleanup = harness.cleanup().await;
                    result?;
                    cleanup
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "creature field deferral scenario panicked")?
        .map_err(Into::into)
}

async fn exercise(harness: &Harness, case: Case) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = fence(1)?.runtime_scope.world_id();
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let (mut deps, rat) = dependencies(active, &pin)?;
    let runtime = deps.runtime.get_mut();
    runtime.remove_test_actor(rat).map_err(debug)?;
    let maximum = runtime
        .companion_policy(CYCLOPS)
        .map_err(debug)?
        .maximum_health;
    let mut spawn = |position, owned| -> TestResult<_> {
        if owned {
            return runtime
                .create_companion(
                    deps.actor,
                    deps.admitted.game_session_id,
                    CYCLOPS,
                    position,
                    None,
                    None,
                    0,
                )
                .map_err(debug);
        }
        let actor = runtime
            .admit_pinned_monster_lab_creature(position, CYCLOPS, maximum)
            .map_err(debug)?;
        runtime
            .install_creature_policy(actor, CYCLOPS)
            .map_err(debug)?;
        Ok(actor)
    };
    let first = spawn(cell(2), case == Case::ScopeChange)?;
    let second = spawn(cell(3), case == Case::TargetRemoved)?;
    assert_eq!(
        runtime
            .visible_entities()
            .creatures
            .iter()
            .map(|v| v.actor)
            .collect::<Vec<_>>(),
        [first, second],
        "the second creature follows the one that would refuse on main",
    );
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    owner.required_capabilities =
        crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[11, 30],
            fence(1)?.runtime_scope,
            1,
            "policy-1",
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        ).map_err(debug)?;
    let source = (deps.actor, deps.admitted.game_session_id);
    let cells = [cell(2), cell(3)];
    let field_cells = match case {
        Case::ReservedSource | Case::ScopeChange | Case::TargetMoves => &cells[..1],
        Case::ReadyPoison => &cells[1..],
        Case::TargetRemoved => &cells[..],
        Case::Hooked | Case::PendingStance | Case::Replay => &[],
    };
    if !field_cells.is_empty() {
        seed_fire(harness, &deps, &owner, native, field_cells).await?;
    }
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    let pass = FieldPass {
        owner: &owner,
        deps: &deps,
        native,
        permit: &permit,
    };
    let mut runtime = deps.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    initialize_source(&mut runtime, &mut states, source, id(41))?;
    let before = health(&runtime, first)?;
    let other_before = health(&runtime, second)?;
    if !matches!(case, Case::ReadyPoison | Case::TargetRemoved) {
        install_dot(&mut runtime, second, None, DotElement::Drown, 0)?;
    }
    match case {
        Case::ReservedSource => {
            let proof = HeldSource(source);
            let held = hold_source(&mut runtime, &proof)?;
            assert!(remaining(&runtime, first)?.is_empty());
            assert!(
                pass.tick(&mut runtime, &mut states, 0).await.is_ok(),
                "reserved source aborted the pass"
            );
            assert_eq!(health(&runtime, first)?, before);
            assert_eq!(
                remaining(&runtime, first)?.len(),
                1,
                "entry committed while the source was reserved"
            );
            assert_eq!(health(&runtime, second)?, other_before - 4);
            runtime
                .release_source_actors_after_observed_outcome(held, &proof)
                .map_err(debug)?;
            pass.tick(&mut runtime, &mut states, 0).await?;
            assert_eq!(health(&runtime, first)?, before - 9);
        }
        Case::Hooked => {
            install_dot(&mut runtime, first, None, DotElement::Drown, 0)?;
            set_guardian_gate(&mut runtime, Some(first))?;
            assert!(
                pass.tick(&mut runtime, &mut states, SECOND).await.is_ok(),
                "hooked guardian aborted the pass"
            );
            assert_eq!(health(&runtime, first)?, before);
            assert_eq!(health(&runtime, second)?, other_before - 8);
            assert_eq!(remaining(&runtime, first)?, [10]);
            set_guardian_gate(&mut runtime, None)?;
            pass.tick(&mut runtime, &mut states, SECOND).await?;
            assert_eq!(health(&runtime, first)?, before - 8);
            pass.tick(&mut runtime, &mut states, SECOND).await?;
            assert_eq!(health(&runtime, first)?, before - 8);
        }
        Case::ReadyPoison => {
            pass.tick(&mut runtime, &mut states, 0).await?;
            let fire_after_entry = remaining(&runtime, second)?;
            assert_eq!(health(&runtime, second)?, other_before - 9);
            assert_eq!(fire_after_entry.len(), 1);
            let session = GameSessionId::decode(&id(51)).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let poisoner = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(poisoner)
                .map_err(debug)?;
            initialize_source(&mut runtime, &mut states, (poisoner, session), id(52))?;
            install_dot(&mut runtime, second, Some(poisoner), DotElement::Poison, 1)?;
            // An earlier due fire turn is the regression witness. The fire entry
            // on the poison target was already applied at its current revision.
            install_dot(&mut runtime, first, Some(source.0), DotElement::Fire, 1)?;
            let _stance = reserve_stance(&runtime, &mut states, source)?;
            assert!(
                pass.tick(&mut runtime, &mut states, 1).await.is_ok(),
                "busy fire witness aborted the pass"
            );
            assert_eq!(health(&runtime, first)?, before);
            assert_eq!(
                health(&runtime, second)?,
                other_before - 13,
                "only poison was due"
            );
            let mut totals = remaining(&runtime, second)?;
            totals.sort_unstable();
            let mut expected = vec![5, fire_after_entry[0]];
            expected.sort_unstable();
            assert_eq!(totals, expected, "fire cursor was not reapplied");
        }
        Case::PendingStance => {
            install_dot(&mut runtime, first, Some(source.0), DotElement::Drown, 0)?;
            let stance = reserve_stance(&runtime, &mut states, source)?;
            assert!(!runtime.actor_spell_reserved(source.0));
            assert!(states.has_pending_spell_commit(source.0, source.1));
            assert!(
                pass.tick(&mut runtime, &mut states, 0).await.is_ok(),
                "pending spell commit aborted the pass"
            );
            assert_eq!(health(&runtime, first)?, before);
            assert_eq!(health(&runtime, second)?, other_before - 4);
            assert!(
                states
                    .get_mut(&runtime, source.0, source.1)
                    .ok_or("player spell state")?
                    .cancel_stance(&stance)
            );
            pass.tick(&mut runtime, &mut states, 0).await?;
            assert_eq!(health(&runtime, first)?, before - 4);
        }
        Case::Replay => {
            install_dot(&mut runtime, first, Some(source.0), DotElement::Drown, 0)?;
            let proof = HeldSource(source);
            let held = hold_source(&mut runtime, &proof)?;
            assert!(
                pass.tick(&mut runtime, &mut states, 0).await.is_ok(),
                "reserved source aborted the two-tick case"
            );
            assert_eq!(remaining(&runtime, first)?, [10]);
            runtime
                .release_source_actors_after_observed_outcome(held, &proof)
                .map_err(debug)?;
            pass.tick(&mut runtime, &mut states, SECOND).await?;
            assert_eq!(health(&runtime, first)?, before - 8);
            assert!(remaining(&runtime, first)?.is_empty());
            pass.tick(&mut runtime, &mut states, SECOND).await?;
            assert_eq!(health(&runtime, first)?, before - 8);
            assert!(remaining(&runtime, first)?.is_empty());
        }
        Case::TargetRemoved | Case::ScopeChange | Case::TargetMoves => {
            let all_targets = [first, second];
            let targets = if case == Case::TargetRemoved {
                &all_targets[..]
            } else {
                &all_targets[..1]
            };
            let proof = HeldTargets {
                targets,
                scope: runtime.binding().runtime_scope(),
                generation: runtime.binding().scope_generation(),
            };
            let mut held = runtime
                .prepare_scope_creature_reservation(&proof)
                .map_err(debug)?;
            runtime
                .reserve_scope_creatures(&mut held, &proof)
                .map_err(debug)?;
            let outcome = pass.tick(&mut runtime, &mut states, 0).await;
            assert!(
                outcome.is_ok(),
                "reserved target aborted the pass: {outcome:?}"
            );
            assert_eq!(states.deferred_creature_field_entries.len(), targets.len());
            for &actor in targets {
                assert!(
                    states
                        .deferred_creature_field_entries
                        .contains_key(&actor.placement_identity())
                );
                assert!(remaining(&runtime, actor)?.is_empty());
            }
            assert_eq!(health(&runtime, first)?, before);
            runtime
                .release_scope_creatures_after_observed_outcome(held, &proof)
                .map_err(debug)?;

            if case == Case::TargetMoves {
                assert_eq!(health(&runtime, second)?, other_before - 4);
                let position = runtime.read_actor_position(first).map_err(debug)?;
                runtime
                    .borrow_movement_position()
                    .commit_creature_relocation(first, position, cell(4))
                    .map_err(debug)?;
                // Yellow flame has seven ten-point ticks spaced ten seconds apart.
                // The pinned Cyclops mitigation truncates each tick to nine HP.
                let last = 1 + 60 * SECOND;
                for tick in 0..7 {
                    pass.tick(&mut runtime, &mut states, 1 + tick * 10 * SECOND)
                        .await?;
                    assert_eq!(
                        health(&runtime, first)?,
                        before - 9 * (i64::try_from(tick)? + 1)
                    );
                }
                assert!(states.deferred_creature_field_entries.is_empty());
                assert!(remaining(&runtime, first)?.is_empty());
                pass.tick(&mut runtime, &mut states, last).await?;
                assert_eq!(health(&runtime, first)?, before - 63);
            } else if case == Case::TargetRemoved {
                kill_between_passes(&mut runtime, first, before)?;
                let doomed = runtime.companion_snapshot(second).map_err(debug)?;
                runtime
                    .despawn_companion(source.0, source.1, &doomed)
                    .map_err(debug)?;
                pass.tick(&mut runtime, &mut states, 1).await?;
                assert!(states.deferred_creature_field_entries.is_empty());
                assert!(
                    runtime
                        .companion_snapshot_including_dead(first)
                        .map_err(debug)?
                        .state
                        .conditions
                        .instances()
                        .is_empty()
                );
                assert!(!runtime.contains_live_creature(second));
            } else {
                let moved = transfer_to_activity(&mut runtime, source)?;
                assert_ne!(moved.runtime_scope(), first.runtime_scope());
                assert!(!runtime.contains_live_creature(first));
                assert!(runtime.contains_live_creature(second));
                drop(states);
                drop(runtime);
                owner.prune_transferred_creature_fields().await;
                assert!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .deferred_creature_field_entries
                        .is_empty()
                );
            }
        }
    }
    Ok(())
}

struct HeldTargets<'a> {
    targets: &'a [ExactActorRef],
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
}

impl source_reservation_seal::Sealed for HeldTargets<'_> {}
impl ScopeCreatureReservationProof for HeldTargets<'_> {
    fn scope(&self) -> (RuntimeScopeRefV1, ScopeOwnershipGeneration) {
        (self.scope, self.generation)
    }
    fn event_identity(&self) -> [u8; 16] {
        id(10)
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        [10; 32]
    }
    fn affected_creatures(&self) -> &[ExactActorRef] {
        self.targets
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.targets
            .iter()
            .all(|&actor| runtime.contains_live_creature(actor))
    }
}

fn kill_between_passes(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    hp: i64,
) -> TestResult {
    let amount = u32::try_from(hp)?;
    let lethal = CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "test.deferral.lethal",
            1,
            ConditionValues::Agony {
                total_min: amount,
                total_max: amount,
                per_tick: amount,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .ok_or("lethal condition")?,
        source: None,
        lineage: None,
        occurrence: DecisionOccurrenceId::from_bytes(id(11)),
    };
    runtime
        .commit_creature_field_entry(actor, 1, Some(&lethal))
        .map_err(debug)?;
    let turn = runtime
        .stage_creature_periodic_turn(actor, 1, TickFacts::default())
        .map_err(debug)?;
    let receipt = runtime.commit_creature_periodic_turn(turn).map_err(debug)?;
    assert_eq!(receipt.health_after, 0);
    Ok(())
}

fn transfer_to_activity(
    runtime: &mut ChannelRuntimeV1,
    (actor, session): (ExactActorRef, GameSessionId),
) -> TestResult<ExactActorRef> {
    let binding = runtime.binding();
    let mut destination = ChannelRuntimeV1::from_committed_scope_assignment(
        RuntimeScopeRefV1::instance(binding.world_id(), id(55)).map_err(debug)?,
        binding.node_id(),
        binding.node_registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        runtime.content_pin().clone(),
    )
    .map_err(debug)?;
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let members = [ActivityTransferMember {
        source_actor: actor,
        source_session: session,
        source_lease: lease,
        destination_session: GameSessionId::decode(&id(56)).map_err(debug)?,
        destination_lease: CharacterLease::new(lease.character_id(), lease.generation() + 1)
            .map_err(debug)?,
        destination_cell: cell(2),
    }];
    // The SQL-backed field pass has already retained the contact. The test
    // drives the physical side of a known activity handoff and its cleanup.
    let mut prepared = runtime
        .prepare_activity_transfer(&mut destination, &members)
        .map_err(debug)?;
    let moved = runtime
        .commit_activity_transfer(&mut destination, &mut prepared)
        .map_err(debug)?;
    Ok(moved.first().ok_or("transferred player")?.destination_actor)
}

struct FieldPass<'a, 'f, 's> {
    owner: &'a ComposedFreshAdmission<'a, 'f, 's>,
    deps: &'a Dependencies,
    native: &'a NativeGameplayState,
    permit: &'a SpellLanePermit,
}

impl FieldPass<'_, '_, '_> {
    async fn tick(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        now: u64,
    ) -> Result<(), DurabilityError> {
        let owner = self.owner;
        let prepared = owner
            .capture_native_health_names(runtime)
            .map_err(|_| DurabilityError::Unavailable)?;
        let health_names = owner
            .read_prepared_native_health_names(self.permit, prepared)
            .await;
        let names = owner
            .seal_native_health_names(runtime, health_names)
            .map_err(|_| DurabilityError::Unavailable)?;
        let objects = self.deps.door.lock().await;
        let mut context = (runtime, states, &*objects, &names, self);
        owner
            .root
            .try_issue_semantic_pass()?
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let (runtime, states, objects, names, pass) = context;
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let binding = runtime.binding();
                    let authority = assert_spell_item_scope_in_transaction(
                        &mut tx,
                        pass.permit,
                        pass.owner.root,
                        pass.owner.character,
                        pass.owner.holder,
                        binding.runtime_scope(),
                        binding.scope_generation().get(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let mut map = CurrentMonsterMap::new(
                        &mut tx,
                        &authority,
                        runtime,
                        &pass.deps.room,
                        objects,
                        pass.native,
                    );
                    pass.owner
                        .tick_creature_fields(&mut map, runtime, states, pass.native, now, names)
                        .await?;
                    drop(map);
                    tx.rollback().await?;
                    Ok(())
                })
            })
            .await
    }
}

fn health(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> TestResult<i64> {
    Ok(runtime.companion_snapshot(actor).map_err(debug)?.health)
}

fn remaining(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> TestResult<Vec<u32>> {
    Ok(runtime
        .companion_snapshot(actor)
        .map_err(debug)?
        .state
        .conditions
        .instances()
        .iter()
        .map(|condition| condition.remaining_total())
        .collect())
}

fn install_dot(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    source: Option<ExactActorRef>,
    element: DotElement,
    now: u64,
) -> TestResult {
    // TEST_PARAMETERS: two five-point ticks, with the first one due immediately.
    let definition = ConditionDefinition::new(
        "test.field.deferral.dot",
        1,
        ConditionValues::DamageOverTime {
            element,
            total_min: 10,
            total_max: 10,
            per_tick: 5,
            interval_ms: 1000,
            delayed: false,
        },
    )
    .ok_or("periodic definition")?;
    if source.is_none() {
        runtime
            .commit_creature_field_entry(
                actor,
                now,
                Some(&CreatureFieldEntry {
                    definition,
                    source: None,
                    lineage: None,
                    occurrence: DecisionOccurrenceId::from_bytes(id(5)),
                }),
            )
            .map_err(debug)?;
    } else {
        let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
        let mut next = snapshot.state.clone();
        next.conditions
            .apply(
                &definition,
                source,
                ConditionSourceKind::Player,
                &[],
                &ApplicationFacts {
                    now,
                    base_speed: 100,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: false,
                    decision_root: &GameplayDecisionRoot::from_bytes(
                        runtime.content_pin().server_artifact_digest(),
                    ),
                    occurrence: DecisionOccurrenceId::from_bytes(id(6)),
                },
            )
            .map_err(debug)?;
        runtime
            .compare_companion_state(&snapshot, next)
            .map_err(debug)?;
    }
    Ok(())
}

fn initialize_source(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    (actor, session): (ExactActorRef, GameSessionId),
    character: [u8; 16],
) -> TestResult {
    states
        .initialize(
            runtime,
            actor,
            session,
            crate::spell::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::EliteKnight,
                level: 50,
                magic_level: 0,
                max_health: 500,
                max_mana: 600,
                max_soul: 100,
            },
            (0, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .ok_or("player spell state")?;
    let lease =
        CharacterLease::new(CharacterId::decode(&character).map_err(debug)?, 1).map_err(debug)?;
    runtime
        .bind_attacker_lease(actor, session, lease)
        .map_err(debug)?;
    Ok(())
}

fn reserve_stance(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    (actor, session): (ExactActorRef, GameSessionId),
) -> TestResult<PreparedStance> {
    use crate::spell::{Vocation, cast::prepare_native_owner_cast};
    let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json",
    )))?;
    let protector = document["profiles"]
        .as_array()
        .and_then(|profiles| profiles.iter().find(|p| p["name"] == "Protector"))
        .ok_or("Protector profile")?;
    let mut spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &serde_json::json!({"spell": protector["spell"]}),
        &protector["dependencies"],
    )
    .map_err(debug)?;
    // Logical reservation fixture, with no commercial proof or durable receipt.
    spell.premium = false;
    let state = states
        .get_mut(runtime, actor, session)
        .ok_or("player spell state")?;
    let now = SemanticTimeMicros::from_micros(0);
    let position = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let paid = prepare_native_owner_cast(
        state,
        &spell,
        &crate::spell::OperationalCastFacts {
            caster_position: crate::spell::chain::TilePosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            target_position: None,
            target: None,
            line_of_sight_clear: None,
            direction_available: false,
            wheel_unlocked: None,
            in_protection_zone: false,
            target_tile_solid: None,
            target_tile_creature: None,
        },
        crate::spell::native::Facts::Stance {
            active: None,
            vocation: Vocation::EliteKnight,
        },
        now,
        &mut |_, _| 0,
    )
    .map_err(debug)?;
    let prepared = PreparedStance::new(
        state,
        paid,
        &spell,
        actor,
        session,
        1,
        std::num::NonZeroU32::MIN,
        fence(1)?,
        "content:1".into(),
        "policy:1".into(),
        now,
    )
    .map_err(debug)?;
    state.reserve_stance(prepared.clone()).map_err(debug)?;
    Ok(prepared)
}

struct HeldSource((ExactActorRef, GameSessionId));
impl source_reservation_seal::Sealed for HeldSource {}
impl SourceActorReservationProof for HeldSource {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        self.0
    }
    fn event_identity(&self) -> [u8; 16] {
        id(9)
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        [9; 32]
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        &[]
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.player_control_facts(self.0.0, self.0.1).is_ok()
    }
}

fn hold_source(
    runtime: &mut ChannelRuntimeV1,
    proof: &HeldSource,
) -> TestResult<SourceActorReservation> {
    let mut held = runtime
        .prepare_source_actor_reservation(proof)
        .map_err(debug)?;
    runtime
        .reserve_source_actors(&mut held, proof)
        .map_err(debug)?;
    assert!(runtime.actor_spell_reserved(proof.0.0));
    Ok(held)
}

// The current DamageGates API only exposes protect. Model the same live hook
// boundary with a fixture gate whose membership can also be removed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct GuardianGate(Option<ExactActorRef>);
impl EncounterHooks for GuardianGate {
    fn clone_hooks(&self) -> Box<dyn EncounterHooks> {
        Box::new(self.clone())
    }
    fn eq_hooks(&self, other: &dyn EncounterHooks) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn contains(&self, actor: ExactActorRef) -> bool {
        self.0 == Some(actor)
    }
    fn damage(
        &self,
        _: &mut ChannelActorCarrier,
        _: &NamespaceContinuityGuard,
        actor: ActorRef,
        _: OwnerDamageCommand<'_>,
        _: Option<AttackerCommand>,
        _: bool,
        _: Option<&ChannelContentPin>,
        _: Option<u16>,
        _: Option<u64>,
    ) -> Option<Result<OwnerDamageResult, CarrierError>> {
        self.0
            .filter(|protected| protected.0 == actor)
            .map(|_| Err(CarrierError::PlanConflict))
    }
    fn projected_death(&mut self, actor: ExactActorRef, _: CreatureDeathOccurrenceKey) {
        if self.contains(actor) {
            self.0 = None;
        }
    }
    fn expand_heals(
        &self,
        _: &ChannelRuntimeV1,
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<EncounterHealProjection, CarrierError> {
        Ok((requests.to_vec(), (0..requests.len()).collect()))
    }
}

fn set_guardian_gate(runtime: &mut ChannelRuntimeV1, actor: Option<ExactActorRef>) -> TestResult {
    runtime
        .with_encounter_hooks::<GuardianGate, _, CarrierError>(|_, gate| {
            gate.0 = actor;
            Ok(())
        })
        .map_err(debug)?;
    Ok(())
}

async fn seed_fire(
    harness: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    native: &NativeGameplayState,
    positions: &[MovementLocalPosition],
) -> TestResult {
    use crate::durability::item_mint::TypedDefinitionRef;
    let policy = native
        .item_policy_for_source_id(21465)
        .ok_or("native fire field")?;
    let definition = qualify_source_field_chain(
        native,
        QualifiedItemDefinition::from_native_policy(policy).map_err(debug)?,
    )
    .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    let mut operations = Vec::new();
    {
        let runtime = deps.runtime.lock().await;
        for (index, &position) in positions.iter().enumerate() {
            let target = SpellGroundTarget::for_native_tile_read(&deps.room, &runtime, position)
                .map_err(debug)?;
            operations.push(SpellItemOperation::MintGround {
                item_instance_id: id(180 + u8::try_from(index)?),
                definition: definition.clone(),
                quantity: 1,
                placement: target.placement(b"creature-field-deferral".to_vec()),
                lifetime_millis: None,
                blocks_movement: policy
                    .record()
                    .attributes
                    .blocks_movement
                    .ok_or("field movement")?,
                blocks_projectile: policy
                    .record()
                    .attributes
                    .blocks_projectile
                    .ok_or("field projectile")?,
                description: None,
            });
        }
    }
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("field fence")?;
    let command = CommandRef::new(
        deps.admitted.game_session_id,
        CommandId::new(4900).map_err(debug)?,
    );
    let request = SpellItemTransactionRequest {
        swing_ordinal: None,
        command,
        spell: TypedDefinitionRef {
            family: "Spell".into(),
            production_key: "fixture:field.deferral".into(),
            revision_ref: "definition-r1".into(),
        },
        catalog_digest: native.source_digest(),
        transaction_id: id(188),
        event_id: id(189),
        cost: CastCostBinding {
            vitals_revision_before: 1,
            vitals_revision_after: 2,
            mana_before: 200,
            mana_after: 180,
            soul_before: 100,
            soul_after: 100,
            cooldowns_before: Vec::new(),
            cooldowns_after: vec![("spell:fixture_field_deferral".into(), 100_000)],
            caster_digest_before: [11; 32],
            caster_digest_after: [12; 32],
        },
        caster_origin: Some(SourceCasterOrigin {
            actor: deps.actor,
            character_lease_generation: fence.character_lease_generation,
        }),
        operations,
        companion: None,
        direct_companion: None,
    };
    let mut tx = harness.pool.begin().await?;
    let authority = assert_spell_item_authority_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        &fence,
        command,
        native.source_digest(),
    )
    .await
    .map_err(debug)?;
    let origin = crate::gameplay_transport::world_required_field_guard::player_origin(
        &owner.required_capabilities,
    );
    let outcome = apply_spell_items_in_transaction(&mut tx, &authority, &request, &origin)
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, SpellItemTransactionOutcome::Applied(_)));
    tx.commit().await?;
    Ok(())
}
