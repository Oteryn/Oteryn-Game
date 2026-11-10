//! Refresh the derived contribution under the current Character and physical actor fences.
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn refresh_cyclopedia_speed(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<u16> {
        {
            let mut runtime = self.runtime.lock().await;
            runtime.owner_fence().ok()?;
            runtime.set_player_discovery_speed(actor, session, 0).ok()?;
        }
        let mut fence =
            super::super::character_gameplay_fence::current(self, session, false).await?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        let (position, subareas, area) = {
            let mut runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if runtime.owner_fence().is_err()
                || binding.runtime_scope() != fence.runtime_scope
                || binding.scope_generation() != fence.scope_ownership_generation
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return None;
            }
            // Never carry an old area's contribution across a physical boundary or a failed read.
            runtime.set_player_discovery_speed(actor, session, 0).ok()?;
            let snapshot = runtime.read_actor_position(actor).ok()?;
            let Some(map_position) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)
            else {
                return Some(0);
            };
            let table = &map_position.map.overlay.base().subareas;
            let subareas = table
                .areas
                .iter()
                .filter_map(|a| a.parent.map(|parent| (parent, a.id)))
                .collect::<Vec<_>>();
            let membership = table
                .resolve(
                    u16::try_from(map_position.at.x).ok()?,
                    u16::try_from(map_position.at.y).ok()?,
                    i8::try_from(map_position.at.floor).ok()?,
                )
                .map(|(area, _)| area);
            (snapshot, subareas, membership)
        };
        let bonus = self
            .root
            .read_cyclopedia_speed(self.character, self.holder, fence, &subareas, area)
            .await
            .ok()?;
        let mut runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || runtime.read_actor_position(actor).ok()? != position
        {
            return None;
        }
        runtime
            .set_player_discovery_speed(actor, session, bonus)
            .ok()?;
        Some(bonus)
    }

    pub(super) async fn cyclopedia_speed_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> GuildReply {
        if !payload.is_empty()
            || CommandId::new(id).is_err()
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_CYCLOPEDIA_MAP_V1)
        {
            return GuildReply::Rejected;
        }
        let (Some(actor), Some(expected)) = (admitted.runtime_actor, admitted.item_fence) else {
            return GuildReply::Rejected;
        };
        let Some(current) =
            super::super::character_gameplay_fence::current(self, admitted.game_session_id, false)
                .await
        else {
            return GuildReply::Rejected;
        };
        if current.item_fence() != expected
            || current.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        self.refresh_cyclopedia_speed(actor, admitted.game_session_id)
            .await
            .map(|bonus| GuildReply::Payload(bonus.to_le_bytes().to_vec()))
            .unwrap_or(GuildReply::Rejected)
    }
}
