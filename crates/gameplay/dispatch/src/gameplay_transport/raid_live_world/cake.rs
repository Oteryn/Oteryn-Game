//! Crystal data-global/scripts/quests/a_piece_of_cake/movements_extraction_spot.lua:
//! 26-49: a live Cake Golem entering an actual machine is extracted, without a kill.
use super::*;
use crate::durability::raid_runner::cake;
use crate::durability::raid_runner::{RaidCreatureIdentity, RaidStepIdentity};
use crate::movement::cake_extraction::{CREATURE, CakeMachineEntry};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn extract_cake_machine_subjects(
        &self,
        firing: [u8; 16],
    ) -> Result<Vec<RaidSpawnIdentity>, ()> {
        self.cake_machine_turn(firing).await
    }
    async fn cake_machine_turn(&self, firing: [u8; 16]) -> Result<Vec<RaidSpawnIdentity>, ()> {
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        let room = self.rooms.gate_monster_ai().ok_or(())?;
        let binding = runtime.binding();
        if binding.world_id() != self.world_id || binding.channel_id() != Some(self.channel_id) {
            return Err(());
        }
        let identity = RaidStepIdentity {
            firing_id: firing,
            channel: self.channel_id,
            generation: binding.scope_generation(),
            ordinal: 0,
        };
        let candidates = runtime
            .raid_creatures(firing)
            .map_err(|_| ())?
            .into_iter()
            .filter_map(|status| {
                let RaidSpawnOutcome::Placed(actor) = status.outcome else {
                    return None;
                };
                runtime
                    .actor_spell_reserved(actor)
                    .then(|| CakeMachineEntry::from_current(&runtime, actor))
                    .flatten()
                    .filter(|entry| entry.identity() == status.identity)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (self, &mut *runtime, &permit, room, candidates);
        pass.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                use crate::durability::spell_item_transaction::commit_creature_item_owner_transaction;
                use crate::durability::spell_item_transaction::{
                    assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                };
                let (owner, runtime, permit, room, candidates) = context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let binding = runtime.binding();
                let authority = assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    binding.runtime_scope(),
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let current =
                    cake::current_source_in_transaction(&mut tx, owner.holder, identity).await?;
                let objects = owner.door.lock().await;
                let mut pending = Vec::new();
                pending
                    .try_reserve_exact(candidates.len())
                    .map_err(|_| DurabilityError::Unavailable)?;
                for source in candidates.iter() {
                    let actor = source.actor();
                    let machine = if current {
                        crate::durability::native_world_read::qualified_creature_source_tile_in_transaction(
                            &mut tx,
                            &authority,
                            room,
                            runtime,
                            &*objects,
                            source.position(),
                        )
                        .await
                        .is_ok_and(|fact| {
                            // Crystal data/items/items.xml:23647-23649 defines
                            // the extraction machine as iron-floor Ground;
                            // movements_extraction_spot.lua:26-49 still requires
                            // the exact current machine position and live Golem.
                            let tile = fact.qualified_tile();
                            tile.ground_source_id() == Some(8886)
                                || tile.top_source_ids().contains(&8886)
                        })
                    } else {
                        false
                    };
                    if !machine && !runtime.actor_spell_reserved(actor) {
                        continue;
                    }
                    let Ok(mut reservation) = runtime.prepare_scope_creature_reservation(source)
                    else {
                        continue;
                    };
                    runtime
                        .reserve_scope_creatures(&mut reservation, source)
                        .map_err(|_| DurabilityError::Unavailable)?;
                    let at = source.position();
                    let position = [
                        u16::try_from(at.x).map_err(|_| DurabilityError::InvalidStoredState)?,
                        u16::try_from(at.y).map_err(|_| DurabilityError::InvalidStoredState)?,
                        u16::try_from(at.floor).map_err(|_| DurabilityError::InvalidStoredState)?,
                    ];
                    let result = if current {
                        cake::extract_in_transaction(
                            &mut tx,
                            owner.holder,
                            identity,
                            RaidCreatureIdentity {
                                wave: source.identity().wave,
                                entry: source.identity().entry,
                                index: source.identity().index,
                            },
                            owner.reset_epoch,
                            position,
                            machine,
                        )
                        .await?
                    } else {
                        crate::durability::raid_schedule::RaidWrite::CompareFailed
                    };
                    pending.push((source, reservation, result));
                }
                // Unknown COMMIT drops local data, but never clears the actual slot's
                // pending_source. The same stable raid identity resumes it next poll.
                commit_creature_item_owner_transaction(tx, deadline).await?;
                drop(objects);
                let mut states = owner.states.spell_states.lock().await;
                let mut extracted = Vec::new();
                extracted
                    .try_reserve_exact(pending.len())
                    .map_err(|_| DurabilityError::Unavailable)?;
                for (source, reservation, result) in pending {
                    runtime
                        .validate_scope_creature_reservation(&reservation, source)
                        .map_err(|_| DurabilityError::Unavailable)?;
                    match result {
                        crate::durability::raid_schedule::RaidWrite::Applied
                        | crate::durability::raid_schedule::RaidWrite::Replayed => {
                            let actor = source.actor();
                            let position = source.position();
                            // Crystal utils_definitions.hpp:87 names CONST_ME_STUN=32.
                            states
                                .sounds
                                .record_item_effect(runtime, actor, position, 32, false);
                            owner
                                .states
                                .chat
                                .publish_cake_extraction_speech(runtime, actor, position);
                            let content = runtime.content_pin().server_artifact_digest();
                            runtime
                                .remove_scope_creature_after_observed_outcome(
                                    reservation,
                                    source,
                                    actor,
                                    CREATURE,
                                    content,
                                )
                                .map_err(|_| DurabilityError::Unavailable)?;
                            extracted.push(source.identity());
                        }
                        crate::durability::raid_schedule::RaidWrite::CompareFailed => runtime
                            .release_scope_creatures_after_observed_outcome(reservation, source)
                            .map_err(|_| DurabilityError::Unavailable)?,
                    }
                }
                Ok(extracted)
            })
        })
        .await
        .map_err(|_| ())
    }
}
