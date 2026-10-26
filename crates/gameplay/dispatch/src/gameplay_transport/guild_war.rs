use super::{ComposedFreshAdmission, connection::AdmittedSession, guild_intent::GuildReply};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::guild_war::{self as wire, GuildWarIntent};
use sha2::{Digest, Sha256};

pub(super) fn owns(kind: u32) -> bool {
    kind == wire::COMMAND_TYPE_GUILD_WAR_INTENT
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn guild_war_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> GuildReply {
        let Some(fence) = admitted.item_fence else {
            return GuildReply::Rejected;
        };
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_GUILD_WAR_V1)
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || admitted.world_id != binding.world_id()
            || admitted.channel_id != binding.channel_id()
            || fence.runtime_scope
                != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !admitted.runtime_actor.is_some_and(|actor| {
                runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            })
        {
            return GuildReply::Rejected;
        }
        drop(runtime);
        let Ok(intent) = wire::decode_guild_war_intent(payload) else {
            return GuildReply::Rejected;
        };
        let Ok(command_id) = CommandId::new(id) else {
            return GuildReply::Rejected;
        };
        let command = CommandRef::new(admitted.game_session_id, command_id);
        let derive =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id);
        let war = match intent {
            GuildWarIntent::Declare { .. } => derive(b"oteryn:guild-war:v1"),
            GuildWarIntent::Accept { war } => war,
        };
        let Some(now) = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
        else {
            return GuildReply::Unavailable;
        };
        match self
            .root
            .commit_guild_war(
                self.character,
                self.holder,
                fence,
                command,
                Sha256::digest(payload).into(),
                intent,
                war,
                now,
            )
            .await
        {
            Ok(result) => wire::encode_guild_war_result(&result)
                .map_or(GuildReply::Unavailable, GuildReply::Payload),
            Err(crate::durability::guild::GuildError::Unavailable(_)) => GuildReply::Unavailable,
            Err(_) => GuildReply::Rejected,
        }
    }
}
