//! GUILD-BANK-1: admitted wire -> native fenced writer. No Account/World comes from the payload.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use super::guild_intent::GuildReply;
use crate::domain::guild::GuildId;
use crate::durability::guild::{
    GuildCommand, GuildDisbandJobRequest, GuildDisbandRequest, GuildError, GuildOperationIdentity,
    GuildOutcome, GuildRequest, GuildResult,
};
use crate::durability::guild_bank::GuildBankRequest;
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::guild_bank::{self as wire, GuildBankAction, GuildBankReply};
use sha2::{Digest, Sha256};

pub(super) fn owns(kind: u32) -> bool {
    matches!(
        kind,
        wire::COMMAND_TYPE_GUILD_BANK_INTENT | wire::COMMAND_TYPE_GUILD_BANK_QUERY
    )
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn guild_bank_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
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
            .contains(wire::CAPABILITY_GUILD_BANK_V1)
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
                    .is_ok_and(|f| f.control_loss.is_none())
            })
        {
            return GuildReply::Rejected;
        }
        if kind == wire::COMMAND_TYPE_GUILD_BANK_QUERY {
            let Ok(query) = wire::decode_guild_bank_query(payload) else {
                return GuildReply::Rejected;
            };
            return reply(
                self.root
                    .query_guild_bank(self.character, self.holder, fence, query)
                    .await,
            );
        }
        if kind != wire::COMMAND_TYPE_GUILD_BANK_INTENT {
            return GuildReply::Rejected;
        }
        let Ok(intent) = wire::decode_guild_bank_intent(payload) else {
            return GuildReply::Rejected;
        };
        let Ok(command_id) = CommandId::new(id) else {
            return GuildReply::Rejected;
        };
        let command = CommandRef::new(admitted.game_session_id, command_id);
        let derive =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id);
        let Some(now) = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
        else {
            return GuildReply::Unavailable;
        };
        if let GuildBankAction::Disband(confirmation) = &intent.action {
            // Current native fence also precedes historical start receipt reads on this path.
            if let Err(e) = self
                .root
                .query_guild_bank(
                    self.character,
                    self.holder,
                    fence,
                    wire::GuildBankQuery {
                        guild: Some(intent.guild),
                        before: None,
                        after_claim: None,
                    },
                )
                .await
            {
                return failure(e);
            }
            let Ok(guild_id) = GuildId::from_bytes(intent.guild) else {
                return GuildReply::Rejected;
            };
            let common = GuildRequest {
                identity: GuildOperationIdentity {
                    occurrence: derive(b"oteryn:guild-bank-disband-occurrence:v1"),
                    transaction_id: derive(b"oteryn:guild-bank-disband-transaction:v1"),
                    event_id: derive(b"oteryn:guild-bank-disband-event:v1"),
                    occurred_at_unix_ms: now,
                    wire_digest: Some(Sha256::digest(payload).into()),
                },
                command,
                guild_id,
                operation: GuildCommand::Disband,
            };
            let Ok(request) = GuildDisbandRequest::new(common, confirmation) else {
                return GuildReply::Rejected;
            };
            let result = self
                .root
                .commit_guild_disband(self.character, self.holder, fence, request)
                .await;
            let receipt = match result {
                Ok(GuildOutcome::Committed(r) | GuildOutcome::Replayed(r)) => r,
                Err(e) => return failure(e),
            };
            let code = if receipt.result == GuildResult::Ok {
                // One bounded World step pays out. Existing guild_scheduler resumes cleanup/restart.
                if let Err(e) = self
                    .root
                    .advance_guild_disband(
                        self.character,
                        self.holder,
                        GuildDisbandJobRequest {
                            world_id: admitted.world_id,
                            guild_id,
                        },
                    )
                    .await
                {
                    return failure(e);
                }
                1
            } else if receipt.result == GuildResult::GuildDisbanding {
                3
            } else {
                2
            };
            return reply(Ok(GuildBankReply {
                code,
                bank_balance: 0,
                guild_balance: 0,
                claim: 0,
                entries: Vec::new(),
                next: None,
                claims: Vec::new(),
                next_claim: None,
            }));
        }
        reply(
            self.root
                .commit_guild_bank(
                    self.character,
                    self.holder,
                    fence,
                    GuildBankRequest {
                        operation_id: derive(b"oteryn:guild-bank-operation:v1"),
                        command,
                        occurred_at: now,
                        wire_digest: Sha256::digest(payload).into(),
                        intent,
                    },
                )
                .await,
        )
    }
}
fn reply(result: Result<GuildBankReply, GuildError>) -> GuildReply {
    match result {
        Ok(r) => {
            wire::encode_guild_bank_reply(&r).map_or(GuildReply::Unavailable, GuildReply::Payload)
        }
        Err(e) => failure(e),
    }
}
fn failure(error: GuildError) -> GuildReply {
    match error {
        GuildError::InvalidInput | GuildError::NotCurrent | GuildError::ConflictingOccurrence => {
            GuildReply::Rejected
        }
        GuildError::Unavailable(_) => GuildReply::Unavailable,
    }
}
