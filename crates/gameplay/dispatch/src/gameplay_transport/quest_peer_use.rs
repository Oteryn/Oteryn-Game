//! Resolve peer quest predicates from an exact, nearby live player, never client IDs alone.
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::durability::reward_claim_mint::quest_peer::QuestPeerUseProof;
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::world_object::UseIntent;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn quest_peer_use(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
    ) -> Result<Option<QuestPeerUseProof>, ()> {
        let Some(target) = intent.use_with else {
            return Ok(None);
        };
        let session = {
            let runtime = self.runtime.lock().await;
            let own = runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .position();
            let (peer, position, session) = runtime
                .positioned_actor_census()
                .map_err(|_| ())?
                .into_iter()
                .find(|(peer, _, session)| {
                    session.is_some()
                        && peer.placement_identity() == target.identity
                        && peer.actor_local_generation() == target.generation
                })
                .ok_or(())?;
            let session = session.ok_or(())?;
            let position = position.position();
            if peer == actor
                || session == command.game_session_id
                || peer.runtime_scope() != actor.runtime_scope()
                || position.floor != own.floor
                || position.x.abs_diff(own.x) > 1
                || position.y.abs_diff(own.y) > 1
            {
                return Err(());
            }
            runtime
                .player_control_facts(peer, session)
                .map_err(|_| ())?;
            if self.states.spell_states.lock().await.is_dead(peer) {
                return Err(());
            }
            session
        };
        let fence = self.current_quest_fence(session).await?.ok_or(())?;
        if command.item_fence.is_none_or(|own| {
            own.runtime_scope != fence.runtime_scope
                || own.scope_ownership_generation != fence.scope_ownership_generation
        }) {
            return Err(());
        }
        Ok(Some(QuestPeerUseProof { fence, target }))
    }
}
