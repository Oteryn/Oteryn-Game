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
        // Capability 492: progress windows of a pity boss carry the Moonsilver tracker.
        let pity = admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_BOSS_DIFFICULTY_PITY_V1);
        match self
            .root
            .boss_difficulty_command_with_pity(
                self.character,
                self.holder,
                fence,
                command_id,
                payload,
                pity,
            )
            .await
        {
            Ok(Some(reply)) => {
                if let Ok(wire::Request::Start { context, .. }) = wire::decode_request(payload)
                    && let Ok((wire::Reply::Window { boss_key, .. }, _)) =
                        wire::decode_reply_with_pity(&reply)
                {
                    let Ok(key) =
                        oteryn_protocol_oteryn::boss_encounter::encode_boss_key(&boss_key)
                    else {
                        return GuildReply::Rejected;
                    };
                    let Some((status, _)) =
                        self.boss_activity_start(admitted, command_id, &key).await
                    else {
                        return GuildReply::Unavailable;
                    };
                    let outcome = if status == crate::foundation::CommandStatus::Accepted {
                        wire::Reply::Closed { context }
                    } else {
                        wire::Reply::Refused {
                            context: Some(context),
                            reason: wire::Refusal::AdmissionRefused,
                        }
                    };
                    return match wire::encode_reply(&outcome) {
                        Ok(bytes) => GuildReply::Payload(bytes),
                        Err(_) => GuildReply::Unavailable,
                    };
                }
                GuildReply::Payload(reply)
            }
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
