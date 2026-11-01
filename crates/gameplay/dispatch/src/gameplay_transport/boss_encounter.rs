//! Shared boss cooldown query, composed with the admitted Character and native durability.
use super::connection::FreshAdmissionAuthority;
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::boss_encounter as wire;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn boss_cooldowns_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> super::guild_intent::GuildReply {
        use super::guild_intent::GuildReply;
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return GuildReply::Rejected;
        };
        if id == 0
            || !matches!(kind, wire::COMMAND_TYPE_BOSS_COOLDOWNS | wire::COMMAND_TYPE_BOSS_REWARD_OPEN)
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_BOSS_ENCOUNTER_V1)
            || (kind == wire::COMMAND_TYPE_BOSS_COOLDOWNS
                && wire::decode_cooldown_query(payload).is_err())
            || (kind == wire::COMMAND_TYPE_BOSS_REWARD_OPEN
                && (wire::decode_reward_chest_query(payload).is_err()
                    || (!payload.is_empty()
                        && (!admitted.continuity.selected_capabilities.contains(
                                oteryn_protocol_oteryn::boss_rewards::CAPABILITY_BOSS_REWARDS_V1)
                            || !self.offered_capabilities().iter().any(|capability|
                                capability.id == oteryn_protocol_oteryn::boss_rewards::CAPABILITY_BOSS_REWARDS_V1)))))
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            // The admitted Channel id is retained origin metadata after an activity
            // handoff. The current exact tagged scope is carried by the item fence.
            if admitted.world_id != binding.world_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
            {
                return GuildReply::Rejected;
            }
        }
        let result = if kind == wire::COMMAND_TYPE_BOSS_REWARD_OPEN {
            self.root
                .read_boss_reward_chest_page(self.character, self.holder, fence, payload)
                .await
        } else {
            self.root
                .read_boss_cooldown_list(self.character, self.holder, fence, payload)
                .await
        };
        match result {
            Ok(Some(bytes)) => GuildReply::Payload(bytes),
            Ok(None) => GuildReply::Rejected,
            Err(_) => GuildReply::Unavailable,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn boss_cooldowns_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
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
        async fn boss_cooldowns_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> crate::gameplay_transport::guild_intent::GuildReply {
            self.boss_cooldowns_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
