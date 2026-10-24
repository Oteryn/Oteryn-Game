//! Basic Guild wire composition. Only native Guild writers decide roles and durable effects.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use crate::domain::{
    CharacterId,
    guild::{GuildId, GuildName},
};
use crate::durability::guild::{
    GuildCommand, GuildError, GuildFoundRequest, GuildOperationIdentity, GuildOutcome,
    GuildRankRequest, GuildRequest,
};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::guild::{self as wire, GuildIntent};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum GuildReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn guild_wire(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        command_type: u32,
        payload: &[u8],
    ) -> GuildReply {
        if super::guild_bank::owns(command_type) {
            return self
                .guild_bank_wire(admitted, command_id, command_type, payload)
                .await;
        }
        let Some(fence) = admitted.item_fence else {
            return GuildReply::Rejected;
        };
        // Keep Channel custody across the durable decision, as Guild chat does. The native writer
        // independently checks recovery, admission, node, session generation, roots and Guild.
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_GUILD_V1)
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
        if command_type == wire::COMMAND_TYPE_GUILD_QUERY {
            let Ok(query) = wire::decode_guild_query(payload) else {
                return GuildReply::Rejected;
            };
            // Canonical encoding also exercises the shared request encoder on the production path.
            if wire::encode_guild_query(&query).is_err() {
                return GuildReply::Rejected;
            }
            return match self
                .root
                .query_guild(self.character, self.holder, fence, query)
                .await
            {
                Ok(result) => wire::encode_guild_query_result(&result)
                    .map_or(GuildReply::Unavailable, GuildReply::Payload),
                Err(error) => failure(error),
            };
        }
        if command_type != wire::COMMAND_TYPE_GUILD_INTENT {
            return GuildReply::Rejected;
        }
        let Ok(intent) = wire::decode_guild_intent(payload) else {
            return GuildReply::Rejected;
        };
        if wire::encode_guild_intent(&intent).is_err() {
            return GuildReply::Rejected;
        }
        let Ok(id) = CommandId::new(command_id) else {
            return GuildReply::Rejected;
        };
        let command = CommandRef::new(admitted.game_session_id, id);
        let derived = |tag: &[u8]| {
            super::npc_trade_command::derived(tag, admitted.game_session_id, command_id)
        };
        let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
            return GuildReply::Unavailable;
        };
        let Ok(now) = i64::try_from(now.as_millis()) else {
            return GuildReply::Unavailable;
        };
        let identity = GuildOperationIdentity {
            occurrence: derived(b"oteryn:guild-wire-occurrence:v1"),
            transaction_id: derived(b"oteryn:guild-wire-transaction:v1"),
            event_id: derived(b"oteryn:guild-wire-event:v1"),
            occurred_at_unix_ms: now,
            wire_digest: Some(Sha256::digest(payload).into()),
        };
        let guild = match &intent {
            GuildIntent::Found { .. } => derived(b"oteryn:guild-wire-guild:v1"),
            GuildIntent::Invite { guild, .. }
            | GuildIntent::Join { guild }
            | GuildIntent::Leave { guild }
            | GuildIntent::Kick { guild, .. }
            | GuildIntent::SetRank { guild, .. }
            | GuildIntent::RevokeInvite { guild, .. } => *guild,
        };
        let Ok(guild_id) = GuildId::from_bytes(guild) else {
            return GuildReply::Rejected;
        };
        let operation = match &intent {
            GuildIntent::Found { .. } => GuildCommand::Found,
            GuildIntent::Invite { name, .. } => {
                let Ok(name) = GuildName::parse(name) else {
                    return GuildReply::Rejected;
                };
                match self
                    .root
                    .resolve_guild_invitee(self.character, self.holder, fence, command, name)
                    .await
                {
                    Ok(Some(character)) => GuildCommand::Invite(character),
                    // Unknown/other-World names get the same native NOT_ALLOWED as inviting self.
                    Ok(None) => GuildCommand::Invite(fence.character_id),
                    Err(error) => return failure(error),
                }
            }
            GuildIntent::Join { .. } => GuildCommand::Accept,
            GuildIntent::Leave { .. } => GuildCommand::Leave,
            GuildIntent::Kick { character, .. } => match CharacterId::from_bytes(*character) {
                Ok(c) => GuildCommand::Exclude(c),
                Err(_) => return GuildReply::Rejected,
            },
            GuildIntent::SetRank { character, .. } => match CharacterId::from_bytes(*character) {
                Ok(c) => GuildCommand::SetRank(c),
                Err(_) => return GuildReply::Rejected,
            },
            GuildIntent::RevokeInvite { character, .. } => {
                match CharacterId::from_bytes(*character) {
                    Ok(c) => GuildCommand::Revoke(c),
                    Err(_) => return GuildReply::Rejected,
                }
            }
        };
        let request = GuildRequest {
            identity,
            command,
            guild_id,
            operation,
        };
        let outcome = match intent {
            GuildIntent::Found { name } => {
                let Ok(name) = GuildName::parse(&name) else {
                    return wire::encode_guild_intent_result(&wire::GuildIntentResult {
                        code: 14,
                        guild,
                        revision: None,
                    })
                    .map_or(GuildReply::Unavailable, GuildReply::Payload);
                };
                let Some(premium) = self.premium_coordinator else {
                    return GuildReply::Unavailable;
                };
                self.root
                    .commit_guild_found(
                        self.character,
                        self.holder,
                        fence,
                        GuildFoundRequest {
                            identity: request.identity,
                            command,
                            guild_id,
                            name,
                        },
                        premium,
                    )
                    .await
            }
            GuildIntent::SetRank {
                level,
                expected_revision,
                ..
            } => {
                let Ok(rank) = GuildRankRequest::new(request, expected_revision, level) else {
                    return GuildReply::Rejected;
                };
                let Some(premium) = self.premium_coordinator else {
                    return GuildReply::Unavailable;
                };
                self.root
                    .commit_guild_rank(self.character, self.holder, fence, rank, premium)
                    .await
            }
            _ => {
                self.root
                    .commit_guild_membership(self.character, self.holder, fence, request)
                    .await
            }
        };
        match outcome {
            Ok(GuildOutcome::Committed(receipt) | GuildOutcome::Replayed(receipt)) => {
                let Ok(code) = u8::try_from(receipt.result.code() + 1) else {
                    return GuildReply::Unavailable;
                };
                wire::encode_guild_intent_result(&wire::GuildIntentResult {
                    code,
                    guild,
                    revision: receipt.revision,
                })
                .map_or(GuildReply::Unavailable, GuildReply::Payload)
            }
            Err(error) => failure(error),
        }
    }
}
fn failure(error: GuildError) -> GuildReply {
    match error {
        GuildError::InvalidInput | GuildError::NotCurrent | GuildError::ConflictingOccurrence => {
            GuildReply::Rejected
        }
        // Never sequence an unknown commit as a rejection. Disconnect; the retained command
        // replays its immutable native receipt after reconnect, including after process restart.
        GuildError::Unavailable(_) => GuildReply::Unavailable,
    }
}
macro_rules! authority_methods {
    () => {
        fn guild_command(
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
        async fn guild_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> guild_intent::GuildReply {
            self.guild_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
