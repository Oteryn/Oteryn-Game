//! W21 window actions under the admitted runtime and durable character/session owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession, guild_intent::GuildReply};
use oteryn_protocol_oteryn::boss_difficulty as wire;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn boss_difficulty_wire(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        payload: &[u8],
    ) -> GuildReply {
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return GuildReply::Rejected;
        };
        if command_id == 0
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_BOSS_DIFFICULTY_V1)
            || wire::decode_request(payload).is_err()
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || fence.runtime_scope.world_id() != admitted.world_id
        {
            return GuildReply::Rejected;
        }
        {
            let runtime = self.runtime.lock().await;
            // Compare the tagged scope against the actual owner. A Channel UUID cannot stand
            // in for an Instance UUID, and a valid foreign actor cannot borrow this runtime.
            if !runtime.owner_fence().is_ok_and(|owner| {
                owner.is_current_for_scope(fence.runtime_scope, fence.scope_ownership_generation)
            }) || !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return GuildReply::Rejected;
            }
        }
        match self
            .root
            .boss_difficulty_command(self.character, self.holder, fence, command_id, payload)
            .await
        {
            Ok(Some(reply)) => GuildReply::Payload(reply),
            Ok(None) => GuildReply::Rejected,
            Err(_) => GuildReply::Unavailable,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn boss_difficulty_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_id: u64,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::guild_intent::GuildReply>
        {
            async { $crate::gameplay_transport::guild_intent::GuildReply::Rejected }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn boss_difficulty_command(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
            payload: &[u8],
        ) -> crate::gameplay_transport::guild_intent::GuildReply {
            self.boss_difficulty_wire(admitted, command_id, payload)
                .await
        }
    };
}
pub(super) use composed_methods;
