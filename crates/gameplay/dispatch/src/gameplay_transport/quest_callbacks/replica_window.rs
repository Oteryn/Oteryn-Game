//! Forgotten Knowledge replica levers, Crystal 201189f actions_servants_mechanism.lua.
use super::*;
use crate::durability::quest_world_callback::replica_window::ReplicaKind;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_replica_lever(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        resolved: &super::super::world_map::MapObjectTarget<()>,
        expected_revision: u64,
    ) -> Option<UseOutcome> {
        let key = resolved.placement_key;
        let kind = [(26663, ReplicaKind::Diamond), (26664, ReplicaKind::Golden)]
            .into_iter()
            .find_map(|(action, kind)| {
                map.facts
                    .quest_callback_matches(key, None, Some(action), None)
                    .then_some(kind)
            })?;
        let disposition = self
            .toggle_replica_lever(actor, command, map, resolved, expected_revision, kind)
            .await;
        Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        })
    }

    async fn toggle_replica_lever(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        resolved: &super::super::world_map::MapObjectTarget<()>,
        expected_revision: u64,
        kind: ReplicaKind,
    ) -> UseDisposition {
        let Some(permit) = self.spell_lane_permit().await else {
            return UseDisposition::Rejected;
        };
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id).await else {
            return UseDisposition::Rejected;
        };
        if command.item_fence != Some(fence.item_fence()) {
            return UseDisposition::Rejected;
        }
        let runtime = self.runtime.lock().await;
        let Ok(states) = self.states.spell_states.try_lock() else {
            return UseDisposition::Rejected;
        };
        let Ok(position) = runtime.read_actor_position(actor) else {
            return UseDisposition::Rejected;
        };
        let Some(at) = native(position.position()) else {
            return UseDisposition::Rejected;
        };
        if !super::super::map_door::reachable(at, resolved.pos) {
            return UseDisposition::TooFar;
        }
        if !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|f| f.control_loss.is_none())
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, None)
            || !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return UseDisposition::Rejected;
        }
        let key = resolved.placement_key;
        if !map.facts.quest_callback_capacity(key, expected_revision, 0) {
            return UseDisposition::StaleState;
        }
        let Some(before) = map
            .overlay
            .base()
            .tile(resolved.pos.x, resolved.pos.y, resolved.pos.floor)
            .and_then(|tile| {
                tile.ids()
                    .iter()
                    .zip(tile.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .nth(usize::from(resolved.ordinal))
            })
            .and_then(|(id, _)| map.facts.base_entry(resolved.pos, resolved.ordinal, *id))
        else {
            return UseDisposition::Rejected;
        };
        let appearance = match before.appearance_id {
            9125 => 9126,
            9126 => 9125,
            _ => return UseDisposition::NothingToUse,
        };
        let Some(after) = map.facts.quest_callback_replacement(key, appearance) else {
            return UseDisposition::Rejected;
        };
        drop(states);
        drop(runtime);
        if before.appearance_id == 9125 {
            let Some(content) = self
                .active_generation
                .and_then(|active| active.native_gameplay())
            else {
                return UseDisposition::Rejected;
            };
            match self
                .root
                .open_replica_window(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    content.source_digest(),
                    kind,
                )
                .await
            {
                Ok(true) => {}
                Ok(false) => {
                    self.states.chat.push_quest_feedback(
                        command.game_session_id,
                        "The mechanism is still active.",
                    );
                    return UseDisposition::Committed;
                }
                Err(_) => return UseDisposition::Rejected,
            }
        }
        if map
            .facts
            .apply_quest_callback(key, expected_revision, after, &[], &[], false, None)
        {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        }
    }
}
