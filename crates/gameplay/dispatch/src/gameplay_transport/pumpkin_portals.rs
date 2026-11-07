//! Current Halloween's ordinary-World participant room; no private Instance.
use super::actor_spell::ChannelSpellStates;
use super::*;
use crate::foundation::MovementPositionSnapshot;
use crate::map::boot::BundleMap;
use crate::map::floor::{PumpkinRoomMove, prepare_pumpkin_room_transfer};
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;

#[derive(Clone, Copy)]
struct PendingEntry {
    actor: ExactActorRef,
    entered: MovementPositionSnapshot,
    occurrence: [u8; 32],
}
#[derive(Default)]
pub(super) struct PumpkinPortalSessions {
    pending: StdMutex<HashMap<GameSessionId, PendingEntry>>,
}
impl PumpkinPortalSessions {
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut entries) = self.pending.lock() {
            entries.remove(&session);
        }
    }
}
fn inside(p: crate::foundation::MovementLocalPosition) -> bool {
    p.floor == 15 && (33312..=33322).contains(&p.x) && (32410..=32420).contains(&p.y)
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn pumpkin_portal_step(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let p = entered.position();
        if (p.x, p.y, p.floor) != (33317, 32415, 7) {
            return entered;
        }
        // A presentation cache never supplies eligibility. The actual current
        // layout and exact actor step are independently checked before SQL.
        {
            let Some(map) = self.bundle_map() else {
                return entered;
            };
            let runtime = self.runtime.lock().await;
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            if prepare_pumpkin_room_transfer(
                &runtime,
                self.movement_cells,
                actor,
                session,
                entered,
                PumpkinRoomMove::Entry,
                &closed,
            )
            .is_err()
            {
                return entered;
            }
        }
        let occurrence = Sha256::new()
            .chain_update(b"oteryn:pumpkin-room-step:v1\0")
            .chain_update(actor.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(self.reset_epoch.to_be_bytes())
            .chain_update(entered.revision().to_be_bytes())
            .finalize()
            .into();
        {
            let Ok(mut pending) = self.states.pumpkin_portals.pending.lock() else {
                return entered;
            };
            if pending.try_reserve(1).is_err() {
                return entered;
            }
            pending.entry(session).or_insert(PendingEntry {
                actor,
                entered,
                occurrence,
            });
        }
        let _ = self.resume_pumpkin_portal(permit, actor, session).await;
        self.runtime
            .lock()
            .await
            .read_actor_position(actor)
            .unwrap_or(entered)
    }

    /// Reconcile the same server-owned occurrence before accepting another step.
    /// Unknown SQL retains intent only; it cannot confer a visit or a reward.
    pub(super) async fn resume_pumpkin_portal(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let pending = {
            let Ok(entries) = self.states.pumpkin_portals.pending.lock() else {
                return false;
            };
            entries.get(&session).copied()
        };
        let Some(pending) = pending else {
            return true;
        };
        if pending.actor != actor {
            self.states.pumpkin_portals.forget(session);
            return true;
        }
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(pending.entered) {
                self.states.pumpkin_portals.forget(session);
                return true;
            }
            if !runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return false;
            }
        }
        match self
            .root
            .admit_pumpkin_reward_visit(
                self.character,
                self.holder,
                fence.item_fence(),
                pending.occurrence,
            )
            .await
        {
            Err(_) => false,
            Ok(None) => {
                self.states.pumpkin_portals.forget(session);
                true
            }
            Ok(Some(_visit)) => {
                let optional_pvp = self.special_movement_policy(permit, actor, session).await;
                let Some(map) = self.bundle_map() else {
                    return false;
                };
                let mut runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if runtime.read_actor_position(actor) != Ok(pending.entered)
                    || states.is_dead(actor)
                    || !states.spell_idle(&runtime, actor, session)
                {
                    return false;
                }
                let closed = crate::map::floor::BundleDoors(map.facts.doors());
                let Ok(proof) = prepare_pumpkin_room_transfer(
                    &runtime,
                    self.movement_cells,
                    actor,
                    session,
                    pending.entered,
                    PumpkinRoomMove::Entry,
                    &closed,
                ) else {
                    return false;
                };
                let Ok(after) = runtime.commit_source_step(proof) else {
                    return false;
                };
                self.pumpkin_movement(
                    &runtime,
                    &mut states,
                    map,
                    actor,
                    session,
                    pending.entered.position(),
                    after.position(),
                    optional_pvp,
                );
                self.states.pumpkin_portals.forget(session);
                true
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn pumpkin_movement(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        before: crate::foundation::MovementLocalPosition,
        after: crate::foundation::MovementLocalPosition,
        optional_pvp: Option<bool>,
    ) {
        let now = self.owner_now().get();
        states.stage_special_movement_step(
            runtime,
            map,
            actor,
            session,
            now,
            Some(before),
            after,
            optional_pvp,
        );
        states.stage_underwater_step(
            runtime,
            actor,
            session,
            now,
            super::paced_movement::underwater_drown::underwater(map, before),
            super::paced_movement::underwater_drown::underwater(map, after),
        );
        self.states.chat.close_out_of_range(runtime, session);
    }

    pub(super) async fn pumpkin_portal_pass(&self) -> Result<(), RespawnExit> {
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        let channel = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return Err(RespawnExit::StaleOwner);
            }
            runtime.binding().channel_id()
        };
        let Some(channel) = channel else {
            return Ok(());
        };
        let open = self
            .root
            .pumpkin_reward_portal_window(self.character, self.world_id, channel)
            .await
            .ok()
            .flatten()
            .is_some();
        let _ = map.facts.project_pumpkin_portal(map.overlay.base(), open);
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(());
        };
        let pending = {
            let Ok(entries) = self.states.pumpkin_portals.pending.lock() else {
                return Ok(());
            };
            entries
                .iter()
                .map(|(session, p)| (*session, p.actor))
                .collect::<Vec<_>>()
        };
        for (session, actor) in pending {
            let _ = self.resume_pumpkin_portal(&permit, actor, session).await;
        }
        let players = {
            let runtime = self.runtime.lock().await;
            let Ok(census) = runtime.positioned_melee_census() else {
                return Ok(());
            };
            census
                .into_iter()
                .filter_map(|(actor, at, session)| {
                    inside(at.position()).then_some((actor, at, session?))
                })
                .collect::<Vec<_>>()
        };
        for (actor, expected, session) in players {
            let Ok(Some(fence)) = self.current_quest_fence(session).await else {
                continue;
            };
            let Ok(character) =
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
            else {
                continue;
            };
            match self
                .root
                .current_pumpkin_reward_visit(self.character, character, self.world_id)
                .await
            {
                Err(_) | Ok(Some(_)) => continue,
                Ok(None) => {}
            }
            let optional_pvp = self.special_movement_policy(&permit, actor, session).await;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if runtime.owner_fence().is_err() {
                return Err(RespawnExit::StaleOwner);
            }
            if runtime.read_actor_position(actor) != Ok(expected)
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, session)
            {
                continue;
            }
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            let Ok(proof) = prepare_pumpkin_room_transfer(
                &runtime,
                self.movement_cells,
                actor,
                session,
                expected,
                PumpkinRoomMove::Return,
                &closed,
            ) else {
                continue;
            };
            let Ok(after) = runtime.commit_source_step(proof) else {
                continue;
            };
            self.pumpkin_movement(
                &runtime,
                &mut states,
                map,
                actor,
                session,
                expected.position(),
                after.position(),
                optional_pvp,
            );
            self.states
                .chat
                .push_quest_feedback(session, "You can't bear staying in this room any longer.");
        }
        Ok(())
    }

    pub(super) async fn run_pumpkin_portals(&self, stop: &CancellationToken) -> RespawnExit {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            if super::world_respawns::stopping_first(interval.tick(), stop)
                .await
                .is_none()
            {
                return RespawnExit::Shutdown;
            }
            if let Err(exit) = self.pumpkin_portal_pass().await {
                return exit;
            }
        }
    }
}
