//! Source map harvesting through ordinary object USE and the existing map-item handle.
use super::*;
use crate::durability::item_craft::{MapHarvestOperand, MapHarvestRequest};
use crate::durability::terrain_tool::{MapMutation, TerrainScope};
use crate::map::overlay::TilePos;
use oteryn_gameplay_items::event_planting as fishing;
use oteryn_protocol_oteryn::world_object::{WorldObjectTarget, encode_item_use_intent};

enum Target<'a> {
    Object(&'a WorldObjectTarget),
    Handle(Option<ItemKey>),
}

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_map_harvest_object(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: &WorldObjectTarget,
    ) -> Result<Option<UseOutcome>, ()> {
        let intent = UseIntent {
            target: UseTarget::WorldObject(target.clone()),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        self.use_map_harvest(actor, command, &intent, Target::Object(target))
            .await
    }

    pub(in crate::gameplay_transport) async fn use_map_harvest_handle(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if source
            .is_some_and(|source| !matches!(source, ItemKey::MapBase { .. } | ItemKey::MapAdded(_)))
            || intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
        {
            return Ok(None);
        }
        self.use_map_harvest(actor, command, intent, Target::Handle(source))
            .await
    }

    async fn use_map_harvest(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        target: Target<'_>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(map) = self.bundle_map() else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let mut request = MapHarvestRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            intent: encode_item_use_intent(intent).map_err(|_| ())?,
            operand: None,
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        if let Some(result) = self
            .root
            .harvest_map_item(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native),
            )
            .await
            .map_err(|_| ())?
        {
            drop(permit);
            if result.disposition == UseDisposition::Committed {
                self.refresh_terrain_tools().await;
            }
            return Ok(Some(outcome(result.disposition)));
        }
        let operand = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || pinned_index(&runtime, self.active_generation).is_none()
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, None)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let facts = map.facts.pinned();
            let source = super::super::world_map::MapViewSource {
                overlay: &map.overlay,
                facts: &facts,
                content_generation: runtime.content_pin().client_artifact_digest(),
                reset_epoch: self.reset_epoch,
            };
            let (pos, key, before) = match target {
                Target::Object(target) => {
                    let selected =
                        match super::super::world_map::resolve_map_target(&source, target, |_| {
                            None::<()>
                        }) {
                            Ok(value) => value,
                            Err(_) => return Ok(None),
                        };
                    let tile = view::compose_tile(&map.overlay, &facts, selected.pos)
                        .map_err(|_| ())?
                        .ok_or(())?;
                    let Some(entry) = tile
                        .entries
                        .iter()
                        .find(|e| e.source.key() == MapEntryKey::Base(selected.placement_key))
                    else {
                        return Ok(None);
                    };
                    (selected.pos, selected.placement_key, entry.facts)
                }
                Target::Handle(Some(handle)) => {
                    match super::super::world_map::resolve_map_handle(&source, handle) {
                        Ok(super::super::world_map::MapHandleTarget::Base {
                            pos,
                            placement_key,
                            ..
                        }) => {
                            let tile = view::compose_tile(&map.overlay, &facts, pos)
                                .map_err(|_| ())?
                                .ok_or(())?;
                            let Some(entry) = tile
                                .entries
                                .iter()
                                .find(|e| e.source.key() == MapEntryKey::Base(placement_key))
                            else {
                                return Ok(None);
                            };
                            (pos, placement_key, entry.facts)
                        }
                        Ok(super::super::world_map::MapHandleTarget::Added { pos, id }) => {
                            // Only the boot owner's distinct stable namespace enters this path.
                            // Other movement, dropped and player-grown objects retain their USE owner.
                            let Some((appearance, x, y, z)) = fishing::world_change_item_source(id)
                            else {
                                return Ok(None);
                            };
                            if appearance != 13842
                                || (pos.x, pos.y, pos.floor) != (x, y, -(z as i8))
                            {
                                return Ok(Some(outcome(UseDisposition::Rejected)));
                            }
                            let tile = view::compose_tile(&map.overlay, &facts, pos)
                                .map_err(|_| ())?
                                .ok_or(())?;
                            let Some(entry) = tile.entries.iter().find(|entry|
                                matches!(entry.source,view::EntrySource::Added(key) if key.get()==id)) else { return Ok(None); };
                            (pos, id, entry.facts)
                        }
                        _ => return Ok(None),
                    }
                }
                Target::Handle(None) => return Ok(None),
            };
            let Some(plan) = fishing::map_harvest(before.appearance_id, 1) else {
                return Ok(None);
            };
            let actor_at = runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .position();
            if actor_at.floor != -i16::from(pos.floor)
                || actor_at.x.abs_diff(i32::from(pos.x)) > 1
                || actor_at.y.abs_diff(i32::from(pos.y)) > 1
            {
                return Ok(Some(outcome(UseDisposition::TooFar)));
            }
            if facts.house_tile(pos) || before.bound || before.count != 1 || before.sub_type != 0 {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            }
            let revision = facts.object_revision(key);
            if !map.facts.quest_callback_capacity(key, revision, 0) {
                return Ok(Some(outcome(UseDisposition::Exhausted)));
            }
            for appearance in std::iter::once(plan.after)
                .chain(plan.decay.and_then(|(id, _)| u16::try_from(id).ok()))
            {
                if map
                    .facts
                    .quest_callback_replacement(key, appearance)
                    .is_none()
                {
                    return Ok(Some(outcome(UseDisposition::NotPossible)));
                }
            }
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let actor_pos = TilePos {
                x: u16::try_from(actor_at.x).map_err(|_| ())?,
                y: u16::try_from(actor_at.y).map_err(|_| ())?,
                floor: i8::try_from(actor_at.floor)
                    .map_err(|_| ())?
                    .checked_neg()
                    .ok_or(())?,
            };
            MapHarvestOperand {
                scope: TerrainScope {
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_generation: native.source_digest(),
                    reset_epoch: i64::try_from(self.reset_epoch).map_err(|_| ())?,
                },
                mutation: MapMutation {
                    placement,
                    expected_revision: i64::try_from(revision).map_err(|_| ())?,
                    before: u32::from(before.appearance_id),
                    after: u32::from(plan.after),
                    decay: plan.decay.into_iter().collect(),
                },
                actor_position: spatial(actor_pos),
                source_position: spatial(pos),
            }
        };
        request.operand = Some(operand);
        let result = self
            .root
            .harvest_map_item(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &NativeItemWeights(native),
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        drop(permit);
        if result.disposition == UseDisposition::Committed {
            self.states.loop_ground.invalidate();
            self.refresh_terrain_tools().await;
        }
        Ok(Some(outcome(result.disposition)))
    }
}

fn spatial(pos: TilePos) -> Vec<u8> {
    crate::durability::map_item_mint_audit::map_item_spatial_position(pos.x, pos.y, pos.floor)
}
