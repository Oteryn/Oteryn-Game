use super::super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_view::ItemKey,
};
use super::{SoulpitCoreSelection, SoulpitEntryIntent, outcome};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::map::view::{self, MapEntryKey};
use oteryn_gameplay_domain::domain::soulpit::{
    self, OBELISK_ACTIVE_SOURCE_ID, OBELISK_INACTIVE_SOURCE_ID, OBELISK_POSITION,
};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Existing USE/use-with routing, with no new client-provided species, reward or room ID.
    pub(in crate::gameplay_transport) async fn use_soulpit_obelisk(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if !matches!(
            intent.target,
            UseTarget::Item(_) | UseTarget::ItemByDefinition(_)
        ) {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let owner = self.states.boss_activity_handoff.get();
        let mut request = SoulpitEntryIntent {
            actor,
            command,
            encoded_intent: encode_item_use_intent(intent).map_err(|_| ())?,
            selected: None,
        };
        if let Some(owner) = &owner
            && let Some(replay) = owner.enter_soulpit(self, request.clone()).await?
        {
            return Ok(Some(replay));
        }
        // A carried target belongs to crafting, including a replay after its handle
        // was retired. Entry-specific live guards must not preempt that receipt.
        if matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
            && !matches!(target, Some(ItemKey::MapBase { .. }))
        {
            return Ok(None);
        }
        let permit = self.spell_lane_permit().await.ok_or(())?;
        {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let binding = runtime.binding();
            let Some(index) =
                super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return Ok(None);
            };
            let Some(at) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)
            else {
                return Ok(None);
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
                        .map(|target| (target.pos, MapEntryKey::Base(target.placement_key)))
                }
                Some(MapUseTarget::Item(_)) => target
                    .and_then(|key| super::super::world_map::resolve_map_handle(&map, key).ok())
                    .and_then(|target| match target {
                        super::super::world_map::MapHandleTarget::Base {
                            pos,
                            placement_key,
                            ..
                        } => Some((pos, MapEntryKey::Base(placement_key))),
                        _ => None,
                    }),
                None => None,
            };
            let Some((position, key)) = resolved else {
                return Ok(None);
            };
            let Some(tile) = view::compose_tile(&at.map.overlay, &facts, position)
                .ok()
                .flatten()
            else {
                return Ok(None);
            };
            let Some(entry) = tile.entries.iter().find(|entry| entry.source.key() == key) else {
                return Ok(None);
            };
            let MapDefinition::Item(reference) = entry.facts.definition else {
                return Ok(None);
            };
            let Some(definition) = super::super::item_look::typed_definition(index, reference)
            else {
                return Ok(None);
            };
            // These fixed map objects have admitted identities but no carried custody
            // policy. Resolve the exact pinned Item key instead of requiring one.
            let source_id = match definition.production_key.as_str() {
                "oteryn:item.tibia.i47367" => Some(OBELISK_INACTIVE_SOURCE_ID),
                "oteryn:item.tibia.i47379" => Some(OBELISK_ACTIVE_SOURCE_ID),
                _ => None,
            };
            if !matches!(
                source_id,
                Some(OBELISK_INACTIVE_SOURCE_ID | OBELISK_ACTIVE_SOURCE_ID)
            ) {
                return Ok(None);
            }
            if fence.game_session_id != command.game_session_id
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            if source_id == Some(OBELISK_ACTIVE_SOURCE_ID) {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            if position.x != OBELISK_POSITION.x
                || position.y != OBELISK_POSITION.y
                || position.floor != OBELISK_POSITION.floor
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let current = oteryn_gameplay_domain::domain::map_position::TilePos {
                x: u16::try_from(at.at.x).map_err(|_| ())?,
                y: u16::try_from(at.at.y).map_err(|_| ())?,
                floor: i8::try_from(at.at.floor).map_err(|_| ())?,
            };
            if soulpit::entry_slot(current) != Some(0) {
                return Ok(Some(outcome(UseDisposition::TooFar)));
            }
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let source = match intent.target {
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            },
            UseTarget::ItemByDefinition(wanted) => native
                .item_index()
                .and_then(|index| index.definition_at(wanted))
                .and_then(|(key, revision)| {
                    inventory
                        .first_unreserved(
                            &crate::durability::item_mint::TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: key.into(),
                                revision_ref: revision.into(),
                            },
                            &[],
                        )
                        .ok()
                }),
            _ => None,
        };
        let Some(source) = source else {
            return Ok(Some(outcome(UseDisposition::StaleState)));
        };
        let carried = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry));
        let Some(item) = carried
            .into_iter()
            .find(|entry| entry.item.item_instance_id == source)
        else {
            return Ok(Some(outcome(UseDisposition::StaleState)));
        };
        let source_id = native
            .item_policy(
                &item.item.definition.production_key,
                &item.item.definition.revision_ref,
            )
            .and_then(|policy| {
                policy
                    .record()
                    .production_binding
                    .external_id
                    .parse::<u32>()
                    .ok()
            });
        if !source_id.is_some_and(soulpit::is_soul_core)
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(source))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        request.selected = Some(SoulpitCoreSelection {
            item_instance_id: source,
            definition: item.item.definition.clone(),
        });
        // The shared owner acquires its own complete admission lane and revalidates all
        // participants. Never retain this Channel lane while handing authority to it.
        drop(permit);
        let Some(owner) = owner else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        owner
            .enter_soulpit(self, request)
            .await?
            .map(Some)
            .ok_or(())
    }

    pub(in crate::gameplay_transport) async fn soulpit_exit_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        at: MovementLocalPosition,
    ) -> Result<(), ()> {
        let exit = soulpit::ARENA_EXIT_POSITION;
        if at.x == i32::from(exit.x)
            && at.y == i32::from(exit.y)
            && at.floor.checked_neg() == Some(i16::from(exit.floor))
            && let Some(owner) = self.states.boss_activity_handoff.get()
        {
            owner.return_from_soulpit(self, actor, session).await?;
        }
        Ok(())
    }
}
