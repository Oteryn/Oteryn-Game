//! CRAFT-1: existing bounded USE/use-with → current map station → atomic item owner.
use super::ComposedFreshAdmission;
use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, pinned_index};
use super::item_view::{ItemKey, SessionItemView};
use crate::durability::item_craft::ItemCraftRequest;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::view::{self, MapEntryKey, MapFacts};
use oteryn_gameplay_progression::hit_training::{FISHING_TRIES_PER_USE, SkillFamily};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(super) mod enchanting;
mod rope;
mod terrain;

pub(super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    intent: UseIntent,
    view: Option<&SessionItemView>,
) -> Option<UseOutcome> {
    let source = match intent.target {
        UseTarget::Item(handle) => view.and_then(|v| v.resolve(handle)),
        _ => None,
    };
    let target = match &intent.use_with_map_item {
        Some(MapUseTarget::Item(handle)) => view.and_then(|v| v.resolve(*handle)),
        _ => None,
    };
    authority
        .use_craft(actor, command, intent, source, target)
        .await
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_cooking_conversion(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        if let Some(result) = self
            .use_enchanting(actor, command, &intent, source, target)
            .await
        {
            return result;
        }
        match self
            .use_terrain_tool(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        match self
            .use_rope_tool(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        let disposition = |d| {
            Some(UseOutcome {
                disposition: d,
                ..UseOutcome::rejected()
            })
        };
        let fence = command.item_fence?;
        let id = CommandId::new(command.command_id).ok()?;
        let native = self.active_generation?.native_gameplay()?;
        let _permit = self.spell_lane_permit().await?;
        let mut request = ItemCraftRequest {
            command: CommandRef::new(command.game_session_id, id),
            source: [0; 16],
            tool: None,
            gathering: None,
            enchanting: None,
            station: None,
            intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent).ok()?,
        };
        // Replay precedes current handles and custody, including input retirement and reconnect.
        if let Some(result) = self
            .root
            .craft_item(
                &_permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native),
                true,
            )
            .await
            .ok()?
        {
            return disposition(result);
        }
        let Some(ItemKey::Instance(source)) = source else {
            return disposition(UseDisposition::StaleState);
        };
        request.source = source;
        let (station, map_target, in_sewers) = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return disposition(UseDisposition::Rejected);
            }
            let index = pinned_index(&runtime, self.active_generation)?;
            let at = super::bundle_map_position(&mut runtime, self.movement_cells, actor)?;
            let facts = at.map.facts.pinned();
            let map = super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let resolved = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    super::world_map::resolve_map_target(&map, object, |_| None::<()>)
                        .ok()
                        .map(|t| (t.pos, MapEntryKey::Base(t.placement_key)))
                }
                Some(MapUseTarget::Item(_)) => target
                    .and_then(|key| super::world_map::resolve_map_handle(&map, key).ok())
                    .and_then(|t| match t {
                        super::world_map::MapHandleTarget::Base {
                            pos, placement_key, ..
                        } => Some((pos, MapEntryKey::Base(placement_key))),
                        _ => None,
                    }),
                None => None,
            };
            let Some((pos, key)) = resolved else {
                return disposition(UseDisposition::StaleState);
            };
            if i16::from(pos.floor) != at.at.floor
                || at.at.x.abs_diff(i32::from(pos.x)) > 1
                || at.at.y.abs_diff(i32::from(pos.y)) > 1
            {
                return disposition(UseDisposition::TooFar);
            }
            let tile = view::compose_tile(&at.map.overlay, &facts, pos).ok()??;
            let entry = tile.entries.iter().find(|e| e.source.key() == key)?;
            let MapDefinition::Item(reference) = entry.facts.definition else {
                return disposition(UseDisposition::NothingToUse);
            };
            // Crystal mechanical_fishing.lua:90-97,203: the sewers box is tested on the target tile.
            let in_sewers =
                oteryn_gameplay_items::fishing::in_yalahar_sewers(pos.x, pos.y, pos.floor);
            let map_target = (
                key,
                facts.object_revision(match key {
                    MapEntryKey::Base(k) => k,
                    _ => return None,
                }),
                entry.facts.bound
                    || facts.house_tile(pos)
                    || entry.facts.count != 1
                    || entry.facts.sub_type != 0,
            );
            (
                super::item_look::typed_definition(index, reference)?,
                map_target,
                in_sewers,
            )
        };
        request.station = Some(station);
        let mut catch_roll = 1;
        let mut nail_catch = false;
        let mut fishing_skill = 10;
        // Fishing is the same durable consume/mint owner as cooking, but keeps the rod and
        // consumes a worm only after the skill-scaled catch succeeds.
        if request
            .station
            .as_ref()
            .is_some_and(|s| oteryn_gameplay_items::fishing::is_water(&s.production_key))
        {
            let inventory = self
                .root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
                .ok()?;
            let entries = inventory
                .backpack
                .iter()
                .flat_map(|b| &b.entries)
                .chain(inventory.nested.iter().map(|(_, entry)| entry));
            let water_key = &request.station.as_ref()?.production_key;
            let find_bait = |key: &str| {
                entries
                    .clone()
                    .find(|entry| entry.item.definition.production_key == key)
                    .map(|entry| entry.item.item_instance_id)
            };
            // Crystal mechanical_fishing.lua:203-248: a nail in the Yalahar sewers selects the
            // mechanical-fish catch; otherwise the ordinary worm rules apply.
            nail_catch = in_sewers
                && find_bait(oteryn_gameplay_items::fishing::NAIL).is_some()
                && entries.clone().any(|entry| {
                    entry.item.item_instance_id == source
                        && oteryn_gameplay_items::fishing::accepts_nail_rod(
                            &entry.item.definition.production_key,
                            water_key,
                        )
                });
            let rod = entries
                .clone()
                .find(|entry| {
                    entry.item.item_instance_id == source
                        && (nail_catch
                            || oteryn_gameplay_items::fishing::accepts_fishing_rod(
                                &entry.item.definition.production_key,
                                water_key,
                            ))
                })?
                .item
                .item_instance_id;
            let worm = find_bait(if nail_catch {
                oteryn_gameplay_items::fishing::NAIL
            } else {
                oteryn_gameplay_items::fishing::WORM
            });
            let now = u64::try_from(self.clock_origin.elapsed().as_micros()).unwrap_or(u64::MAX);
            let skill = {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                states
                    .get(&runtime, actor, command.game_session_id)
                    .and_then(|state| state.live_skill(SkillFamily::Fishing.index(), now))
                    .and_then(|value| u16::try_from(value).ok())
                    .unwrap_or(10)
            };
            let digest = Sha256::new()
                .chain_update(b"oteryn.fishing.catch.v1")
                .chain_update(command.game_session_id.as_bytes())
                .chain_update(command.command_id.to_be_bytes())
                .chain_update(&request.intent)
                .finalize();
            let mut occurrence = [0; 16];
            occurrence.copy_from_slice(&digest[..16]);
            let draw = deterministic_decision_u64(
                &GameplayDecisionRoot::from_bytes(native.source_digest()),
                DecisionOccurrenceId::from_bytes(occurrence),
                "fishing.catch",
                0,
            )
            .ok()?;
            let roll = u32::try_from(crate::spell::uniform_draw(draw, 1, 100_000)).ok()?;
            catch_roll = roll;
            fishing_skill = skill;
            let mut due = Vec::new();
            {
                let runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                super::attack::skill_training::train_hit(
                    &mut states,
                    &runtime,
                    native.training_formula(),
                    actor,
                    command.game_session_id,
                    SkillFamily::Fishing,
                    now,
                    |_| FISHING_TRIES_PER_USE,
                    &mut due,
                );
            }
            self.save_trained_levels(due).await;
            if !oteryn_gameplay_items::fishing::catches(skill, roll) || worm.is_none() {
                return disposition(UseDisposition::NothingToUse);
            }
            request.tool = Some(rod);
            request.source = worm?;
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let tool = inventory
            .backpack
            .iter()
            .flat_map(|b| &b.entries)
            .find(|entry| entry.item.item_instance_id == source);
        let target_id = request
            .station
            .as_ref()
            .and_then(|s| item_id(&s.production_key));
        let planned = match (tool, target_id) {
            (Some(tool), Some(target_id)) => {
                let tool_key = &tool.item.definition.production_key;
                if let Some(tool_id) = item_id(tool_key)
                    && (nail_catch
                        || oteryn_gameplay_items::fishing::gathering_rule(tool_id, target_id)
                            .is_some())
                {
                    let attempt = if nail_catch {
                        oteryn_gameplay_items::fishing::nail_attempt(
                            target_id,
                            fishing_skill,
                            catch_roll,
                        )?
                    } else {
                        oteryn_gameplay_items::fishing::gathering_attempt(
                            tool_id,
                            target_id,
                            fishing_skill,
                            catch_roll,
                            gathering_roll(
                                native.source_digest(),
                                command.game_session_id,
                                command.command_id,
                                &request.intent,
                            )?,
                        )?
                    };
                    // Crystal items.xml:18875-18881: only spent hole 7237 decays;
                    // the open 7236 made by a pick has no duration/decayTo.
                    let decay = if attempt.after == Some(7237) {
                        vec![(7200, 900_000)]
                    } else if let Some(fishable) = attempt
                        .after
                        .and_then(oteryn_gameplay_items::fishing::nail_spent_water_decays_to)
                    {
                        // Crystal items.xml:13998-14004: spent nail water decays after 1200 s.
                        vec![(
                            u32::from(fishable),
                            oteryn_gameplay_items::fishing::NAIL_WATER_DECAY_MS,
                        )]
                    } else {
                        Vec::new()
                    };
                    Some((tool, target_id, attempt, decay))
                } else {
                    // M08: scythe and sickle harvests, with the decay chain of their catalogue.
                    oteryn_gameplay_world::terrain_tools::harvest_plan(
                        tool_key,
                        u32::from(target_id),
                    )
                    .map(|plan| (tool, target_id, plan.attempt, plan.decay))
                }
            }
            _ => None,
        };
        if let Some((tool, target_id, attempt, decay)) = planned {
            let (MapEntryKey::Base(key), revision, invalid) = map_target else {
                return None;
            };
            if invalid {
                return disposition(UseDisposition::NothingToUse);
            }
            let map = self.bundle_map()?;
            if map.facts.terrain_tool_action(key) != Some(None)
                || !map.facts.quest_callback_capacity(key, revision, 0)
            {
                return disposition(UseDisposition::NothingToUse);
            }
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            request.gathering = Some(crate::durability::item_craft::GatheringCraft {
                scope: crate::durability::terrain_tool::TerrainScope {
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_generation: native.source_digest(),
                    reset_epoch: i64::try_from(self.reset_epoch).ok()?,
                },
                mutation: crate::durability::terrain_tool::TerrainMutation {
                    tool: source,
                    tool_key: tool.item.definition.production_key.clone(),
                    tool_revision: tool.item.definition.revision_ref.clone(),
                    placement,
                    expected_revision: i64::try_from(revision).ok()?,
                    before: u32::from(target_id),
                    after: attempt.after.map_or(0, u32::from),
                    decay,
                },
                attempt,
            });
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(source))
        {
            return disposition(UseDisposition::Rejected);
        }
        let result = self
            .root
            .craft_item(
                &_permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &NativeItemWeights(native),
                false,
            )
            .await
            .ok()??;
        drop(_permit);
        if result == UseDisposition::Committed {
            self.refresh_terrain_tools().await;
        }
        disposition(result)
    }
}

fn item_id(key: &str) -> Option<u16> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}

pub(in crate::gameplay_transport) fn gathering_roll(
    digest: [u8; 32],
    session: crate::foundation::GameSessionId,
    command_id: u64,
    intent: &[u8],
) -> Option<u32> {
    let digest = Sha256::new()
        .chain_update(b"oteryn.gathering.yield.v1")
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .chain_update(intent)
        .chain_update(digest)
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let draw = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(digest.into()),
        DecisionOccurrenceId::from_bytes(occurrence),
        "gathering.yield",
        0,
    )
    .ok()?;
    u32::try_from(crate::spell::uniform_draw(draw, 1, 100_000)).ok()
}

#[cfg(test)]
#[path = "item_craft_crystal_parity_tests.rs"]
mod crystal_parity_tests;
