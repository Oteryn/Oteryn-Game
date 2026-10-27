//! Leadership transfer delegates to the existing fenced Guild/Premium writer.
use super::{ComposedFreshAdmission, connection::AdmittedSession, guild_intent::GuildReply};
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::guild::{
    GuildCommand, GuildError, GuildOperationIdentity, GuildOutcome, GuildRequest,
    GuildResignRequest,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::{guild, house};
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    // Called under guild_wire's current Channel/controller/connection fence.
    pub(super) async fn guildhall_leadership(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> GuildReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(house::CAPABILITY_GUILDHALL_V1)
        {
            return GuildReply::Rejected;
        }
        let (Some(fence), Some(premium), Ok(intent), Ok(command)) = (
            admitted.item_fence,
            self.premium_coordinator,
            house::decode_guildhall_leadership(payload),
            CommandId::new(id),
        ) else {
            return GuildReply::Rejected;
        };
        let (Ok(guild_id), Ok(successor)) = (
            GuildId::from_bytes(intent.guild),
            CharacterId::from_bytes(intent.successor),
        ) else {
            return GuildReply::Rejected;
        };
        let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
            return GuildReply::Unavailable;
        };
        let Ok(now) = i64::try_from(now.as_millis()) else {
            return GuildReply::Unavailable;
        };
        let derived =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id);
        let common = GuildRequest {
            identity: GuildOperationIdentity {
                occurrence: derived(b"oteryn:guildhall-leadership-occurrence:v1"),
                transaction_id: derived(b"oteryn:guildhall-leadership-transaction:v1"),
                event_id: derived(b"oteryn:guildhall-leadership-event:v1"),
                occurred_at_unix_ms: now,
                wire_digest: Some(Sha256::digest(payload).into()),
            },
            command: CommandRef::new(admitted.game_session_id, command),
            guild_id,
            operation: GuildCommand::Resign(successor),
        };
        let Ok(request) = GuildResignRequest::new(common, intent.expected_revision) else {
            return GuildReply::Rejected;
        };
        match self
            .root
            .commit_guild_resign(self.character, self.holder, fence, request, premium)
            .await
        {
            Ok(GuildOutcome::Committed(receipt) | GuildOutcome::Replayed(receipt)) => {
                let Ok(code) = u8::try_from(receipt.result.code() + 1) else {
                    return GuildReply::Unavailable;
                };
                guild::encode_guild_intent_result(&guild::GuildIntentResult {
                    code,
                    guild: intent.guild,
                    revision: receipt.revision,
                })
                .map_or(GuildReply::Unavailable, GuildReply::Payload)
            }
            Err(GuildError::Unavailable(_)) => GuildReply::Unavailable,
            Err(_) => GuildReply::Rejected,
        }
    }
}
