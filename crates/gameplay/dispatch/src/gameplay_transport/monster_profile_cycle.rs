//! Channel composition: current Item/Map reads, one native think and synchronous summons.
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates};
use crate::ai_think::summon_plan::summon_path::NativeMonsterThinkRead;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::ChannelRuntimeV1;
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_simulation_determinism::GameplayDecisionRoot;

mod bloodjaw_targeting;
pub(super) mod native_callbacks;
pub(super) mod physical_spell;
pub(in crate::gameplay_transport) mod rotten_area;
pub(super) mod rotten_waves;
pub(super) mod source_heals;
mod source_items;

pub(super) fn publish_native_facing(
    runtime: &mut ChannelRuntimeV1,
    table: &crate::ai_think::CreatureAiTable,
    actor: crate::foundation::ExactActorRef,
) {
    let Some(state) = table.state(actor) else {
        return;
    };
    if !state.profile().is_native() {
        return;
    }
    let Ok(expected) = runtime.read_actor_position(actor) else {
        return;
    };
    let facing = match state.attack_facing() {
        crate::movement::CardinalStep::North => crate::foundation::MovementFacing::North,
        crate::movement::CardinalStep::East => crate::foundation::MovementFacing::East,
        crate::movement::CardinalStep::South => crate::foundation::MovementFacing::South,
        crate::movement::CardinalStep::West => crate::foundation::MovementFacing::West,
        crate::movement::CardinalStep::NorthEast | crate::movement::CardinalStep::NorthWest => {
            crate::foundation::MovementFacing::North
        }
        crate::movement::CardinalStep::SouthEast | crate::movement::CardinalStep::SouthWest => {
            crate::foundation::MovementFacing::South
        }
    };
    if expected.facing() == Some(facing) {
        return;
    }
    // Publish the source AI turn through the same pose owner as ordinary turns.
    // This neither moves the actor nor changes its last translation revision.
    let _ = runtime
        .borrow_movement_position()
        .commit_turn(expected, facing);
}

// The bounded source overlay admits only the exact live Instance creature's
// added callback; ordinary source abilities retain their existing closure gate.
fn accepts_source_overlay(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    actor: crate::foundation::ExactActorRef,
    ability: &crate::content::ProjectV2DefinitionRef,
) -> bool {
    if !matches!(
        runtime.binding().runtime_scope(),
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) || native.source_digest() != runtime.content_pin().server_artifact_digest()
    {
        return false;
    }
    let Ok(key) = runtime.current_live_creature_identity(actor) else {
        return false;
    };
    let mut records = native
        .creature_profiles()
        .records
        .iter()
        .filter(|record| record.profile.target.key.as_bytes() == key);
    let Some(record) = records.next() else {
        return false;
    };
    records.next().is_none()
        && native
            .soul_war_source_attacks(&record.profile.target)
            .is_some_and(|source| source.accepts_ability(ability))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn drive_monster_profiles(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        revisions: &crate::ability::RevisionSet,
        candidates: &[crate::ai_think::targeting::TargetCandidate],
        now: u64,
        native_health_names: &super::source_names::NativeHealthNamesData,
    ) -> Result<(Option<u64>, Vec<crate::foundation::ExactActorRef>), ()> {
        use crate::durability::spell_item_transaction::{
            assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
        };
        let primal_pending = states.primal_callbacks.normal_pending();
        let now = primal_pending
            .as_ref()
            .map_or(now, |pending| pending.batch().observed_at_us());
        if primal_pending
            .as_ref()
            .is_some_and(|pending| !pending.source_ready() || !pending.batch().current_for(runtime))
        {
            return Err(());
        }
        runtime.note_rotten_movement_time(now).map_err(|_| ())?;
        if runtime.rotten_movement_pending() {
            return Err(());
        }
        if self.source_death_can_begin_without_timer_read(runtime, states, now)? {
            return if self.prepare_source_death_head(runtime, states, attack, room, native, now)? {
                Ok((Some(now), Vec::new()))
            } else {
                // The outer native loop drops its guards before executing the
                // exact retained copied action, including all physical SQL.
                Err(())
            };
        }
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let (next_due, proposals, native_actors) = {
            let mut context = (
                self,
                &mut *runtime,
                &mut *states,
                attack,
                room,
                native,
                permit,
                revisions,
                candidates,
                primal_pending.as_ref(),
                native_health_names,
            );
            pass.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                let (owner, runtime, states, attack, room, native, permit, revisions, candidates, primal_pending, native_health_names) =
                    context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let binding = runtime.binding();
                let scope = binding.runtime_scope();
                let authority = assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    scope,
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                // Deferred source tasks precede fresh movement/thinks. Read the
                // same current party basis only when their retained older casts
                // can need it, then release this read transaction after one task.
                let source_parties = if runtime.has_pending_source_creature_deaths() {
                    let required = native.creature_profiles().records.iter().any(|record| {
                        matches!(&record.profile.data, crate::content::ProjectV2AuthoringProfileData::Creature(creature)
                            if creature.details.as_ref().is_some_and(|details| details.reference_level_overrides.iter()
                                .any(|area| area.party_mode == crate::content::ProjectV2ReferenceLevelPartyMode::LowestMember)))
                    });
                    physical_spell::read_reference_parties(&mut tx, &authority, owner.root, runtime, states, required).await
                } else { Default::default() };
                let source_instance = matches!(scope, crate::foundation::RuntimeScopeRefV1::Instance { .. })
                    && room.source_world().is_some();
                let wave_boss_present = source_instance && rotten_waves::has_boss(runtime);
                let wave_enabled = source_instance
                    && (wave_boss_present || states.rotten_waves.has_attempts());
                let wave_taints = if wave_enabled && wave_boss_present {
                    let sessions = runtime.positioned_actor_census()
                        .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                        .into_iter().filter_map(|(_, _, session)| session).collect::<Vec<_>>();
                    crate::durability::quest_state::rotten_blood::room_taints::read_in_transaction(
                        &mut tx, &authority, owner.holder, &sessions, native.source_digest(),
                    ).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?
                } else { Vec::new() };
                let objects = owner.door.lock().await;
                let mut map =
                    CurrentMonsterMap::new(&mut tx, &authority, runtime, room, &objects, native);
                if runtime.has_pending_source_creature_deaths() {
                    let complete = owner.drive_one_source_creature_death(runtime, states, attack,
                        room, native, &mut map, &source_parties, now, native_health_names).await
                        .map_err(|()|crate::durability::DurabilityError::Unavailable)?;
                    if !complete || !map.validate_transaction(runtime).await {
                        return Err(crate::durability::DurabilityError::Unavailable);
                    }
                    drop(map);
                    tx.rollback().await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                    // Immediate continuation names actual remaining source/AI
                    // work; it does not replace a source timer deadline.
                    return Ok((Some(now), Vec::new(), Vec::new()));
                }
                let root = GameplayDecisionRoot::from_bytes(native.source_digest());
                let bloodjaw_players = if wave_enabled {
                    bloodjaw_targeting::current_players(runtime, states)
                } else { None };
                let table = &mut states.monster_melee.creature_ai;
                let empty = std::collections::BTreeSet::new();
                let doors = if matches!(
                    room.movement_cells().index(),
                    crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
                ) {
                    &empty
                } else {
                    objects.collision_cells()
                };
                let player_waiting = attack.players_waiting(now);
                if primal_pending.is_none() {
                // Carried creature requests precede player chase and fresh thinks.
                table
                    .drive_current_movement(
                        runtime,
                        room.movement_cells(),
                        doors,
                        *native,
                        &root,
                        now,
                        &mut attack.path_budget,
                        player_waiting,
                        &mut map,
                    )
                    .await;
                attack.drain_chase_searches(
                    runtime,
                    room,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now),
                    &|at| super::attack::chase_path_tile(room.movement_cells(), doors, at),
                );
                let player_waiting_now = attack.players_waiting(now);
                if runtime.rotten_movement_pending() {
                    return Err(crate::durability::DurabilityError::Unavailable);
                }
                if player_waiting && !player_waiting_now {
                    table
                        .drive_current_movement(
                            runtime,
                            room.movement_cells(),
                            doors,
                            *native,
                            &root,
                            now,
                            &mut attack.path_budget,
                            false,
                            &mut map,
                        )
                        .await;
                }
                }
                let player_waiting_now = attack.players_waiting(now);
                // Crystal game.cpp:7051-7053 runs source onThink before onAttacking.
                // Reserve the existing bounded occurrences, release the table borrow,
                // then re-read physical type/HP before their native schedules run.
                let mut owned_batch = if primal_pending.is_none() { Some(table.reserve_current_thinks(runtime,now)
                    .ok_or(crate::durability::DurabilityError::Unavailable)?) } else { None };
                let batch = match primal_pending.as_ref() {
                    Some(pending) => pending.batch(),
                    None => owned_batch.as_ref().ok_or(crate::durability::DurabilityError::Unavailable)?,
                };
                if let Some(claim) = super::rotten_blood_callbacks::prepare_bak_first_think(
                    runtime, states, room, native, batch.occurrences(), now,
                ).map_err(|()|crate::durability::DurabilityError::Unavailable)? {
                    // Decide the source GLOBAL phase in this exact first-think
                    // transaction. Commit it BEFORE any HP/AI source consumer;
                    // the retained original occurrence resumes after publication.
                    if !map.validate_transaction(runtime).await {
                        return Err(crate::durability::DurabilityError::Unavailable);
                    }
                    drop(map);
                    crate::durability::quest_state::rotten_flame::capture_bak_first_think_in_transaction(
                        &mut tx, owner.holder, &claim,
                    ).await?;
                    tx.commit().await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                    return Ok((Some(0), Vec::new(), Vec::new()));
                }
                let published_soul_war_world = owner.active_generation
                    .and_then(|active|active.qualify_published_world_context(room,runtime.content_pin()));
                if rotten_area::drive(owner, runtime, states, room, native, batch,
                    published_soul_war_world.as_ref(), now)
                    .map_err(|()|crate::durability::DurabilityError::Unavailable)? {
                    // The native occurrence remains reserved until the original
                    // cohort finishes. This branch issued no physical/Quest
                    // writes. Drop the existing read transaction, then perform
                    // the detached scoped read outside runtime/state guards.
                    drop(map);
                    drop(tx);
                    return Ok((Some(0),Vec::new(),Vec::new()));
                }
                super::soul_war_callbacks::refresh_taints(owner.root,runtime,states,room,native,&mut map,now,published_soul_war_world.as_ref())
                    .await.map_err(|()|crate::durability::DurabilityError::Unavailable)?;
                super::soul_war_callbacks::drive(
                    runtime, states, attack, room, native, &mut map, batch.occurrences(), now,
                ).await.map_err(|()| crate::durability::DurabilityError::Unavailable)?;
                let primal_due = if room.pinned_activity_source_room() == Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Primal)
                    && matches!(runtime.binding().runtime_scope(),crate::foundation::RuntimeScopeRefV1::Instance{..}) {
                    if primal_pending.is_none() {
                        let batch = owned_batch.take().ok_or(crate::durability::DurabilityError::Unavailable)?;
                        states.primal_callbacks.prepare_normal(batch).map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                        drop(map); drop(tx);
                        return Ok((Some(0),Vec::new(),Vec::new()));
                    }
                    states.primal_callbacks.next_retained_due_us()
                } else { None };
                let rotten_due = if matches!(room.pinned_activity_source_room(),
                    Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Murcion
                        | crate::content::bosses::activity_template::PinnedActivitySourceRoom::Ichgahal
                        | crate::content::bosses::activity_template::PinnedActivitySourceRoom::Vemiath
                        | crate::content::bosses::activity_template::PinnedActivitySourceRoom::Chagorz
                        | crate::content::bosses::activity_template::PinnedActivitySourceRoom::Bakragore)) {
                    super::rotten_blood_callbacks::refresh_taints(owner.root, runtime, states, &mut map)
                        .await.map_err(|()| crate::durability::DurabilityError::Unavailable)?;
                    super::rotten_blood_callbacks::drive(runtime, states, attack, room, native, &mut map,
                        batch.occurrences(), now).await
                        .map_err(|()| crate::durability::DurabilityError::Unavailable)?
                } else {
                    states.rotten_blood_callbacks = Default::default();
                    None
                };

                if let Some(active)=owner.active_generation {
                    let callbacks=states.native_callbacks.clone();
                    callbacks.think(active,runtime,room,native,&mut map,
                        &mut states.monster_melee.creature_ai,batch.occurrences(),now).await
                        .map_err(|()|crate::durability::DurabilityError::Unavailable)?;
                }
                let table = &mut states.monster_melee.creature_ai;
                for occurrence in batch.occurrences() {
                    if runtime.contains_live_creature(occurrence.actor) {
                        table.refresh_current_native_creature(runtime, native, occurrence.actor)
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    }
                }
                if wave_enabled { rotten_waves::warm(runtime, &mut map).await; }
                let mut reader = NativeMonsterThinkRead::new(
                    map,
                    native,
                    &mut attack.path_budget,
                    now,
                    player_waiting_now,
                );
                // Native spell proposals carry the source closure's actual formula revision.
                let formula_revisions = native
                    .creature_profiles()
                    .records
                    .iter()
                    .filter_map(|record| record.attack_definitions.as_ref())
                    .flat_map(|definitions| &definitions.profiles)
                    .filter(|profile| {
                        matches!(
                            profile.data,
                            crate::content::ProjectV2AuthoringProfileData::Formula(_)
                        )
                    })
                    .map(|profile| profile.target.revision.as_str())
                    .collect::<std::collections::BTreeSet<_>>();
                let formula_revision = if formula_revisions.len() == 1 {
                    formula_revisions
                        .iter()
                        .next()
                        .copied()
                        .unwrap_or(revisions.formula())
                } else {
                    revisions.formula()
                };
                let schedule_revisions = crate::ability::RevisionSet::new(
                    revisions.ruleset(),
                    revisions.content(),
                    revisions.world_policy(),
                    formula_revision,
                    revisions.simulation(),
                )
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                let source_attack_abilities = native
                    .creature_profiles()
                    .records
                    .iter()
                    .filter_map(|record| record.attack_definitions.as_ref())
                    .flat_map(|definitions| &definitions.profiles)
                    .filter(|profile| {
                        matches!(
                            profile.data,
                            crate::content::ProjectV2AuthoringProfileData::Ability(_)
                        )
                    })
                    .map(|profile| &profile.target)
                    .collect::<std::collections::BTreeSet<_>>();
                let summons = &mut states.monster_summons;
                let waves = &mut states.rotten_waves;
                let mut proposals = Vec::new();
                let native_reports = if primal_pending.as_ref().is_some_and(|pending|pending.ai_known()) { Vec::new() } else { table
                    .run_reserved_current_thinks_borrowed(
                        runtime,
                        batch,
                        &root,
                        &schedule_revisions,
                        candidates,
                        &mut reader,
                        |runtime, table, actor, report, reader| {
                            publish_native_facing(runtime, table, actor);
                            let selected = report.proposals.iter().filter(|proposal| {
                                source_attack_abilities.contains(&proposal.ability)
                                    || accepts_source_overlay(runtime,native,actor,&proposal.ability)
                            }).cloned();
                            if let Some(pending) = primal_pending.as_ref() {
                                pending.record_report(actor,selected);
                            } else { proposals.extend(selected); }
                            summons
                                .consume_report(runtime, table, native, actor, report, reader, now);
                            if wave_enabled && !report.over_budget && bloodjaw_targeting::retarget(runtime, table,
                                actor, bloodjaw_players.as_deref(), now).is_err() {
                                eprintln!("oteryn-game-server event=bloodjaw_target_refused current_owner=true");
                            }
                            if wave_enabled && waves.consume(runtime, table, native, actor, report,
                                reader.map.view(), room, &wave_taints, now).is_err() {
                                eprintln!("oteryn-game-server event=rotten_wave_refused current_owner=true");
                            }
                        },
                    )
                    .await };
                let native_actors = if let Some(pending) = primal_pending.as_ref() {
                    pending.finish_ai();
                    let retained = pending.results().map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                    proposals = retained.0;
                    retained.1
                } else { native_reports.into_iter().map(|(actor,_)|actor).collect::<Vec<_>>() };
                let next_step = table
                    .drive_current_movement(
                        runtime,
                        room.movement_cells(),
                        doors,
                        *native,
                        &root,
                        now,
                        reader.budget,
                        player_waiting_now,
                        &mut reader.map,
                    )
                    .await;
                if !reader.map.validate_transaction(runtime).await {
                    return Err(crate::durability::DurabilityError::Unavailable);
                }
                // Transaction is read-only. Keep its scope lock until every synchronous consumer ends.
                drop(reader);
                let next_due = table.earliest_due().into_iter().chain(next_step).min();
                source_heals::execute(runtime, &mut states.native_self_heals, &mut states.native_self_heal_content, native, &proposals);
                owner
                    .tick_creature_fields(&mut tx, &authority, runtime, states, room, native, now, native_health_names)
                    .await?;
                let needs_reference_parties = native.creature_profiles().records.iter().any(|record| {
                    matches!(&record.profile.data, crate::content::ProjectV2AuthoringProfileData::Creature(creature)
                        if creature.details.as_ref().is_some_and(|details| details.reference_level_overrides.iter()
                            .any(|area| area.party_mode == crate::content::ProjectV2ReferenceLevelPartyMode::LowestMember)))
                });
                let parties = physical_spell::read_reference_parties(
                    &mut tx, &authority, owner.root, runtime, states, needs_reference_parties,
                )
                .await;
                let mut map =
                    CurrentMonsterMap::new(&mut tx, &authority, runtime, room, &objects, native);
                // Preserve the native scheduler's proposal order across ordinary
                // attacks and source callbacks. Delayed work also runs on idle turns.
                let callbacks = states.native_callbacks.clone();
                let mut soul_war_attack_due = None;
                let callback_result: Result<(), ()> = async {
                    for current in std::iter::once(&[][..]).chain(proposals.iter().map(std::slice::from_ref)) {
                        let due = super::soul_war_callbacks::drive_attacks(
                            runtime, states, room, native, &mut map, current, now,
                        ).await?;
                        soul_war_attack_due = soul_war_attack_due.into_iter().chain(due).min();
                        if let Some(active) = owner.active_generation {
                            callbacks.execute(active, runtime, states, attack, room, native,
                                &mut map, current, &parties, now, native_health_names).await?;
                        }
                        physical_spell::execute_proposals(runtime, states, attack, room, native,
                            &mut map, current, &parties, now, owner.imported_charms, native_health_names).await;
                    }
                    physical_spell::drain_due(runtime, states, attack, room, native,
                        &mut map, &parties, now, owner.imported_charms).await;
                    Ok(())
                }.await;
                callback_result.map_err(|()| crate::durability::DurabilityError::Unavailable)?;

                drop(map);
                let next_due = next_due.into_iter()
                    .chain(primal_due)
                    .chain(rotten_due)
                    .chain(states.rotten_blood_callbacks.next_due_time())
                    .chain(states.native_callbacks.next_due_time())
                    .chain(soul_war_attack_due)
                    .chain(states.soul_war_callbacks.next_native_due())
                    .chain(states.monster_spells.next_windup_due_time().map(|due| due.get()))
                    .chain(states.monster_chains.as_ref().and_then(|owner| owner.next_due_time()).map(|due| due.get()))
                    .min();
                tx.rollback()
                    .await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                Ok((next_due, proposals, native_actors))
            })
        })
        .await
        .map_err(|_| ())?
        };
        if states.rotten_blood_callbacks.critical_scope_pending() {
            return Err(());
        }
        // End the read transaction and semantic pass before the existing durable Item
        // owner opens its own fenced write transaction for the same source occurrence.
        source_items::execute(self, runtime, room, native, permit, &proposals)
            .await
            .map_err(|_| ())?;
        if let Some(pending) = primal_pending.as_ref() {
            states.primal_callbacks.complete_normal(pending)?;
        }
        Ok((next_due, native_actors))
    }
}
