//! Carried terrain tools use the existing USE_WITH map route and durable item owner.
use super::*;
use crate::content::{ReferenceItemField, native_gameplay::NativeGameplayState};
use crate::durability::item_decay_retire::CorpseDecayFence;
use crate::durability::terrain_tool::{
    TerrainMutation, TerrainScope, WOODEN_BAR_DECAY_MS, wooden_bar_remains,
};
use crate::gameplay_transport::house_furnishing::destroy_roll_succeeds;
use crate::gameplay_transport::world_map::{
    MapHandleTarget, MapTargetRefusal, resolve_map_handle, resolve_map_target,
};
use crate::map::facts::wooden_bar_collision;
use crate::map::view::MapFacts;
use oteryn_gameplay_items::hotkey_search::HotkeyRefusal;
use oteryn_gameplay_world::terrain_tools;

pub(super) enum TerrainUseError {
    Unavailable,
    MapResyncRequired,
}

impl From<()> for TerrainUseError {
    fn from((): ()) -> Self {
        Self::Unavailable
    }
}

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

pub(in crate::gameplay_transport) fn world_destroy_attack(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Option<i32> {
    let item = item_id(&definition.production_key)?;
    let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
    let ReferenceItemField::Known(weapon) = &policy.record().semantics.weapon else {
        return None;
    };
    let ReferenceItemField::Known(attack) = weapon.attack else {
        return None;
    };
    if !(0..=65535).contains(&attack.0) {
        return None;
    }
    crystal_global_destroy_threshold(item)
}

/// Crystal Global 201189f8 destroy.lua's weaponIds, plus the machete action's
/// onDestroyItem fallback. register_actions.lua rolls against max(10, getAttack()).
/// The thresholds come from data/items/items.xml (SHA-256
/// 8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509).
fn crystal_global_destroy_threshold(item: u16) -> Option<i32> {
    Some(match item {
        3267
        | 3268
        | 3270
        | 3272
        | 3289..=3292
        | 3336
        | 9387
        | 12673
        | 17784
        | 27651
        | 28464
        | 28724
        | 29421
        | 29422
        | 30283
        | 30395
        | 30396
        | 31580
        | 31614
        | 32093
        | 32616
        | 34155
        | 34253
        | 34254
        | 43864..=43875 => 10,
        3294 | 28725 | 29419 => 11,
        3273 | 3274 | 3308 | 3337 | 6101 => 12,
        3338 => 13,
        3264 | 3325 | 17109 => 14,
        3276 | 3283 => 15,
        3286 | 3299 | 3300 | 9373 | 9375 | 9385 => 16,
        3285 | 3329 | 9386 | 9396 | 17824 => 17,
        860 | 3297 | 3310 | 9376 | 27523 => 18,
        3327 | 7773 | 7774 => 21,
        3316 => 23,
        3271 | 3280 | 3305 | 12510 | 27525 => 24,
        3266 | 3282 | 3298 | 3348 | 22760 => 25,
        3301 | 22759 => 26,
        3320 | 17812 | 17813 => 27,
        3311 | 3317 | 3346 | 7385 | 10389 => 28,
        3265 | 7381 | 7425 => 30,
        3323 | 7432 | 17859 => 31,
        3322 | 7408 | 10392 => 32,
        3318 | 21172 => 33,
        3313 | 7387 | 21177 => 34,
        3269 | 3275 | 3344 | 7379 | 7430 | 7449 | 12683 => 35,
        3295 | 3324 => 36,
        3326 | 7437 | 10406 => 37,
        3333 | 3339 => 38,
        3314 | 3321 | 3328 | 7451 | 7456 | 21178..=21180 | 22172 => 39,
        3343 | 7404 | 7407 | 7419 | 7424 | 7454 | 17828 | 21174 => 40,
        3312 | 3334 | 7412 | 10391 => 41,
        3284 | 7380 | 7383 | 7392 | 7406 | 7409 | 7411 | 12731 | 14040 => 42,
        7386 | 7388 | 7410 | 7413 | 7416 | 7426 | 10390 | 13987 => 43,
        7384 | 7389 | 7402 | 7415 | 7427 | 7436 | 14250 | 21171 => 44,
        3279 | 3335 | 7391 | 7417 | 7421 | 7433 | 22171 => 45,
        3281 | 3315 | 7418 | 7420 | 7422 | 7428 | 14043 | 21176 | 28829..=28832 => 46,
        3302 | 7382 | 7390 | 7414 | 7429 | 7434 | 10388 | 11657 | 16160 | 21173 => 47,
        3288 | 3332 | 3342 | 7403 | 7431 | 7455 | 11692 | 11693 | 20076 | 22727 => 48,
        3309 | 3331 | 3340 | 7405 | 7435 | 8102 | 13991 | 14001 | 16175 | 20070 | 21219 => 49,
        3319 | 3341 | 6553 | 8100 | 8103 | 16162 | 20065 | 20077 | 27449 | 27453 => 50,
        7423 | 7453 | 8096 | 8099 | 8101 | 8104 | 16161 | 20067 | 20071 | 20073 | 20079 | 22762
        | 27451 => 51,
        7450 | 8097 | 20066 | 20068 | 20074 | 20078 | 27450 | 27452 | 30398 | 35514 => 52,
        3296 | 8098 | 20072 | 20080 | 27454 => 53,
        20069 | 20075 => 54,
        3278 | 20081 => 55,
        28723 => 56,
        6528 => 72,
        _ => return None,
    })
}

fn world_destroy_roll(root: [u8; 32], command: UseCommand, placement: &[u8]) -> Result<u8, ()> {
    let digest = Sha256::new()
        .chain_update(b"oteryn.world.furniture.destroy.v1")
        .chain_update(command.game_session_id.as_bytes())
        .chain_update(command.command_id.to_be_bytes())
        .chain_update(placement)
        .finalize();
    let occurrence = DecisionOccurrenceId::from_bytes(digest[..16].try_into().map_err(|_| ())?);
    let root = GameplayDecisionRoot::from_bytes(root);
    let mut draw_index = 0;
    loop {
        let draw = deterministic_decision_u64(
            &root,
            occurrence,
            "world.furniture.destroy.roll",
            draw_index,
        )
        .map_err(|_| ())?;
        if draw < u64::MAX - u64::MAX % 80 {
            return u8::try_from(draw % 80 + 1).map_err(|_| ());
        }
        draw_index = draw_index.checked_add(1).ok_or(())?;
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn terrain_scope(&self) -> Option<TerrainScope> {
        let map = self.bundle_map()?;
        let native = self.active_generation?.native_gameplay()?;
        Some(TerrainScope {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_generation: native.source_digest(),
            reset_epoch: i64::try_from(self.reset_epoch).ok()?,
        })
    }

    pub(super) async fn use_terrain_tool(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, TerrainUseError> {
        let Some(scope) = self.terrain_scope() else {
            return Ok(None);
        };
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let encoded =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        if let Some(same) = self.bundle_map().and_then(|map| {
            map.facts.naginata_pick_replay(
                *command.game_session_id.as_bytes(),
                command.command_id,
                &encoded,
            )
        }) {
            return Ok(Some(outcome(if same {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            })));
        }
        // Drop also invalidates a projection after a cancelled/unknown COMMIT.
        let _ground_change = super::super::loop_move::GroundChange(
            intent
                .use_with_map_item
                .as_ref()
                .map(|_| &self.states.loop_ground),
        );
        let permit = self.spell_lane_permit().await.ok_or(())?;
        if let Some(result) = self
            .root
            .apply_terrain_tool(
                &permit,
                self.character,
                self.holder,
                fence,
                command_ref,
                encoded.clone(),
                scope.clone(),
                None,
            )
            .await
            .map_err(|_| ())?
        {
            // A replay may acknowledge an already-committed gobbler XP award.
            if result == UseDisposition::Committed {
                self.states.level_refresh.mark([fence.character_id]);
                // A recovered receipt also repairs the Channel's physical projection.
                // Release the item lane before the existing snapshot owner reacquires it.
                drop(permit);
                self.refresh_terrain_tools().await;
                crate::gameplay_transport::quest_terrain_uses::after_tool(self, actor, command)
                    .await;
            }
            return Ok(Some(outcome(result)));
        }
        let native = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(())?;
        let inventory = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.root
                .read_item_use_inventory(self.character, self.holder, fence),
        )
        .await
        .map_err(|_| ())?
        .map_err(|_| ())?;
        let index = native.item_index().ok_or(())?;
        let tool = match intent.target {
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            },
            UseTarget::ItemByDefinition(wanted) => {
                index.definition_at(wanted).and_then(|(key, revision)| {
                    let wanted = TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key.into(),
                        revision_ref: revision.into(),
                    };
                    match inventory.first_unreserved(&wanted, &[]) {
                        Ok(tool) => Some(tool),
                        // The bounded search already selected equipment in display order.
                        // Only the non-consuming bar plan can use this custody.
                        Err(HotkeyRefusal::EquipmentSource)
                            if intent.use_with_map_item.is_some()
                                && world_destroy_attack(native, &wanted).is_some() =>
                        {
                            inventory
                                .equipment
                                .iter()
                                .find(|item| item.definition == wanted)
                                .map(|item| item.item_instance_id)
                        }
                        Err(_) => None,
                    }
                })
            }
            _ => None,
        };
        let carried = inventory
            .backpack
            .iter()
            .flat_map(|b| &b.entries)
            .chain(inventory.nested.iter().map(|(_, e)| e))
            .map(|entry| &entry.item)
            .find(|item| Some(item.item_instance_id) == tool);
        let Some(item) = carried.or_else(|| {
            inventory
                .equipment
                .iter()
                .find(|item| Some(item.item_instance_id) == tool)
        }) else {
            return Ok(None);
        };
        let definition = &item.definition;
        let terrain_tool = carried.is_some()
            && terrain_tools::TerrainTool::from_production_key(&definition.production_key)
                .is_some();
        let destroy_attack = intent
            .use_with_map_item
            .as_ref()
            .and_then(|_| world_destroy_attack(native, definition));
        if (!terrain_tool && destroy_attack.is_none())
            || index
                .definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
                .is_none()
        {
            return Ok(None);
        }
        if destroy_attack.is_none()
            && !self
                .states
                .player_trades
                .physical(command.game_session_id, tool)
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let (mutation, yield_item, broken_at) = {
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if runtime.binding().world_id() != self.world_id
                || runtime.binding().channel_id() != Some(self.channel_id)
                || pinned_index(&runtime, self.active_generation).is_none()
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let Some(at) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)
            else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            let facts = at.map.facts.pinned();
            let map = super::super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let nothing = MapTargetRefusal::Use(UseDisposition::NothingToUse);
            let resolved = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    resolve_map_target(&map, object, |_| None::<()>)
                        .map(|target| (target.pos, target.placement_key))
                }
                Some(MapUseTarget::Item(_)) => target
                    .ok_or(MapTargetRefusal::Use(UseDisposition::StaleState))
                    .and_then(|key| resolve_map_handle(&map, key).map_err(MapTargetRefusal::Use))
                    .and_then(|target| match target {
                        MapHandleTarget::Base {
                            pos, placement_key, ..
                        } => Ok((pos, placement_key)),
                        MapHandleTarget::Added { .. } | MapHandleTarget::Ground { .. } => {
                            Err(nothing)
                        }
                    }),
                None => Err(nothing),
            };
            let (pos, key) = match resolved {
                Ok(placement) => placement,
                Err(MapTargetRefusal::Use(disposition)) => return Ok(Some(outcome(disposition))),
                Err(MapTargetRefusal::StateRevisionMismatch) => {
                    return Err(TerrainUseError::MapResyncRequired);
                }
            };
            let too_far = i16::from(pos.floor) != at.at.floor
                || at.at.x.abs_diff(i32::from(pos.x)) > 1
                || at.at.y.abs_diff(i32::from(pos.y)) > 1;
            if terrain_tool && too_far {
                return Ok(Some(outcome(UseDisposition::TooFar)));
            }
            let tile = view::compose_tile(&at.map.overlay, &facts, pos)
                .map_err(|_| ())?
                .ok_or(())?;
            let before = tile
                .entries
                .iter()
                .find(|e| e.source.key() == MapEntryKey::Base(key))
                .ok_or(())?
                .facts;
            if facts.house_tile(pos)
                || before.pickupable
                || before.count != 1
                || before.sub_type != 0
            {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            }
            let MapDefinition::Item(reference) = before.definition else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            let target = super::super::item_look::typed_definition(index, reference).ok_or(())?;
            let Some(state) = target
                .production_key
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|s| s.parse().ok())
            else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            let furniture_break = destroy_attack.zip(wooden_bar_remains(state));
            if !terrain_tool && furniture_break.is_none() {
                return Ok(None);
            }
            if too_far {
                return Ok(Some(outcome(UseDisposition::TooFar)));
            }
            if destroy_attack.is_some()
                && furniture_break.is_none()
                && !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, tool)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            // The daily projector creates slime only on these dungeon ground entries.
            // Carrying the tool never authorizes unrelated map appearances or positions.
            if definition.production_key == "oteryn:item.tibia.i12077"
                && (!(33306..=33369).contains(&pos.x)
                    || !(31847..=31919).contains(&pos.y)
                    || pos.floor != -9
                    || before.terrain_kind
                        != Some(oteryn_world_bundle::bundle::TerrainKind::Ground)
                    || view::placement_of(key).is_none_or(|(_, ordinal)| ordinal != 0))
            {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            }
            let Some(action) = at.map.facts.terrain_tool_action(key) else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            if state == 1791
                && action == Some(40031)
                && pos
                    == (crate::map::overlay::TilePos {
                        x: 32356,
                        y: 32074,
                        floor: -10,
                    })
                && definition.production_key == "oteryn:item.tibia.i3456"
            {
                // The source's shared six-hit counter belongs to this Channel map,
                // not to a Character's durable quest tracks. Command replay lives
                // beside the map projection so it cannot deal physical damage twice.
                let digest = Sha256::new()
                    .chain_update(b"oteryn:naginata-pick:v1")
                    .chain_update(command.game_session_id.as_bytes())
                    .chain_update(command.command_id.to_be_bytes())
                    .finalize();
                let mut occurrence = [0; 16];
                occurrence.copy_from_slice(&digest[..16]);
                let draw = deterministic_decision_u64(
                    &GameplayDecisionRoot::from_bytes(
                        runtime.content_pin().server_artifact_digest(),
                    ),
                    DecisionOccurrenceId::from_bytes(occurrence),
                    "quest.naginata.pick_damage",
                    0,
                )
                .map_err(|_| ())?;
                let rolled = crate::spell::uniform_draw(draw, 31, 39);
                let damage = states
                    .get(&runtime, actor, command.game_session_id)
                    .and_then(|state| {
                        state.equipped_element_protection(
                            crate::content::ReferenceResistanceKind::Physical,
                        )
                    })
                    .and_then(|protection| {
                        oteryn_gameplay_items::equipment_effects::apply_protection(
                            rolled, protection,
                        )
                    })
                    .unwrap_or(rolled);
                let damage = u32::try_from(damage).map_err(|_| ())?;
                let committed = at.map.facts.apply_naginata_pick(
                    key,
                    facts.object_revision(key),
                    before,
                    *command.game_session_id.as_bytes(),
                    command.command_id,
                    &encoded,
                    || {
                        states
                            .apply_creature_damage_native(
                                &runtime,
                                actor,
                                command.game_session_id,
                                damage,
                                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                                    self.owner_now().get(),
                                ),
                            )
                            .is_some()
                    },
                );
                if committed {
                    at.map.facts.emit_movement_effect(pos, 3);
                }
                return Ok(Some(outcome(if committed {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                })));
            }
            let mut placement = at.map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let (after, decay, yield_item) =
                if let Some((attack, (debris, restored))) = furniture_break {
                    if before.bound
                        || action.is_some()
                        || at.map.facts.action_at(key).is_some()
                        || u32::from(before.appearance_id) != state
                        || [debris, restored].into_iter().any(|id| {
                            u16::try_from(id)
                                .ok()
                                .and_then(wooden_bar_collision)
                                .is_none()
                        })
                    {
                        return Ok(Some(outcome(UseDisposition::NothingToUse)));
                    }
                    if !self
                        .states
                        .player_trades
                        .unreserved(command.game_session_id, item.item_instance_id)
                    {
                        return Ok(Some(outcome(UseDisposition::Rejected)));
                    }
                    if !self.states.world_gate.lock().is_ok_and(|mut gate| {
                        gate.admit_ex_action(actor, self.clock_origin.elapsed())
                    }) {
                        return Ok(Some(outcome(UseDisposition::Exhausted)));
                    }
                    if !destroy_roll_succeeds(
                        world_destroy_roll(scope.content_generation, command, &placement)?,
                        attack,
                    ) {
                        at.map.facts.emit_movement_effect(pos, 3);
                        return Ok(Some(outcome(UseDisposition::Committed)));
                    }
                    (debris, vec![(restored, WOODEN_BAR_DECAY_MS)], None)
                } else {
                    let catalogue = terrain_tools::crystal_core_bindings();
                    let Some(plan) = terrain_tools::dispatch(
                        &catalogue,
                        &definition.production_key,
                        state,
                        action,
                        0,
                    ) else {
                        return Ok(Some(outcome(UseDisposition::NothingToUse)));
                    };
                    let decay = plan
                        .binding
                        .decay
                        .iter()
                        .map(|s| Ok((s.state, i64::try_from(s.after.as_millis()).map_err(|_| ())?)))
                        .collect::<Result<_, ()>>()?;
                    (plan.binding.transformed, decay, plan.binding.yield_item)
                };
            for state in std::iter::once(after).chain(decay.iter().map(|&(state, _)| state)) {
                let key = format!("oteryn:item.tibia.i{state}");
                if index
                    .definition_ref("Item", &key, &target.revision_ref)
                    .is_none()
                {
                    return Ok(Some(outcome(UseDisposition::NothingToUse)));
                }
            }
            (
                TerrainMutation {
                    tool: item.item_instance_id,
                    tool_key: definition.production_key.clone(),
                    tool_revision: definition.revision_ref.clone(),
                    placement,
                    expected_revision: i64::try_from(facts.object_revision(key)).map_err(|_| ())?,
                    before: state,
                    after,
                    decay,
                },
                yield_item,
                furniture_break.map(|_| pos),
            )
        };
        let soil = mutation
            .tool_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u16>().ok())
            .and_then(|soil| {
                u16::try_from(mutation.before)
                    .ok()
                    .and_then(|target| oteryn_gameplay_items::elemental_soil::charge(soil, target))
            });
        if soil.is_some() || yield_item.is_some() {
            let tool = mutation.tool;
            let after = u16::try_from(mutation.after).map_err(|_| ())?;
            let request = ItemCraftRequest {
                command: command_ref,
                source: tool,
                tool: if soil.is_some() { None } else { Some(tool) },
                ingredient: None,
                station: None,
                intent: encoded,
                gathering: Some(crate::durability::item_craft::GatheringCraft {
                    scope,
                    mutation,
                    attempt: soil.unwrap_or(oteryn_gameplay_items::fishing::GatheringAttempt {
                        after: Some(after),
                        yield_item: yield_item.map(u16::try_from).transpose().map_err(|_| ())?,
                        consumes_bait: false,
                    }),
                }),
                enchanting: None,
            };
            let result = self
                .root
                .craft_item(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    native,
                    &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                    false,
                )
                .await
                .map_err(|_| ())?
                .ok_or(())?;
            drop(permit);
            if result == UseDisposition::Committed {
                self.refresh_terrain_tools().await;
            }
            return Ok(Some(outcome(result)));
        }
        // The lane remains held until durable acknowledgement. The next clock/read recovers an
        // ambiguous commit from the same rows, so no speculative runtime mutation occurs.
        let result = if mutation.tool_key == "oteryn:item.tibia.i12077" {
            use crate::durability::character_progression::ExperienceAwardRequest;
            use crate::durability::terrain_tool::{TerrainExperience, voice_experience_occurrence};
            let Some(binding) = self.progression_binding(command.game_session_id) else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let Some(mut xp_fence) = self.current_quest_fence(command.game_session_id).await?
            else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            xp_fence.expected_character_revision = slot
                .cursor(self.root, self.character)
                .await
                .map_err(|_| ())?;
            let experience = Box::new(TerrainExperience {
                fence: xp_fence,
                request: ExperienceAwardRequest {
                    occurrence: voice_experience_occurrence(command_ref).map_err(|_| ())?,
                    amount: oteryn_simulation_determinism::ExactI64::new(20),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
            });
            let durable = self
                .root
                .apply_terrain_tool_with_experience(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    command_ref,
                    encoded,
                    scope,
                    Some(mutation),
                    Some(experience),
                )
                .await;
            // Reload after success or an unknown outcome: an ambiguous commit may
            // already have advanced CharacterRevision with the terrain receipt.
            slot.observe_endgame_progression();
            let result = durable.map_err(|_| ())?.ok_or(())?;
            if result == UseDisposition::Committed {
                self.states.level_refresh.mark([fence.character_id]);
            }
            result
        } else {
            self.root
                .apply_terrain_tool(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    command_ref,
                    encoded,
                    scope,
                    Some(mutation),
                )
                .await
                .map_err(|_| ())?
                .ok_or(())?
        };
        drop(permit);
        if result == UseDisposition::Committed {
            self.refresh_terrain_tools().await;
            if let (Some(pos), Some(map)) = (broken_at, self.bundle_map()) {
                map.facts.emit_movement_effect(pos, 3);
            }
            crate::gameplay_transport::quest_terrain_uses::after_tool(self, actor, command).await;
        }
        Ok(Some(outcome(result)))
    }

    pub(in crate::gameplay_transport) async fn refresh_terrain_tools(&self) {
        self.root
            .with_background_operation(self.refresh_terrain_tools_inner())
            .await;
    }

    async fn refresh_terrain_tools_inner(&self) {
        if !self.refresh_cake_world_map().await {
            return;
        }
        let Some(scope) = self.terrain_scope() else {
            return;
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        let fence = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().world_id() != self.world_id
                || runtime.binding().channel_id() != Some(self.channel_id)
            {
                return;
            }
            let Ok(owner) = runtime.owner_fence() else {
                return;
            };
            CorpseDecayFence {
                runtime_scope: runtime.binding().runtime_scope(),
                scope_ownership_generation: owner.generation(),
            }
        };
        let Ok(rows) = self
            .root
            .terrain_tool_snapshot(&permit, self.character, self.holder, fence, scope)
            .await
        else {
            return;
        };
        let runtime = self.runtime.lock().await;
        if !runtime
            .owner_fence()
            .is_ok_and(|owner| owner.generation() == fence.scope_ownership_generation)
        {
            return;
        }
        let Some(map) = self.bundle_map() else {
            return;
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return;
        };
        let Some(index) = native.item_index() else {
            return;
        };
        for row in rows {
            if row.placement.len() != 40 || row.placement[..32] != map.overlay.base().digest()[..] {
                return;
            }
            let Ok(bytes) = row.placement[32..].try_into() else {
                return;
            };
            let key = u64::from_be_bytes(bytes);
            if oteryn_gameplay_items::event_planting::world_change_item_source(key).is_some() {
                let (Ok(revision), Ok(appearance)) =
                    (u64::try_from(row.revision), u16::try_from(row.item))
                else {
                    return;
                };
                if map
                    .facts
                    .install_world_change_harvest_state(key, revision, appearance)
                    != Some(true)
                {
                    return;
                }
                continue;
            }
            let Some((pos, _)) = view::placement_of(key) else {
                return;
            };
            let facts = map.facts.pinned();
            let Some(tile) = view::compose_tile(&map.overlay, &facts, pos).ok().flatten() else {
                return;
            };
            if row.item == 0 && facts.base_hidden(key) {
                continue;
            }
            let Some(entry) = tile
                .entries
                .iter()
                .find(|e| e.source.key() == MapEntryKey::Base(key))
            else {
                return;
            };
            // Map-authored plants may be Terrain; the pinned palette owns each successor.
            if matches!(
                row.item,
                306 | 324 | 330 | 13842 | 13843 | 14029 | 14031 | 14032 | 3699 | 3700
            ) {
                let Some(after) = u16::try_from(row.item)
                    .ok()
                    .and_then(|appearance| map.facts.quest_callback_replacement(key, appearance))
                else {
                    return;
                };
                let Ok(revision) = u64::try_from(row.revision) else {
                    return;
                };
                if !map
                    .facts
                    .install_gathering_state(key, revision, entry.facts, Some(after))
                {
                    return;
                }
                continue;
            }
            let MapDefinition::Item(reference) = entry.facts.definition else {
                return;
            };
            let Some(definition) = super::super::item_look::typed_definition(index, reference)
            else {
                return;
            };
            if row.item == 0 {
                if !map.facts.install_gathering_state(
                    key,
                    u64::try_from(row.revision).unwrap_or(0),
                    entry.facts,
                    None,
                ) {
                    return;
                }
                continue;
            }
            let Some(reference) = index.definition_ref(
                "Item",
                &format!("oteryn:item.tibia.i{}", row.item),
                &definition.revision_ref,
            ) else {
                return;
            };
            let Ok(revision) = u64::try_from(row.revision) else {
                return;
            };
            let after = view::EntryFacts {
                definition: MapDefinition::Item(reference),
                appearance_id: u16::try_from(row.item).unwrap_or(0),
                blocks_projectile: native
                    .item_policy_for_source_id(row.item)
                    .and_then(|policy| policy.record().attributes.blocks_projectile)
                    .unwrap_or(entry.facts.blocks_projectile),
                ..entry.facts
            };
            if matches!(row.item, 2295 | 2296 | 3145 | 3146)
                && matches!(entry.facts.appearance_id, 2291..=2296 | 3145 | 3146)
            {
                if !map.install_wooden_bar_state(key, revision, after) {
                    return;
                }
                continue;
            }
            let gathering = matches!(
                row.item,
                4012 | 4325 | 4026 | 4063 | 4098 | 4138 | 7442 | 7237 | 7200
            );
            if gathering {
                let Some(policy) = native
                    .item_policy_for_source_id(row.item)
                    .map(|p| p.record())
                else {
                    return;
                };
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return;
                };
                let crate::content::ReferenceItemField::Known(pickupable) = physical.pickupable
                else {
                    return;
                };
                let Some(blocks_projectile) = policy.attributes.blocks_projectile else {
                    return;
                };
                let after = view::EntryFacts {
                    pickupable,
                    blocks_projectile,
                    ..after
                };
                if !map
                    .facts
                    .install_gathering_state(key, revision, entry.facts, Some(after))
                {
                    return;
                }
            } else if !map.facts.install_terrain_state(key, revision, after) {
                return;
            }
        }
    }
}
