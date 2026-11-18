//! Forgotten Knowledge replica levers, Crystal 201189f actions_servants_mechanism.lua.
use super::*;
use crate::durability::quest_world_callback::replica_window::{ReplicaKind, ReplicaPortalAccess};

pub(super) const REPLICA_PORTAL: TilePos = TilePos {
    x: 32815,
    y: 32870,
    floor: -13,
};
pub(super) const REPLICA_CHAMBER: TilePos = TilePos {
    x: 32760,
    y: 32876,
    floor: -14,
};
const REPLICA_PORTAL_REFUSAL: TilePos = TilePos {
    x: 32815,
    y: 32872,
    floor: -13,
};

pub(super) fn replica_portal_destination(access: ReplicaPortalAccess) -> Option<TilePos> {
    match access {
        ReplicaPortalAccess::Dormant => None,
        ReplicaPortalAccess::InsufficientEnergy | ReplicaPortalAccess::LloydCooldown => {
            Some(REPLICA_PORTAL_REFUSAL)
        }
        ReplicaPortalAccess::Ready => Some(REPLICA_CHAMBER),
    }
}

fn clear_replicas(runtime: &mut ChannelRuntimeV1, map: &BundleMap, live: [bool; 2], now: u64) {
    use sha2::{Digest, Sha256};
    let Ok(census) = runtime.current_live_creature_census() else {
        return;
    };
    for actor in census {
        let Some(at) = runtime
            .read_actor_position(actor)
            .ok()
            .and_then(|p| native(p.position()))
        else {
            continue;
        };
        if at.floor != -13 || at.x.abs_diff(32815) > 63 || at.y.abs_diff(32874) > 63 {
            continue;
        }
        let Ok(identity) = runtime.creature_spell_target_atom(actor) else {
            continue;
        };
        let clear = match identity.as_str() {
            "oteryn:creature.diamond_servant_replica" => !live[ReplicaKind::Diamond.index()],
            "oteryn:creature.golden_servant_replica" => !live[ReplicaKind::Golden.index()],
            "oteryn:creature.iron_servant_replica" => {
                // Replaying the same owner deadline cannot redraw the 40% cleanup.
                let draw = Sha256::new()
                    .chain_update(b"oteryn:replica-cleanup:v1")
                    .chain_update(runtime.content_pin().server_artifact_digest())
                    .chain_update(actor.placement_identity())
                    .chain_update(now.to_be_bytes())
                    .finalize();
                u16::from_be_bytes([draw[0], draw[1]]) % 100 < 40
            }
            _ => false,
        };
        if clear && runtime.remove_encounter_actor(actor).is_ok() {
            map.facts.emit_movement_effect(at, 3);
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn expire_replica_windows(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        now: u64,
    ) {
        let Ok(mut deadlines) = self.states.quest_callbacks.replica_deadlines.lock() else {
            return;
        };
        let mut due = None;
        for deadline in deadlines.iter_mut() {
            if let Some(expired) = *deadline
                && expired <= now
            {
                due = Some(due.map_or(expired, |earliest: u64| earliest.min(expired)));
                *deadline = None;
            }
        }
        if let Some(due) = due {
            clear_replicas(
                runtime,
                map,
                deadlines.map(|deadline| deadline.is_some()),
                due,
            );
        }
    }

    pub(super) async fn replica_portal_access(
        &self,
        session: GameSessionId,
    ) -> Option<ReplicaPortalAccess> {
        let now = self.owner_now().get();
        let fence = self.current_quest_fence(session).await.ok()??;
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())?;
        let (access, remaining) = self
            .root
            .read_replica_portal(self.character, self.holder, fence, native.source_digest())
            .await
            .ok()?;
        if remaining > 0 {
            let map = self.bundle_map()?;
            let due = now.checked_add(remaining)?;
            let has_source_cells = [REPLICA_PORTAL, REPLICA_CHAMBER]
                .into_iter()
                .all(|pos| map.overlay.base().tile(pos.x, pos.y, pos.floor).is_some());
            if !has_source_cells
                || !map.facts.project_timed_quest_portal(
                    REPLICA_PORTAL,
                    REPLICA_CHAMBER,
                    10840,
                    self.owner_now().get(),
                    due,
                )
            {
                return None;
            }
        }
        Some(access)
    }

    pub(in crate::gameplay_transport) async fn project_replica_portal_after_death(
        &self,
        creature: &str,
        session: GameSessionId,
    ) {
        if matches!(
            creature,
            "oteryn:creature.diamond_servant_replica" | "oteryn:creature.golden_servant_replica"
        ) {
            self.replica_portal_access(session).await;
        }
    }

    pub(super) fn announce_replica_portal(
        &self,
        map: &BundleMap,
        session: GameSessionId,
        access: ReplicaPortalAccess,
        destination: TilePos,
    ) {
        let refusal = match access {
            ReplicaPortalAccess::Ready => {
                map.facts.emit_movement_effect(destination, 12);
                return;
            }
            ReplicaPortalAccess::InsufficientEnergy => {
                "Seems that you don't absorb enough energy to use this portal."
            }
            ReplicaPortalAccess::LloydCooldown => "You have to wait to challenge this enemy again!",
            ReplicaPortalAccess::Dormant => return,
        };
        map.facts.emit_movement_effect(REPLICA_PORTAL, 11);
        map.facts.emit_movement_effect(destination, 11);
        self.states.chat.push_quest_feedback(session, refusal);
    }

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
            let now = self.owner_now().get();
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
                Ok(Some(windows)) => {
                    let mut runtime = self.runtime.lock().await;
                    if runtime.owner_fence().is_err() {
                        return UseDisposition::Rejected;
                    }
                    let Ok(mut deadlines) = self.states.quest_callbacks.replica_deadlines.lock()
                    else {
                        return UseDisposition::Rejected;
                    };
                    *deadlines = windows.map(|remaining| {
                        remaining.and_then(|remaining| now.checked_add(remaining))
                    });
                    let mut previous = deadlines.map(|deadline| deadline.is_some());
                    previous[kind.index()] = false;
                    clear_replicas(&mut runtime, map, previous, now);
                }
                Ok(None) => {
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
