//! Noodles uses the existing Quest writer and dynamic NPC carrier; no reward or NPC owner here.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::world_event::changes::actions::{LiveChanges, WorldActionContext};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, MovementFacing, MovementLocalPosition,
    NpcPlacement, NpcPlacementError,
};
use oteryn_gameplay_quests::quest::{noodles as quest, world_callbacks::Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}
fn local((x, y, floor): (u16, u16, i8)) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(x),
        y: i32::from(y),
        floor: -i16::from(floor),
    }
}
fn native(position: MovementLocalPosition) -> Option<Position> {
    Some(Position {
        x: u16::try_from(position.x).ok()?,
        y: u16::try_from(position.y).ok()?,
        floor: i8::try_from(-position.floor).ok()?,
    })
}

/// Read-only phase observations may choose NPC placements, never award custody or progress.
/// Both the server's existing NPC tick and a committed collar action reuse this projector.
pub fn project(
    runtime: &mut ChannelRuntimeV1,
    live: &LiveChanges,
    admitted: impl Fn(&str) -> bool,
    enterable: impl Fn(MovementLocalPosition) -> bool,
) -> Result<(), NpcPlacementError> {
    let visible = runtime.visible_entities();
    let current = |key: &str| {
        visible
            .npcs
            .iter()
            .find(|npc| runtime.npc_key(npc.actor) == Some(key))
            .map(|npc| npc.position)
    };
    let following = live
        .escort_character
        .and_then(|character| runtime.bound_character_position(&character));
    let wanted = if live.noodles.missing {
        match live.escort_character {
            Some(_) => following.map(|position| {
                current(quest::NOODLES)
                    .filter(|dog| {
                        dog.floor == position.floor
                            && dog.x.abs_diff(position.x) <= 1
                            && dog.y.abs_diff(position.y) <= 1
                    })
                    .unwrap_or(position)
            }),
            None => quest::ROAMING
                .get(usize::from(live.noodles.roaming_index))
                .copied()
                .map(local),
        }
    } else {
        Some(local(quest::PALACE))
    };
    let lady = live
        .noodles
        .dog_lady
        .then(|| local(quest::DOG_LADY_POSITION));
    for (key, wanted) in [(quest::NOODLES, wanted), (quest::DOG_LADY, lady)] {
        let positions = runtime.visible_entities();
        let held: Vec<_> = positions
            .npcs
            .iter()
            .filter(|npc| runtime.npc_key(npc.actor) == Some(key))
            .collect();
        let candidate = wanted.filter(|_| admitted(key)).and_then(|wanted| {
            [
                (0, 0),
                (0, -1),
                (1, 0),
                (0, 1),
                (-1, 0),
                (1, -1),
                (1, 1),
                (-1, 1),
                (-1, -1),
            ]
            .into_iter()
            .map(|(dx, dy)| MovementLocalPosition {
                x: wanted.x + dx,
                y: wanted.y + dy,
                ..wanted
            })
            .find(|position| {
                enterable(*position)
                    && !positions
                        .players
                        .iter()
                        .chain(&positions.creatures)
                        .any(|actor| actor.position == *position)
                    && !positions.npcs.iter().any(|npc| {
                        runtime.npc_key(npc.actor) != Some(key) && npc.position == *position
                    })
            })
        });
        if held.len() == 1 && candidate == held.first().map(|npc| npc.position) {
            continue;
        }
        for _ in held {
            runtime.remove_dynamic_npc(key)?;
        }
        if let Some(position) = candidate {
            let (Ok(x), Ok(y), Ok(floor)) = (
                u16::try_from(position.x),
                u16::try_from(position.y),
                i8::try_from(-position.floor),
            ) else {
                continue;
            };
            runtime.place_dynamic_npc(
                key,
                NpcPlacement {
                    x,
                    y,
                    floor,
                    direction: MovementFacing::South,
                },
            )?;
        }
    }
    Ok(())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_noodles_collar(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        visible_target: bool,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(named) = intent.use_with else {
            return Ok(None);
        };
        if intent.use_with_map_item.is_some() || intent.use_with_position.is_some() {
            return Ok(None);
        }
        let Some(item_fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(ItemKey::Instance(source)) = source else {
            return Ok(None);
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, item_fence)
            .await
            .map_err(|_| ())?;
        let collar = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .any(|entry| {
                entry.item.item_instance_id == source
                    && entry.item.definition.production_key == quest::COLLAR_ITEM
            });
        if !collar {
            return Ok(None);
        }
        if !visible_target
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(source))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        let mut slot = self
            .revision_sequencer
            .acquire(item_fence.character_id)
            .await;
        let Some(fence) = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
        else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        if fence.item_fence() != item_fence {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let (transition, context) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let visible = runtime.visible_entities();
            let Some(player) = visible.players.iter().find(|player| player.actor == actor) else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            let Some(npc) = visible.npcs.iter().find(|npc| {
                npc.actor.placement_identity() == named.identity
                    && npc.generation == named.generation
            }) else {
                return Ok(Some(outcome(UseDisposition::StaleState)));
            };
            let Some(key) = runtime.npc_key(npc.actor) else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            let transition = match key {
                quest::NOODLES => quest::ESCORT,
                quest::KING => quest::RETURN,
                _ => return Ok(Some(outcome(UseDisposition::NothingToUse))),
            };
            if player.position.floor != npc.position.floor
                || player.position.x.abs_diff(npc.position.x) > 1
                || player.position.y.abs_diff(npc.position.y) > 1
            {
                return Ok(Some(outcome(UseDisposition::NotPossible)));
            }
            let context = WorldActionContext {
                actor: native(player.position).ok_or(())?,
                target_npc: key.into(),
                target_position: native(npc.position).ok_or(())?,
            };
            (transition, context)
        };
        let request = QuestTransitionRequest {
            transition_key: transition.into(),
            cause: QuestCause::Use(CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            )),
        };
        self.schedule_quest_refresh(command.game_session_id);
        match slot
            .commit_world_action(
                self.root,
                self.character,
                self.holder,
                fence,
                request,
                std::sync::Arc::clone(catalogue),
                context,
            )
            .await
            .map_err(|_| ())?
        {
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_) => {
                self.refresh_quest_session(command.game_session_id, false)
                    .await;
                // Native projection is recoverable by the existing NPC clock; a
                // blocked cell cannot turn a committed World action into a refusal.
                if let Ok(Some(live)) = self
                    .root
                    .live_world_changes(item_fence.runtime_scope.world_id(), self.reset_epoch)
                    .await
                {
                    let mut runtime = self.runtime.lock().await;
                    let _ = project(
                        &mut runtime,
                        &live,
                        |key| {
                            self.states
                                .chat
                                .npc_talk()
                                .is_some_and(|book| book.get(key).is_some())
                        },
                        |position| {
                            self.movement_cells.index().lookup(
                                self.movement_cells.scope(),
                                crate::content::LogicalCell {
                                    x: position.x,
                                    y: position.y,
                                    z: i32::from(position.floor),
                                },
                            ) == Ok(crate::content::CollisionClass::Walkable)
                        },
                    );
                }
                Ok(Some(outcome(UseDisposition::Committed)))
            }
            QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed => {
                Ok(Some(outcome(UseDisposition::Rejected)))
            }
        }
    }
}
