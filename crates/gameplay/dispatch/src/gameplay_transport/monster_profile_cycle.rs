//! Channel composition: current Item/Map reads, one native think and synchronous summons.
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates};
use crate::ai_think::summon_plan::summon_path::NativeMonsterThinkRead;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::ChannelRuntimeV1;
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_simulation_determinism::GameplayDecisionRoot;

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
    ) -> Result<Option<u64>, ()> {
        use crate::durability::spell_item_transaction::{
            assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
        };
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (
            self, runtime, states, attack, room, native, permit, revisions, candidates,
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
                let summons = &mut states.monster_summons;
                table
                    .run_due_current_thinks(
                        runtime,
                        now,
                        &root,
                        revisions,
                        candidates,
                        &mut reader,
                        |runtime, table, actor, report, reader| {
                            summons
                                .consume_report(runtime, table, native, actor, report, reader, now)
                        },
                    )
                    .await;
                let next_step = table
                    .drive_current_movement(
                        runtime,
                        room.movement_cells(),
                        objects.collision_cells(),
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
                tx.rollback()
                    .await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                Ok(table.earliest_due().into_iter().chain(next_step).min())
            })
        })
        .await
        .map_err(|_| ())
    }
}
