//! Existing creature-targeted USE with the current carried tool and exact actor.
use super::super::super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_ref_admission::pinned_index,
    item_view::ItemKey,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_encounter_tool_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let (Some(wanted), Some(fence)) = (intent.use_with, command.item_fence) else {
            return Ok(None);
        };
        if !matches!(
            intent.target,
            UseTarget::Item(_) | UseTarget::ItemByDefinition(_)
        ) || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
            || fence.game_session_id != command.game_session_id
        {
            return Ok(None);
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return Ok(None);
        };
        let _revision = self.revision_sequencer.acquire(fence.character_id).await;
        let mut runtime = self.runtime.lock().await;
        let Some(index) = pinned_index(&runtime, self.active_generation) else {
            return Ok(None);
        };
        let Some(boss) = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|creature| {
                creature.actor.placement_identity() == wanted.identity
                    && creature.generation == wanted.generation
            })
            .map(|creature| creature.actor)
        else {
            return Ok(None);
        };
        let appearance = {
            let Ok(attack) = self.states.attack.try_lock() else {
                return Ok(None);
            };
            let mut capture = attack.kills().1.lock().map_err(|_| ())?;
            let Some(fight) = capture
                .outcomes
                .for_tool(&runtime, boss)
                .and_then(|run| run.shield.as_ref())
            else {
                return Ok(None);
            };
            fight.rule.tool_appearance
        };
        let key = format!("oteryn:item.tibia.i{appearance}");
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let inventory = match self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        {
            Ok(inventory) => inventory,
            Err(crate::durability::item_use::ItemUseError::Unavailable(_)) => return Err(()),
            Err(_) => return Ok(outcome(UseDisposition::StaleState)),
        };
        let selected = match intent.target {
            UseTarget::Item(_) => {
                let Some(ItemKey::Instance(id)) = source else {
                    return Ok(None);
                };
                inventory
                    .backpack
                    .iter()
                    .flat_map(|bag| &bag.entries)
                    .chain(inventory.nested.iter().map(|(_, entry)| entry))
                    .map(|entry| &entry.item)
                    .find(|item| {
                        item.item_instance_id == id && item.definition.production_key == key
                    })
                    .filter(|item| {
                        index
                            .definition_ref(
                                &item.definition.family,
                                &item.definition.production_key,
                                &item.definition.revision_ref,
                            )
                            .is_some()
                    })
                    .map(|item| item.item_instance_id)
            }
            UseTarget::ItemByDefinition(ordinal) => {
                let Some((definition, revision)) = index.definition_at(ordinal) else {
                    return Ok(None);
                };
                if definition != key {
                    return Ok(None);
                }
                inventory
                    .first_unreserved(
                        &TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: definition.into(),
                            revision_ref: revision.into(),
                        },
                        &[],
                    )
                    .ok()
            }
            _ => return Ok(None),
        };
        let Some(item) = selected else {
            return Ok(None);
        };
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(item))
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let states = self.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let Ok(from) = runtime.read_actor_position(actor) else {
            return Ok(outcome(UseDisposition::StaleState));
        };
        let Ok(to) = runtime.read_actor_position(boss) else {
            return Ok(outcome(UseDisposition::NoTarget));
        };
        let (from, to) = (from.position(), to.position());
        if from.floor != to.floor || from.x.abs_diff(to.x) > 1 || from.y.abs_diff(to.y) > 1 {
            return Ok(outcome(UseDisposition::TooFar));
        }
        let Ok(attack) = self.states.attack.try_lock() else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let mut capture = attack.kills().1.lock().map_err(|_| ())?;
        let Some(run) = capture.outcomes.for_tool(&runtime, boss) else {
            return Ok(outcome(UseDisposition::NoTarget));
        };
        if !run
            .room
            .as_ref()
            .is_some_and(|room| super::super::super::quest_callbacks::room_contains(room, from))
        {
            return Ok(outcome(UseDisposition::NotPossible));
        }
        let Some(fight) = run.shield.as_mut() else {
            return Ok(outcome(UseDisposition::NoTarget));
        };
        if fight
            .use_tool(
                &mut runtime,
                boss,
                command.command_id,
                command.game_session_id,
                self.owner_now().get() / 1_000,
            )
            .is_err()
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        states.owner_wake.notify_one();
        Ok(outcome(UseDisposition::Committed))
    }
}
