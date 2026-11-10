//! Crystal ff7ede59 svargrond_arena.lua: pillar=1841, teleport=5022.
use super::*;
use crate::map::overlay::TilePos;

pub(super) fn pillar(pit: u8) -> Option<TilePos> {
    let at = center(pit)?;
    Some(TilePos {
        x: (at.x - 6).try_into().ok()?,
        y: at.y.try_into().ok()?,
        floor: -7,
    })
}

pub(super) fn owns_exit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    at: MovementLocalPosition,
) -> bool {
    owner.states.arena_matches.live.lock().is_ok_and(|matches| {
        matches.get(&session).is_some_and(|live| {
            live.actor == actor && live.state.phase == 2 && live.deadline > Instant::now()
                && pillar(live.state.pit).is_some_and(|p| {
                    at.x == i32::from(p.x) && at.y == i32::from(p.y) && at.floor == -i16::from(p.floor)
                })
        })
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_arena_exit(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: crate::foundation::MovementPositionSnapshot,
    ) -> crate::foundation::MovementPositionSnapshot {
        if !owns_exit(self, actor, session, entered.position()) { return entered; }
        let Some(command) = super::super::quest_callbacks::step_command(actor, session, entered.revision(), self.reset_epoch) else { return entered; };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else { return entered; };
        let caller = {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            let Some(channel) = binding.channel_id() else { return entered; };
            ArenaCaller { actor, supplied: fence.item_fence(), world: binding.world_id(), channel }
        };
        let cause = durable::MatchCause::Step { command, actor: actor.placement_identity(), revision: entered.revision(), epoch: self.reset_epoch };
        let Ok(payload) = wire::encode_match_request(A::Advance) else { return entered; };
        let _ = self.arena_match_native(caller, cause, A::Advance, &payload, Some(entered)).await;
        self.runtime.lock().await.read_actor_position(actor).unwrap_or(entered)
    }
}
