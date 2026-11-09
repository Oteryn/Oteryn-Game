//! M52 map timers run independently of announcement delivery.
use super::*;
use crate::durability::raid_runner::RaidStepIdentity;
use crate::map::view::EntryFacts;
use oteryn_protocol_oteryn::world_map::MapDefinition;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn apply_feroxa_map_clock(
        &self,
        firing: [u8; 16],
        now_ms: i64,
    ) -> Result<(), ()> {
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let runtime = self.runtime.lock().await;
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
        let room = self.rooms.gate_monster_ai().ok_or(())?;
        let native = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or(())?;
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (self, &*runtime, &permit, room, native);
        pass.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                use crate::durability::spell_item_transaction::{
                    assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                };
                let (owner, runtime, permit, room, native) = context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let binding = runtime.binding();
                let _authority = assert_spell_item_scope_in_transaction(
                    &mut tx, permit, owner.root, owner.character, owner.holder,
                    binding.runtime_scope(), binding.scope_generation().get(),
                ).await.map_err(|_| DurabilityError::Unavailable)?;
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index()
                else { return Err(DurabilityError::Unavailable); };
                let elapsed = crate::durability::raid_runner::feroxa_map_elapsed_in_transaction(
                    &mut tx, owner.holder, identity, owner.world_id, owner.reset_epoch,
                    index.map().overlay.base().digest(), now_ms,
                ).await?;
                let maps = crate::content::feroxa_source_maps::FeroxaSourceMaps::embedded()
                    .ok_or(DurabilityError::Unavailable)?;
                if maps.due(elapsed).is_none() {
                    tx.rollback().await?;
                    return Ok(());
                }
                if !crate::durability::world_event::changes::callbacks::require(
                    &mut tx, owner.world_id, owner.reset_epoch,
                    "oteryn:quest-callback/grimvale-full-moon-entry",
                ).await? { return Err(DurabilityError::Unavailable); }
                let item_index = native.item_index().ok_or(DurabilityError::Unavailable)?;
                let map = index.map();
                let (phase, source, tiles) = crate::map::facts::prepare_feroxa_phase(elapsed, |id| {
                    map.facts.movement_item_facts(id).or_else(|| Some(EntryFacts {
                        definition: MapDefinition::Item(item_index.definition_ref(
                            "Item", &format!("oteryn:item.tibia.i{id}"), "definition-r1",
                        )?),
                        terrain_kind: None, appearance_id: id, blocks_projectile: false,
                        pickupable: false, bound: false, count: 1, sub_type: 0,
                    }))
                }).ok_or(DurabilityError::Unavailable)?;
                // An ACKed final wave already ejected its original spectators.
                // Recover its open entry without repeating those actor effects.
                let entry_open: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND action_kind=2 AND state=2)")
                    .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice())
                    .fetch_one(&mut *tx).await?;
                let objects = owner.door.lock().await;
                if !map.publish_feroxa_phase(firing, phase, source, tiles)
                    || (entry_open && !project_grimvale_portal(map, true))
                { return Err(DurabilityError::Unavailable); }
                drop(objects);
                // Only current durable reads; publication happened while every
                // World/Channel/firing/physical Item guard was still retained.
                tx.rollback().await?;
                Ok(())
            })
        }).await.map_err(|_| ())
    }
}
