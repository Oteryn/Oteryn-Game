//! Carried terrain tools use the existing USE_WITH map route and durable item owner.
use super::*;
use crate::durability::item_decay_retire::CorpseDecayFence;
use crate::durability::terrain_tool::{TerrainMutation, TerrainScope};
use crate::map::view::MapFacts;
use oteryn_gameplay_world::terrain_tools;

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The consume/transform owner binds the jam to the original command and
    /// writes the replacement, recovery timer, and achievement together.
    pub(in crate::gameplay_transport) async fn commit_agent_jam(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        command: UseCommand,
        tool: [u8; 16],
        intent: Vec<u8>,
    ) -> Result<UseOutcome, ()> {
        let fence = command.item_fence.ok_or(())?;
        let native = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(())?;
        let request = crate::durability::consume_transform::ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            source: tool,
            target: None,
            intent,
        };
        let content = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let receipt = self
            .root
            .jam_secret_agent_tool(
                permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &content,
            )
            .await
            .map_err(|_| ())?;
        let Some(receipt) = receipt else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        if receipt.fresh && receipt.disposition == UseDisposition::Committed {
            self.states.chat.push_quest_feedback(
                command.game_session_id,
                "Oh no! Your tool is jammed and can't be used for a minute.",
            );
        }
        Ok(outcome(receipt.disposition))
    }

    pub(in crate::gameplay_transport) fn terrain_scope(&self) -> Option<TerrainScope> {
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
    ) -> Result<Option<UseOutcome>, ()> {
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
        self.mark_use_receipt(
            command,
            Some(crate::gameplay_transport::session_ingress::UseReceipt::Terrain),
        );
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let retry = AdmissionRetryBudget::for_command(self.root);
        let receipt = retry
            .retry_command_guard(async || {
                self.root
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
            })
            .await
            .map_err(|_| ())?;
        if let Some(result) = receipt {
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
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let inventory = retry_item_use_holder(&retry, async || {
            self.root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
        })
        .await;
        let (Ok(inventory), Some(index)) = (inventory, native.item_index()) else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let tool = match intent.target {
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            },
            UseTarget::ItemByDefinition(wanted) => {
                index.definition_at(wanted).and_then(|(key, revision)| {
                    inventory
                        .first_unreserved(
                            &TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: key.into(),
                                revision_ref: revision.into(),
                            },
                            &[],
                        )
                        .ok()
                })
            }
            _ => None,
        };
        let Some(entry) = inventory
            .backpack
            .iter()
            .flat_map(|b| &b.entries)
            .chain(inventory.nested.iter().map(|(_, e)| e))
            .find(|e| Some(e.item.item_instance_id) == tool)
        else {
            return Ok(None);
        };
        let definition = &entry.item.definition;
        if terrain_tools::TerrainTool::from_production_key(&definition.production_key).is_none()
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
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, tool)
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let agent_tool = super::rope::secret_agent_rope(&definition.production_key);
        let ground = if agent_tool && matches!(target, Some(ItemKey::Instance(_))) {
            let map = self.bundle_map().ok_or(())?;
            Some(
                super::super::loop_move::loop_ground(self, map.overlay.base(), Some(index))
                    .await
                    .ok_or(())?,
            )
        } else {
            None
        };
        let mutation = {
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
                overlay: ground
                    .as_ref()
                    .map_or(&at.map.overlay, |ground| ground.overlay()),
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let resolved = intent
                .use_with_map_item
                .as_ref()
                .and_then(|intent| super::rope::terrain_target(&map, intent, target).ok());
            let Some((pos, key)) = resolved.filter(|(_, key)| agent_tool || key.is_some()) else {
                return Ok(Some(outcome(UseDisposition::StaleState)));
            };
            if i16::from(pos.floor) != at.at.floor
                || at.at.x.abs_diff(i32::from(pos.x)) > 1
                || at.at.y.abs_diff(i32::from(pos.y)) > 1
            {
                return Ok(Some(outcome(UseDisposition::TooFar)));
            }
            if super::rope::agent_tool_jams(
                command,
                &definition.production_key,
                entry.item.item_instance_id,
            ) {
                drop((states, runtime));
                return self
                    .commit_agent_jam(&permit, command, entry.item.item_instance_id, encoded)
                    .await
                    .map(Some);
            }
            let Some(key) = key else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            let Ok(Some(tile)) = view::compose_tile(&at.map.overlay, &facts, pos) else {
                return Ok(Some(UseOutcome::rejected()));
            };
            let Some(map_entry) = tile
                .entries
                .iter()
                .find(|e| e.source.key() == MapEntryKey::Base(key))
            else {
                return Ok(Some(UseOutcome::rejected()));
            };
            let before = map_entry.facts;
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
            let Some(target) = super::super::item_look::typed_definition(index, reference) else {
                return Ok(Some(UseOutcome::rejected()));
            };
            let Some(state) = target
                .production_key
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|s| s.parse().ok())
            else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
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
            let catalogue = terrain_tools::crystal_core_bindings();
            let Some(plan) =
                terrain_tools::dispatch(&catalogue, &definition.production_key, state, action, 0)
            else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            for state in std::iter::once(plan.binding.transformed)
                .chain(plan.binding.decay.iter().map(|s| s.state))
            {
                let key = format!("oteryn:item.tibia.i{state}");
                if index
                    .definition_ref("Item", &key, &target.revision_ref)
                    .is_none()
                {
                    return Ok(Some(outcome(UseDisposition::NothingToUse)));
                }
            }
            let mut placement = at.map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            (
                TerrainMutation {
                    tool: entry.item.item_instance_id,
                    tool_key: definition.production_key.clone(),
                    tool_revision: definition.revision_ref.clone(),
                    placement,
                    expected_revision: i64::try_from(facts.object_revision(key)).map_err(|_| ())?,
                    before: state,
                    after: plan.binding.transformed,
                    decay: plan
                        .binding
                        .decay
                        .iter()
                        .map(|s| Ok((s.state, i64::try_from(s.after.as_millis()).map_err(|_| ())?)))
                        .collect::<Result<_, ()>>()?,
                },
                plan.binding.yield_item,
            )
        };
        let soil = mutation
            .0
            .tool_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u16>().ok())
            .and_then(|soil| {
                u16::try_from(mutation.0.before)
                    .ok()
                    .and_then(|target| oteryn_gameplay_items::elemental_soil::charge(soil, target))
            });
        if soil.is_some() || mutation.1.is_some() {
            let tool = mutation.0.tool;
            let after = u16::try_from(mutation.0.after).map_err(|_| ())?;
            let request = ItemCraftRequest {
                command: command_ref,
                source: tool,
                tool: if soil.is_some() { None } else { Some(tool) },
                ingredient: None,
                station: None,
                intent: encoded,
                gathering: Some(crate::durability::item_craft::GatheringCraft {
                    scope,
                    mutation: mutation.0,
                    attempt: soil.unwrap_or(oteryn_gameplay_items::fishing::GatheringAttempt {
                        after: Some(after),
                        yield_item: mutation.1.map(u16::try_from).transpose().map_err(|_| ())?,
                        consumes_bait: false,
                    }),
                }),
                enchanting: None,
            };
            let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
            self.mark_use_receipt(
                command,
                Some(crate::gameplay_transport::session_ingress::UseReceipt::Craft),
            );
            let result = retry_busy(&retry, async || {
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
            })
            .await;
            let result = write_disposition(result)?;
            drop(permit);
            if result == UseDisposition::Committed {
                self.refresh_terrain_tools().await;
            }
            return Ok(Some(outcome(result)));
        }
        let mutation = mutation.0;
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
            let result = write_disposition(durable)?;
            if result == UseDisposition::Committed {
                self.states.level_refresh.mark([fence.character_id]);
            }
            result
        } else {
            let durable = retry_busy(&retry, async || {
                self.root
                    .apply_terrain_tool(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        command_ref,
                        encoded.clone(),
                        scope.clone(),
                        Some(mutation.clone()),
                    )
                    .await
            })
            .await;
            write_disposition(durable)?
        };
        drop(permit);
        if result == UseDisposition::Committed {
            self.refresh_terrain_tools().await;
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
            let gathering = matches!(
                row.item,
                4012 | 4325 | 4026 | 4063 | 4098 | 4138 | 7442 | 7444..=7446 | 7237 | 7200
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
