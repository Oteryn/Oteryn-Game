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
        let mutation = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
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
                super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)
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
            let resolved = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    super::super::world_map::resolve_map_target(&map, object, |_| None::<()>)
                        .ok()
                        .map(|t| (t.pos, t.placement_key))
                }
                Some(MapUseTarget::Item(_)) => target
                    .and_then(|key| super::super::world_map::resolve_map_handle(&map, key).ok())
                    .and_then(|t| match t {
                        super::super::world_map::MapHandleTarget::Base {
                            pos,
                            placement_key,
                            ..
                        } => Some((pos, placement_key)),
                        _ => None,
                    }),
                None => None,
            };
            let Some((pos, key)) = resolved else {
                return Ok(Some(outcome(UseDisposition::StaleState)));
            };
            if i16::from(pos.floor) != at.at.floor
                || at.at.x.abs_diff(i32::from(pos.x)) > 1
                || at.at.y.abs_diff(i32::from(pos.y)) > 1
            {
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
            let Some(action) = at.map.facts.terrain_tool_action(key) else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            let catalogue = terrain_tools::crystal_core_bindings();
            let Some(plan) =
                terrain_tools::dispatch(&catalogue, &definition.production_key, state, action, 0)
            else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            // Harvest must use the audited item mint owner. Never transform and silently lose
            // the promised yield while that slice remains unqualified.
            if plan.binding.yield_item.is_some() {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            }
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
            }
        };
        // The lane remains held until durable acknowledgement. The next clock/read recovers an
        // ambiguous commit from the same rows, so no speculative runtime mutation occurs.
        let result = self
            .root
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
            .ok_or(())?;
        drop(permit);
        if result == UseDisposition::Committed {
            self.refresh_terrain_tools().await;
        }
        Ok(Some(outcome(result)))
    }

    pub(in crate::gameplay_transport) async fn refresh_terrain_tools(&self) {
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
                ..entry.facts
            };
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
