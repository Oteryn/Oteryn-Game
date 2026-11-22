//! CRAFT-1: existing bounded USE/use-with → current map station → atomic item owner.
use super::ComposedFreshAdmission;
use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, pinned_index};
use super::item_view::{ItemKey, SessionItemView};
use crate::durability::character_death::PlayerDeathOccurrence;
use crate::durability::item_craft::ItemCraftRequest;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::view::{self, MapEntryKey, MapFacts};
use oteryn_gameplay_items::fishing::{self, MechanicalSewerAttempt};
use oteryn_gameplay_progression::hit_training::{FISHING_TRIES_PER_USE, SkillFamily};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

mod dispenser;
pub(super) mod enchanting;
mod harvest;
mod planting;
mod rope;
mod skinning;
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
        if let Some(result) = self
            .use_enchanting(actor, command, &intent, source, target)
            .await
        {
            return result;
        }
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
            ingredient: None,
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
                &NativeItemWeights(native).with_wheel(self.premium_coordinator),
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
            let inventory = self
                .root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
                .ok()?;
            let entry = inventory
                .backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .find(|entry| entry.item.item_instance_id == ingredient)?;
            request.ingredient = Some(ingredient);
            request.station = Some(entry.item.definition.clone());
        }
        let (station, map_target) = {
            if let Some(station) = request.station.clone() {
                (
                    station,
                    (
                        MapEntryKey::Base(0),
                        0,
                        false,
                        crate::map::overlay::TilePos {
                            x: 0,
                            y: 0,
                            floor: 0,
                        },
                    ),
                )
            } else {
                let mut runtime = self.runtime.lock().await;
                let index = pinned_index(&runtime, self.active_generation)?;
                let at = super::bundle_map_position(&mut runtime, self.movement_cells(), actor)?;
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
                    pos,
                );
                (
                    super::item_look::typed_definition(index, reference)?,
                    map_target,
                )
            }
        };
        request.station = Some(station);
        let mut catch_roll = 1;
        let mut fishing_skill = 10;
        // Fishing is the same durable consume/mint owner as cooking, but keeps the rod and
        // consumes a worm only after the skill-scaled catch succeeds.
        let water_key = &request.station.as_ref()?.production_key;
        let target_pos = map_target.3;
        let sewer_water = item_id(water_key).is_some_and(|id| {
            fishing::is_yalahar_sewer_water(id, target_pos.x, target_pos.y, target_pos.floor)
        });
        if sewer_water || fishing::is_water(water_key) {
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
            let target_id = item_id(water_key)?;
            let rod = entries.clone().find(|entry| {
                entry.item.item_instance_id == source
                    && (fishing::accepts_fishing_rod(
                        &entry.item.definition.production_key,
                        water_key,
                    ) || (sewer_water
                        && item_id(&entry.item.definition.production_key) == Some(9306)))
            });
            let Some(rod) = rod else {
                return disposition(UseDisposition::NothingToUse);
            };
            let mechanical_sewer =
                sewer_water && item_id(&rod.item.definition.production_key) == Some(9306);
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
            if mechanical_sewer && nail.is_none() {
                return disposition(UseDisposition::NothingToUse);
            }
            let rolls = fishing_rolls(
                native.source_digest(),
                command.game_session_id,
                command.command_id,
                &request.intent,
            )?;
            catch_roll = rolls.catch;
            let mut due = Vec::new();
            let caught;
            let sewer_attempt;
            {
                let runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                let now = self.owner_now().get();
                fishing_skill = states
                    .get(&runtime, actor, command.game_session_id)
                    .and_then(|state| state.live_skill(SkillFamily::Fishing.index(), now))
                    .and_then(|value| u16::try_from(value).ok())
                    .unwrap_or(10);
                caught = fishing::catches(fishing_skill, catch_roll);
                sewer_attempt = mechanical_sewer.then(|| {
                    fishing::mechanical_sewer_attempt(
                        target_id,
                        true,
                        nail.is_some(),
                        caught,
                        rolls.shock,
                    )
                });
                let shocked = sewer_attempt == Some(MechanicalSewerAttempt::Shocked);
                let sewer_map = if mechanical_sewer {
                    if runtime.owner_fence().is_err() {
                        return disposition(UseDisposition::StaleState);
                    }
                    if shocked
                        && (!self
                            .states
                            .player_trades
                            .physical(command.game_session_id, Some(source))
                            || states.is_dead(actor)
                            || !states.spell_idle(&runtime, actor, command.game_session_id))
                    {
                        return disposition(UseDisposition::Rejected);
                    }
                    let map = self.bundle_map()?;
                    let (MapEntryKey::Base(placement), revision, _, _) = map_target else {
                        return disposition(UseDisposition::StaleState);
                    };
                    if map.facts.pinned().object_revision(placement) != revision {
                        return disposition(UseDisposition::StaleState);
                    }
                    map.facts.emit_movement_effect(target_pos, 2); // CONST_ME_LOSEENERGY
                    Some(map)
                } else {
                    None
                };
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
                if let Some(map) = sewer_map
                    && shocked
                {
                    let session = command.game_session_id;
                    let Some(current) = states.get(&runtime, actor, session) else {
                        return disposition(UseDisposition::StaleState);
                    };
                    let (mut next, hit) =
                        crate::spell::actor_conditions::stage_item_health_loss(current, 35, now)
                            .ok()?;
                    let death = if hit.health_after == 0 {
                        states.deaths.try_reserve(1).ok()?;
                        let occurrence = PlayerDeathOccurrence::from_bytes(
                            super::SecureIdentifiers.source_think_event_id()?,
                        )
                        .ok()?;
                        Some(super::actor_spell::PlayerDeath {
                            occurrence,
                            public_cause: None,
                            cell: runtime.read_actor_position(actor).ok()?.position(),
                        })
                    } else {
                        let _ = stage_sewer_shock_outfit(&mut next, native, now, &rolls);
                        next.source_set_batch_revision(current.revision().checked_add(1)?);
                        None
                    };
                    if !states.commit(&runtime, actor, session, next) {
                        return disposition(UseDisposition::StaleState);
                    }
                    if let Some(death) = death {
                        states.deaths.push((actor, session, death));
                    }
                    map.facts.emit_movement_effect(target_pos, 12); // CONST_ME_ENERGYHIT
                    self.states.chat.publish_quest_speech(
                        &runtime,
                        actor,
                        session,
                        "Yikes! That DID hurt!",
                    );
                    states.owner_wake.notify_one();
                }
            }
            self.save_trained_levels(due).await;
            if sewer_attempt == Some(MechanicalSewerAttempt::Shocked) {
                // The sequencer retains the USE result before the response and serves
                // duplicate commands from there. No bait or map mutation is committed.
                return disposition(UseDisposition::Committed);
            }
            let consumable = if mechanical_sewer { nail } else { worm };
            if !caught || consumable.is_none() {
                return disposition(UseDisposition::NothingToUse);
            }
            request.tool = Some(rod);
            request.source = consumable?;
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
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .find(|entry| entry.item.item_instance_id == source);
        let target_id = request
            .station
            .as_ref()
            .and_then(|s| item_id(&s.production_key));
        if let (Some(tool), Some(target_id)) = (tool, target_id)
            && let Some(tool_id) = item_id(&tool.item.definition.production_key)
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
            let (MapEntryKey::Base(key), revision, invalid, _) = map_target else {
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
                &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                false,
            )
            .await;
        let result = result.ok()??;
        drop(_permit);
        if result == UseDisposition::Committed {
            self.refresh_terrain_tools().await;
        }
        disposition(result)
    }
}

/// Decision used by the real I11 two-carried-item baking handler.
pub(super) fn carried_ingredient_action_allowed(is_dead: bool, spell_idle: bool) -> bool {
    !is_dead && spell_idle
}

fn item_id(key: &str) -> Option<u16> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}

pub(super) struct FishingRolls {
    pub(super) catch: u32,
    pub(super) shock: u32,
    root: GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
}

pub(super) fn fishing_rolls(
    source_digest: [u8; 32],
    session: crate::foundation::GameSessionId,
    command_id: u64,
    intent: &[u8],
) -> Option<FishingRolls> {
    let digest = Sha256::new()
        .chain_update(b"oteryn.fishing.catch.v1")
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .chain_update(intent)
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    let root = GameplayDecisionRoot::from_bytes(source_digest);
    let catch = deterministic_decision_u64(&root, occurrence, "fishing.catch", 0).ok()?;
    let shock = deterministic_decision_u64(&root, occurrence, "fishing.sewer_shock", 0).ok()?;
    Some(FishingRolls {
        catch: u32::try_from(crate::spell::uniform_draw(catch, 1, 100_000)).ok()?,
        shock: u32::try_from(crate::spell::uniform_draw(shock, 1, 100)).ok()?,
        root,
        occurrence,
    })
}

fn stage_sewer_shock_outfit(
    next: &mut crate::spell::cast::PlayerSpellState,
    native: &crate::content::native_gameplay::NativeGameplayState,
    now: u64,
    rolls: &FishingRolls,
) -> Option<()> {
    use crate::foundation::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
    };
    let appearance = native
        .spell_appearances()?
        .for_creature("oteryn:creature.skeleton", "definition-r1")?;
    let definition = ConditionDefinition::new(
        "fishing.sewer_shock",
        1,
        ConditionValues::Outfit {
            look_type: appearance.look_type(),
            duration_ms: 4_000,
        },
    )?
    .with_appearance(appearance.selection())?;
    let base_speed = u16::try_from(next.owned_base_speed()).ok()?;
    next.conditions
        .apply(
            &definition,
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &ApplicationFacts {
                now,
                base_speed,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &rolls.root,
                occurrence: rolls.occurrence,
            },
        )
        .ok()?;
    Some(())
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
