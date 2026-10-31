//! Channel composition: current Item/Map reads, one native think and synchronous summons.
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates};
use crate::ai_think::summon_plan::summon_path::NativeMonsterThinkRead;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::ChannelRuntimeV1;
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_simulation_determinism::GameplayDecisionRoot;

pub(super) mod physical_spell;
mod source_items;

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
    ) -> Result<(Option<u64>, Vec<crate::foundation::ExactActorRef>), ()> {
        use crate::durability::spell_item_transaction::{
            assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
        };
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let (next_due, proposals, native_actors) = {
            let mut context = (
                self,
                &mut *runtime,
                states,
                attack,
                room,
                native,
                permit,
                revisions,
                candidates,
            );
            pass.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                let (owner, runtime, states, attack, room, native, permit, revisions, candidates) =
                    context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let binding = runtime.binding();
                let scope = crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                );
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
                let objects = owner.door.lock().await;
                let mut map =
                    CurrentMonsterMap::new(&mut tx, &authority, runtime, room, &objects, native);
                let root = GameplayDecisionRoot::from_bytes(native.source_digest());
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
                let mut proposals = Vec::new();
                let native_reports = table
                    .run_due_current_thinks(
                        runtime,
                        now,
                        &root,
                        &schedule_revisions,
                        candidates,
                        &mut reader,
                        |runtime, table, actor, report, reader| {
                            proposals.extend(
                                report
                                    .proposals
                                    .iter()
                                    .filter(|proposal| {
                                        source_attack_abilities.contains(&proposal.ability)
                                    })
                                    .cloned(),
                            );
                            summons
                                .consume_report(runtime, table, native, actor, report, reader, now)
                        },
                    )
                    .await;
                let native_actors = native_reports.into_iter()
                    .map(|(actor, _)| actor).collect::<Vec<_>>();
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
                owner
                    .tick_creature_fields(&mut tx, &authority, runtime, states, room, native, now)
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
                physical_spell::execute(
                    runtime, states, attack, room, native, &mut map, &proposals, &parties, now,
                )
                .await;
                drop(map);
                let next_due = next_due.into_iter()
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
        // End the read transaction and semantic pass before the existing durable Item
        // owner opens its own fenced write transaction for the same source occurrence.
        source_items::execute(self, runtime, room, native, permit, &proposals)
            .await
            .map_err(|_| ())?;
        Ok((next_due, native_actors))
    }
}
