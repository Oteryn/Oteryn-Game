//! CRAFT-1: existing bounded USE/use-with → current map station → atomic item owner.
use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, pinned_index};
use super::item_view::{ItemKey, SessionItemView};
use super::{AdmissionRefusal, AdmissionRetryBudget, ComposedFreshAdmission};
use crate::durability::DurabilityError;
use crate::durability::item_craft::{
    CRUSHER_KEY, GroundSculpture, ItemCraftRequest, SculptureGroundTarget,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::ItemUseError;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::view::{self, MapEntryKey, MapFacts};
use oteryn_gameplay_progression::hit_training::{FISHING_TRIES_PER_USE, SkillFamily};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

mod dispenser;
#[cfg(test)]
pub(in crate::gameplay_transport) use dispenser::DISPENSER_ACK_BARRIER;
pub(super) mod enchanting;
mod harvest;
mod planting;
pub(super) mod rope;
mod skinning;
mod terrain;

#[cfg(test)]
tokio::task_local! {
    pub(super) static CRAFT_RECEIPT_BARRIER: std::sync::Arc<tokio::sync::Barrier>;
}

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

pub(super) async fn retry_item_use_holder<T>(
    retry: &AdmissionRetryBudget<'_>,
    mut step: impl AsyncFnMut() -> Result<T, ItemUseError>,
) -> Result<T, ItemUseError> {
    retry_busy(retry, async || match step().await {
        Err(ItemUseError::Unavailable(error)) => Err(error),
        result => Ok(result),
    })
    .await
    .unwrap_or_else(|error| Err(ItemUseError::Unavailable(error)))
}

// These calls acquire the holder before issuing the transaction. Only retry an
// unavailable checkout here: a deadline on a live pass may interrupt COMMIT.
async fn retry_busy<T>(
    retry: &AdmissionRetryBudget<'_>,
    mut operation: impl AsyncFnMut() -> Result<T, DurabilityError>,
) -> Result<T, DurabilityError> {
    loop {
        let result = operation().await;
        if !matches!(result, Err(DurabilityError::RootUnavailable))
            || !retry
                .retries_unavailable(AdmissionRefusal::Unavailable)
                .await
        {
            return result;
        }
    }
}

fn write_disposition(
    result: Result<Option<UseDisposition>, DurabilityError>,
) -> Result<UseDisposition, ()> {
    match result {
        Ok(Some(disposition)) => Ok(disposition),
        Err(
            DurabilityError::Database(_)
            | DurabilityError::CommitOutcomeUnknown
            | DurabilityError::RootPassDeadlineExceeded,
        ) => {
            // A database failure during the mutation is not a gameplay refusal.
            // Keep the USE reserved for receipt reconciliation.
            Err(())
        }
        Ok(None) | Err(_) => Ok(UseDisposition::Rejected),
    }
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
        if matches!(
            intent.use_with_map_item.as_ref(),
            Some(MapUseTarget::Item(_))
        ) {
            match self
                .use_carried_quest_item_targeted(actor, command, &intent, source, target)
                .await
            {
                Ok(Some(outcome)) => return Some(outcome),
                Ok(None) => {}
                Err(()) => return None,
            }
        }
        match self
            .use_carried_quest_recipe(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        if let Some(outcome) = self
            .use_shared_callback_handle(actor, command, source, target)
            .await
        {
            return Some(outcome);
        }
        match self
            .use_dispenser(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        match self
            .use_skinning(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        match self
            .use_planting(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        self.mark_use_receipt(command, Some(super::session_ingress::UseReceipt::Craft));
        if let Some(result) = self
            .use_enchanting(actor, command, &intent, source, target)
            .await
        {
            return result;
        }
        self.mark_use_receipt(command, None);
        match self
            .use_soulpit_obelisk(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        match self
            .use_soul_core(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        self.mark_use_receipt(command, None);
        match self
            .use_rope_tool(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(rope::RopeUseError::Preflight) => return Some(UseOutcome::rejected()),
            Err(rope::RopeUseError::OutcomeUnknown) => return None,
        }
        match self
            .use_terrain_tool(actor, command, &intent, source, target)
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
        let (Some(fence), Ok(id), Some(native)) = (
            command.item_fence,
            CommandId::new(command.command_id),
            self.active_generation
                .and_then(crate::content::ActiveGeneration::native_gameplay),
        ) else {
            return disposition(UseDisposition::Rejected);
        };
        self.mark_use_receipt(command, Some(super::session_ingress::UseReceipt::Craft));
        let permit = self.spell_lane_permit().await?;
        let Ok(encoded) = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
        else {
            return disposition(UseDisposition::Rejected);
        };
        let retry = AdmissionRetryBudget::for_command(self.root);
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let mut request = ItemCraftRequest {
            command: CommandRef::new(command.game_session_id, id),
            source: [0; 16],
            tool: None,
            ingredient: None,
            gathering: None,
            enchanting: None,
            station: None,
            intent: encoded,
        };
        // Replay precedes current handles and custody, including input retirement and reconnect.
        #[cfg(test)]
        if let Ok(barrier) = CRAFT_RECEIPT_BARRIER.try_with(std::sync::Arc::clone) {
            barrier.wait().await;
            barrier.wait().await;
        }
        match retry
            .retry_command_guard(async || {
                self.root
                    .craft_item(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        request.clone(),
                        native,
                        &weights,
                        true,
                    )
                    .await
            })
            .await
        {
            Ok(Some(result)) => {
                if result == UseDisposition::Committed {
                    self.states.loop_ground.invalidate();
                }
                return disposition(result);
            }
            Ok(None) => {}
            // A failed receipt lookup leaves the original command eligible for replay.
            Err(_) => return None,
        }
        let Some(ItemKey::Instance(source)) = source else {
            return disposition(UseDisposition::StaleState);
        };
        request.source = source;
        let mut sculpture_ground = None;
        // Crystal 201189f8 data-global/scripts/actions/other/baking.lua:8-37,49-51 uses a
        // second carried ingredient for dough, chocolate/garlic dough, trays and sugar oat.
        // Resolve its exact current definition here; the durable owner locks and consumes it.
        if let Some(ItemKey::Instance(ingredient)) = target {
            // I11's carried second-ingredient branch is an action in Crystal's baking handler;
            // unlike the other craft owners above, it must not run while dead or casting.
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !carried_ingredient_action_allowed(
                states.is_dead(actor),
                states.spell_idle(&runtime, actor, command.game_session_id),
            ) {
                return disposition(UseDisposition::Rejected);
            }
            drop(states);
            drop(runtime);
            let Ok(inventory) = retry_item_use_holder(&retry, async || {
                self.root
                    .read_item_use_inventory(self.character, self.holder, fence)
                    .await
            })
            .await
            else {
                return disposition(UseDisposition::Rejected);
            };
            let mut carried = inventory
                .backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .chain(inventory.nested.iter().map(|(_, entry)| entry));
            let entry = carried
                .clone()
                .find(|entry| entry.item.item_instance_id == ingredient);
            let crusher = carried.find(|entry| {
                entry.item.item_instance_id == source
                    && entry.item.definition.production_key == CRUSHER_KEY
            });
            if let Some(crusher) = crusher {
                if !matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
                    || intent.use_with.is_some()
                    || intent.use_with_position.is_some()
                {
                    return disposition(UseDisposition::NothingToUse);
                }
                if entry.is_none() {
                    return disposition(UseDisposition::StaleState);
                }
                request.source = ingredient;
                request.tool = Some(source);
                request.station = Some(crusher.item.definition.clone());
            } else if let Some(entry) = entry {
                request.ingredient = Some(ingredient);
                request.station = Some(entry.item.definition.clone());
            } else {
                let Some(placement) = sculpture_tile(self, actor).await else {
                    return disposition(UseDisposition::Rejected);
                };
                let (Some(index), Some(map)) = (native.item_index(), self.bundle_map()) else {
                    return disposition(UseDisposition::Rejected);
                };
                let Some(ground) =
                    super::loop_move::loop_ground(self, map.overlay.base(), Some(index)).await
                else {
                    return disposition(UseDisposition::Rejected);
                };
                let Some((pos, _)) = ground.ground_entry(&ingredient) else {
                    return disposition(UseDisposition::Rejected);
                };
                let facts = map.facts.pinned();
                let facts = ground.facts(&facts);
                let tile = view::compose_tile(ground.overlay(), &facts, pos).ok()??;
                let entry = tile
                    .entries
                    .iter()
                    .find(|entry| entry.source.key() == MapEntryKey::Ground(ingredient))?;
                let MapDefinition::Item(reference) = entry.facts.definition else {
                    return None;
                };
                let definition = super::item_look::typed_definition(index, reference)?;
                let tool = inventory
                    .backpack
                    .iter()
                    .flat_map(|b| &b.entries)
                    .chain(inventory.nested.iter().map(|(_, entry)| entry))
                    .find(|entry| entry.item.item_instance_id == source)?;
                if !item_id(&tool.item.definition.production_key)
                    .zip(item_id(&definition.production_key))
                    .is_some_and(|(tool, target)| {
                        oteryn_gameplay_items::fishing::is_sculpture(tool, target)
                    })
                {
                    return None;
                }
                let actor_cell = crate::durability::spell_items_abi::decode_ground_cell(
                    &placement.spatial_position,
                )
                .ok()?;
                if actor_cell.z != i32::from(pos.floor)
                    || actor_cell.x.abs_diff(i32::from(pos.x)) > 1
                    || actor_cell.y.abs_diff(i32::from(pos.y)) > 1
                {
                    return disposition(UseDisposition::TooFar);
                }
                if entry.facts.bound
                    || facts.house_tile(pos)
                    || entry.facts.count != 1
                    || entry.facts.sub_type != 0
                {
                    return disposition(UseDisposition::NothingToUse);
                }
                sculpture_ground = Some(GroundSculpture {
                    actor: placement,
                    target: Some(SculptureGroundTarget {
                        position: ground.pickup_cell(&ingredient)?,
                        ordinal: ground.ground_ordinal(&ingredient)?,
                    }),
                });
                request.ingredient = Some(ingredient);
                request.station = Some(definition);
            }
        }
        let (station, map_target) = {
            if let Some(station) = request.station.clone() {
                (station, None)
            } else {
                let mut runtime = self.runtime.lock().await;
                let (Some(index), Some(at)) = (
                    pinned_index(&runtime, self.active_generation),
                    super::bundle_map_position(&mut runtime, self.movement_cells(), actor),
                ) else {
                    return disposition(UseDisposition::Rejected);
                };
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
                let Ok(Some(tile)) = view::compose_tile(&at.map.overlay, &facts, pos) else {
                    return disposition(UseDisposition::Rejected);
                };
                let Some(entry) = tile.entries.iter().find(|e| e.source.key() == key) else {
                    return disposition(UseDisposition::Rejected);
                };
                let MapDefinition::Item(reference) = entry.facts.definition else {
                    return disposition(UseDisposition::NothingToUse);
                };
                let map_target = (
                    key,
                    facts.object_revision(match key {
                        MapEntryKey::Base(k) => k,
                        _ => return disposition(UseDisposition::Rejected),
                    }),
                    entry.facts.bound
                        || facts.house_tile(pos)
                        || entry.facts.count != 1
                        || entry.facts.sub_type != 0,
                    pos,
                );
                let Some(definition) = super::item_look::typed_definition(index, reference) else {
                    return disposition(UseDisposition::Rejected);
                };
                (definition, Some(map_target))
            }
        };
        request.station = Some(station);
        let mut catch_roll = 1;
        let mut fishing_skill = 10;
        // Fishing is the same durable consume/mint owner as cooking, but keeps the rod and
        // consumes a worm only after the skill-scaled catch succeeds.
        if let Some((_, _, _, target_pos)) = map_target
            && request
                .station
                .as_ref()
                .is_some_and(|s| oteryn_gameplay_items::fishing::is_water(&s.production_key))
        {
            let Ok(inventory) = retry_item_use_holder(&retry, async || {
                self.root
                    .read_item_use_inventory(self.character, self.holder, fence)
                    .await
            })
            .await
            else {
                return disposition(UseDisposition::Rejected);
            };
            let entries = inventory
                .backpack
                .iter()
                .flat_map(|b| &b.entries)
                .chain(inventory.nested.iter().map(|(_, entry)| entry));
            let water_key = &request.station.as_ref()?.production_key;
            let target_id = item_id(water_key)?;
            let mechanical_sewer = oteryn_gameplay_items::fishing::is_yalahar_sewer_water(
                target_id,
                target_pos.x,
                target_pos.y,
                target_pos.floor,
            );
            let rod = entries.clone().find(|entry| {
                entry.item.item_instance_id == source
                    && oteryn_gameplay_items::fishing::accepts_fishing_rod(
                        &entry.item.definition.production_key,
                        water_key,
                    )
                    || (entry.item.item_instance_id == source
                        && mechanical_sewer
                        && entry.item.definition.production_key == "oteryn:item.tibia.i9306")
            });
            let Some(rod) = rod else {
                return disposition(UseDisposition::NothingToUse);
            };
            let rod = rod.item.item_instance_id;
            let worm = entries
                .clone()
                .find(|entry| {
                    entry.item.definition.production_key == oteryn_gameplay_items::fishing::WORM
                })
                .map(|entry| entry.item.item_instance_id);
            let nail = entries
                .clone()
                .find(|entry| {
                    entry.item.definition.production_key == oteryn_gameplay_items::fishing::NAIL
                })
                .map(|entry| entry.item.item_instance_id);
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
            let consumable = if mechanical_sewer { nail } else { worm };
            if !oteryn_gameplay_items::fishing::catches(skill, roll) || consumable.is_none() {
                return disposition(UseDisposition::NothingToUse);
            }
            request.tool = Some(rod);
            request.source = consumable?;
        }
        let Ok(inventory) = retry_item_use_holder(&retry, async || {
            self.root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
        })
        .await
        else {
            return disposition(UseDisposition::Rejected);
        };
        let tool = inventory
            .backpack
            .iter()
            .flat_map(|b| &b.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .find(|entry| entry.item.item_instance_id == source);
        let target_id = request
            .station
            .as_ref()
            .and_then(|s| item_id(&s.production_key));
        let tool_id = tool.and_then(|entry| item_id(&entry.item.definition.production_key));
        if let Some((MapEntryKey::Base(key), revision, invalid, _)) = map_target
            && let (Some(tool), Some(tool_id), Some(target_id)) = (tool, tool_id, target_id)
            && let Some(attempt) = oteryn_gameplay_items::event_planting::planting_attempt(
                tool_id, target_id,
            )
            .or_else(|| {
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
                )
            })
        {
            let seed_attempt =
                oteryn_gameplay_items::event_planting::planting_attempt(tool_id, target_id);
            if invalid {
                return disposition(UseDisposition::NothingToUse);
            }
            let map = self.bundle_map()?;
            if map.facts.terrain_tool_action(key) != Some(None)
                || !map.facts.quest_callback_capacity(key, revision, 0)
            {
                return disposition(UseDisposition::NothingToUse);
            }
            if seed_attempt.is_some() {
                let after = attempt.after?;
                let appearances = std::iter::once(u32::from(after)).chain(
                    oteryn_gameplay_items::event_planting::planting_decays(after)
                        .into_iter()
                        .map(|(state, _)| state),
                );
                for appearance in appearances {
                    let appearance = u16::try_from(appearance).ok()?;
                    if map
                        .facts
                        .quest_callback_replacement(key, appearance)
                        .is_none()
                    {
                        return disposition(UseDisposition::NotPossible);
                    }
                }
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
                    // Crystal items.xml:18875-18881: only spent hole 7237 decays;
                    // the open 7236 made by a pick has no duration/decayTo.
                    decay: if attempt.after == Some(7237) {
                        vec![(7200, 900_000)]
                    } else if tool_id == 9306 && attempt.after == Some(4609) {
                        // Crystal items.xml:13998-14005: spent sewer water recovers in 20 min.
                        vec![(4597, 1_200_000)]
                    } else if seed_attempt.is_some() {
                        oteryn_gameplay_items::event_planting::planting_decays(attempt.after?)
                    } else {
                        Vec::new()
                    },
                },
                attempt,
            });
        }
        let terrain_marble_yield = request
            .gathering
            .as_ref()
            .is_some_and(|g| g.mutation.before == 10426 && g.attempt.yield_item.is_some());
        let carried_marble_yield = sculpture_ground.is_none()
            && request.ingredient.is_some()
            && target_id == Some(10426)
            && tool_id.is_some_and(|tool| {
                gathering_roll(
                    native.source_digest(),
                    command.game_session_id,
                    command.command_id,
                    &request.intent,
                )
                .and_then(|roll| {
                    oteryn_gameplay_items::fishing::gathering_attempt(tool, 10426, 10, 1, roll)
                })
                .is_some_and(|attempt| attempt.yield_item.is_some())
            });
        if terrain_marble_yield || carried_marble_yield {
            let placement = sculpture_tile(self, actor).await;
            if terrain_marble_yield && placement.is_none() {
                return None;
            }
            sculpture_ground = placement.map(|actor| GroundSculpture {
                actor,
                target: None,
            });
        }
        if !self.states.player_trades.physical_items(
            command.game_session_id,
            [
                Some(source),
                request.ingredient,
                request.tool,
                Some(request.source),
            ],
        ) {
            return disposition(UseDisposition::Rejected);
        }
        // Invalidate even when the write's reply is lost or the task is
        // cancelled. The next projection waits for the physical owner.
        let ground_change = sculpture_ground
            .as_ref()
            .map(|_| self.states.loop_ground.sculpture_change());
        let result = retry_busy(&retry, async || {
            if let Some(ground) = sculpture_ground.clone() {
                self.root
                    .craft_item_on_ground(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        request.clone(),
                        native,
                        &weights,
                        ground,
                    )
                    .await
            } else {
                self.root
                    .craft_item(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        request.clone(),
                        native,
                        &weights,
                        false,
                    )
                    .await
            }
        })
        .await;
        let result = write_disposition(result).ok()?;
        drop(ground_change);
        drop(permit);
        if result == UseDisposition::Committed {
            self.refresh_terrain_tools().await;
        }
        disposition(result)
    }
}

async fn sculpture_tile(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> Option<crate::durability::item_mint::GroundPlacement> {
    let mut runtime = admission.runtime.lock().await;
    let at = super::bundle_map_position(&mut runtime, admission.movement_cells(), actor)?;
    pinned_index(&runtime, admission.active_generation)?;
    let pin = runtime.content_pin();
    let position = crate::durability::map_item_mint_audit::map_item_spatial_position(
        u16::try_from(at.at.x).ok()?,
        u16::try_from(at.at.y).ok()?,
        i8::try_from(at.at.floor).ok()?,
    );
    Some(crate::durability::item_mint::GroundPlacement {
        spatial_position: position,
        corpse_ref: b"sculpture-craft-v1".to_vec(),
        map_revision: crate::map::overlay::map_revision(at.map.overlay.base()),
        content_revision: format!(
            "sha256:{}",
            pin.server_artifact_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        native_room_placement_context: pin.frame_binding_digest().to_vec(),
    })
}

/// Decision used by the real I11 two-carried-item baking handler.
pub(super) fn carried_ingredient_action_allowed(is_dead: bool, spell_idle: bool) -> bool {
    !is_dead && spell_idle
}

fn item_id(key: &str) -> Option<u16> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}

pub(in crate::gameplay_transport) use crate::durability::item_craft::gathering_roll;

#[cfg(test)]
#[path = "item_craft_crystal_parity_tests.rs"]
mod crystal_parity_tests;
