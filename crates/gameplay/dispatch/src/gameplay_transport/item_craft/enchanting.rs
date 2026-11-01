//! I19 use-with adapter. The craft receipt, resource save and live successor share one window.
use super::*;
use crate::durability::character_last_position::{LastPositionKey, LastVitals};
use crate::durability::item_craft::EnchantingCraft;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{ChannelRuntimeV1, GameSessionId};
use crate::spell::cast::PlayerSpellState;
use oteryn_gameplay_items::item_craft::enchanting as rules;

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingEnchanting {
    actor: ExactActorRef,
    session: GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    request: ItemCraftRequest,
    before: PlayerSpellState,
    after: PlayerSpellState,
}

fn park(pending: PendingEnchanting) -> Box<dyn std::any::Any + Send> {
    Box::new(
        super::super::actor_spell::native_combat_cast::UnresolvedSpellCommit::Enchanting(Box::new(
            pending,
        )),
    )
}

fn finish(
    pending: &PendingEnchanting,
    runtime: &ChannelRuntimeV1,
    states: &mut super::super::actor_spell::ChannelSpellStates,
    committed: bool,
) -> bool {
    if committed
        && states.get(runtime, pending.actor, pending.session) != Some(&pending.after)
        && (states.get(runtime, pending.actor, pending.session) != Some(&pending.before)
            || !states.commit(
                runtime,
                pending.actor,
                pending.session,
                pending.after.clone(),
            ))
    {
        return false;
    }
    states
        .enchanting_pending
        .retain(|pair| *pair != (pending.actor, pending.session));
    true
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Outer None declines a different source; inner None closes an uncertain acknowledgement.
    #[allow(
        clippy::too_many_lines,
        reason = "one staged use-with attempt retains its successor through the commit window"
    )]
    pub(super) async fn use_enchanting(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<Option<UseOutcome>> {
        let fence = command.item_fence?;
        let native = self.active_generation?.native_gameplay()?;
        let mut permit = self.spell_lane_permit().await?;
        let mut request = ItemCraftRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).ok()?,
            ),
            source: [0; 16],
            tool: None,
            station: None,
            gathering: None,
            enchanting: None,
            intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).ok()?,
        };
        let outcome = |disposition| {
            Some(Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            }))
        };
        match self
            .root
            .craft_item(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native),
                true,
            )
            .await
        {
            Ok(Some(result)) => return outcome(result),
            Ok(None) => {}
            Err(_) => return Some(None),
        }
        let Some(ItemKey::Instance(source)) = source else {
            return None;
        };
        let inventory = match self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        {
            Ok(i) => i,
            Err(_) => return Some(None),
        };
        let entries = inventory.backpack.iter().flat_map(|b| &b.entries);
        let source_item = entries
            .clone()
            .find(|e| e.item.item_instance_id == source)?;
        if !rules::is_gem(&source_item.item.definition.production_key) {
            return None;
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(source))
        {
            return outcome(UseDisposition::Rejected);
        }
        request.source = source;
        let target_id = match target {
            Some(ItemKey::Instance(id)) => Some(id),
            _ => None,
        };
        let target_item =
            target_id.and_then(|id| entries.clone().find(|e| e.item.item_instance_id == id));
        if target_id.is_some() && target_item.is_none() {
            return outcome(UseDisposition::StaleState);
        }
        if target_id.is_some_and(|id| {
            !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(id))
        }) {
            return outcome(UseDisposition::Rejected);
        }
        let (station, before, save) = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return outcome(UseDisposition::Rejected);
            }
            let before = states
                .get(&runtime, actor, command.game_session_id)?
                .clone();
            let position = runtime.read_actor_position(actor).ok()?.position();
            let save = self.placed_record(&runtime, position);
            let station = if let Some(target_item) = target_item {
                target_item.item.definition.clone()
            } else {
                let index = pinned_index(&runtime, self.active_generation)?;
                let at =
                    super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)?;
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
                            .map(|t| (t.pos, MapEntryKey::Base(t.placement_key)))
                    }
                    Some(MapUseTarget::Item(_)) => target
                        .and_then(|key| super::super::world_map::resolve_map_handle(&map, key).ok())
                        .and_then(|t| match t {
                            super::super::world_map::MapHandleTarget::Base {
                                pos,
                                placement_key,
                                ..
                            } => Some((pos, MapEntryKey::Base(placement_key))),
                            _ => None,
                        }),
                    None => None,
                };
                let Some((pos, key)) = resolved else {
                    return outcome(UseDisposition::StaleState);
                };
                if i16::from(pos.floor) != at.at.floor
                    || at.at.x.abs_diff(i32::from(pos.x)) > 1
                    || at.at.y.abs_diff(i32::from(pos.y)) > 1
                {
                    return outcome(UseDisposition::TooFar);
                }
                let tile = view::compose_tile(&at.map.overlay, &facts, pos).ok()??;
                let entry = tile.entries.iter().find(|e| e.source.key() == key)?;
                let MapDefinition::Item(reference) = entry.facts.definition else {
                    return outcome(UseDisposition::NothingToUse);
                };
                super::super::item_look::typed_definition(index, reference)?
            };
            (station, before, save)
        };
        let Some(rule) = rules::source_id(&source_item.item.definition.production_key)
            .zip(rules::source_id(&station.production_key))
            .and_then(|(s, t)| {
                rules::rule(
                    s,
                    t,
                    source_item.item.quantity,
                    target_item.map_or(1, |e| e.item.quantity),
                )
            })
        else {
            return outcome(UseDisposition::NothingToUse);
        };
        if (rule.mana > 0) != target_id.is_none() {
            return outcome(UseDisposition::NothingToUse);
        }
        request.station = Some(station);
        let (after, cost) = if rule.mana > 0 {
            let Some((after, cost)) =
                before.prepare_gem_enchanting(source_item.item.quantity, self.owner_now().get())
            else {
                return outcome(UseDisposition::NotPossible);
            };
            (after, Some(cost))
        } else {
            (before.clone(), None)
        };
        let (cost, save) = if rule.mana > 0 {
            let mut record = save.record;
            record.vitals = LastVitals::new(after.vitals().health, after.vitals().mana);
            record.rest = Some(after.bed_rest_snapshot(self.owner_now().get()));
            (
                cost,
                Some((
                    LastPositionKey {
                        character_lease_generation: fence.character_lease_generation,
                        write_sequence: save.write_sequence,
                    },
                    record,
                )),
            )
        } else {
            (None, None)
        };
        request.enchanting = Some(EnchantingCraft {
            target: target_id,
            cost,
            save,
        });
        let pending = PendingEnchanting {
            actor,
            session: command.game_session_id,
            fence,
            request: request.clone(),
            before,
            after,
        };
        let mut window = permit.open_commit_window(pending, park);
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if states.get(&runtime, actor, command.game_session_id)
                != Some(&window.attempt().before)
            {
                window.release();
                return outcome(UseDisposition::StaleState);
            }
            states
                .enchanting_pending
                .push((actor, command.game_session_id));
        }
        let result = self
            .root
            .craft_item_owned(
                self.character,
                self.holder,
                fence,
                request,
                native,
                &NativeItemWeights(native),
                false,
                Some(&mut window),
            )
            .await;
        match result {
            Ok(Some(result)) => {
                let runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if !finish(
                    window.attempt(),
                    &runtime,
                    &mut states,
                    result == UseDisposition::Committed,
                ) {
                    return Some(None);
                }
                window.install();
                outcome(result)
            }
            _ => Some(None),
        }
    }

    pub(in crate::gameplay_transport) async fn resolve_enchanting(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingEnchanting,
    ) {
        let window = permit.open_commit_window(pending, park);
        let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return;
        };
        let pending = window.attempt();
        let result = self
            .root
            .craft_item(
                window.permit(),
                self.character,
                self.holder,
                pending.fence,
                pending.request.clone(),
                native,
                &NativeItemWeights(native),
                true,
            )
            .await;
        if let Ok(result) = result {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if result.is_some_and(|r| r != UseDisposition::Committed) {
                return;
            }
            if finish(
                window.attempt(),
                &runtime,
                &mut states,
                result == Some(UseDisposition::Committed),
            ) {
                window.install();
            }
        }
    }
}
