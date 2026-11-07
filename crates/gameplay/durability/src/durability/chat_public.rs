//! Current Control-issued channel roles and session-fenced Help moderation.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::character_name::CharacterName;
pub use oteryn_gameplay_social_rules::public_channels::{
    ChatRole, HelpAction, HelpMute, HelpRefusal,
};
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntentResult};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone, Default)]
pub struct PublicChatAccess {
    pub role: ChatRole,
    pub guild_leader: bool,
    pub guild_name: Option<String>,
    pub help_wait_seconds: u32,
}

#[derive(Debug)]
pub struct HelpCommandOutcome {
    pub result: ChatIntentResult,
    pub target_name: Option<String>,
    /// A retained replay must not announce the same transition twice.
    pub applied: bool,
}

/// A retained command is resolved before current role, mute or room admission gates.
#[derive(Debug)]
pub enum HelpReceipt {
    Unrecorded,
    Retained(ChatIntentResult),
    StaleFence,
}

fn result(disposition: ChatDisposition) -> ChatIntentResult {
    ChatIntentResult {
        disposition,
        wait_seconds: 0,
    }
}

async fn active(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    node: &NodeIncarnationProof,
) -> Result<bool, DurabilityError> {
    if assert_session_fence(tx, fence, node).await?.is_err() {
        return Ok(false);
    }
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT session_state=2 AND current_transport_ref IS NOT NULL AND control_loss_epoch IS NULL \
         FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid",
    ).bind(fence.game_session_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?.unwrap_or(false))
}

async fn role(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8],
) -> Result<ChatRole, DurabilityError> {
    // The Control issuer takes the exclusive form, including for a previously missing row.
    sqlx::query(
        "SELECT pg_advisory_xact_lock_shared(hashtextextended((encode($1,'hex')::uuid)::text,717))",
    )
    .bind(character)
    .execute(&mut **tx)
    .await?;
    // The advisory lock supplies custody; row locking would require UPDATE and violate
    // the coordinator's runtime-SELECT-only role boundary.
    let row = sqlx::query(
        "SELECT group_id,account_type,group_access FROM game_chat_roles \
        WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?;
    match row {
        None => Ok(ChatRole::default()),
        Some(row) => Ok(ChatRole {
            group_id: u8::try_from(row.try_get::<i16, _>("group_id")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            account_type: u8::try_from(row.try_get::<i16, _>("account_type")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            access: row.try_get("group_access")?,
        }),
    }
}

async fn db_now(tx: &mut Transaction<'_, Postgres>) -> Result<u64, DurabilityError> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)
}

async fn help_mute(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8],
) -> Result<HelpMute, DurabilityError> {
    let row = sqlx::query(
        "SELECT condition_until_ms,speech_until_ms FROM game_chat_help_mutes \
        WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?;
    match row {
        None => Ok(HelpMute::default()),
        Some(row) => Ok(HelpMute {
            condition_until_ms: u64::try_from(row.try_get::<i64, _>("condition_until_ms")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            speech_until_ms: u64::try_from(row.try_get::<i64, _>("speech_until_ms")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        }),
    }
}

fn help_digest(action: HelpAction, target: &CharacterName) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([match action {
        HelpAction::Mute => 1,
        HelpAction::Unmute => 2,
    }]);
    hasher.update(target.comparison_key().as_bytes());
    hasher.finalize().into()
}

fn help_request_digest(action: HelpAction, target: &str) -> [u8; 32] {
    if let Ok(target) = CharacterName::parse(target) {
        return help_digest(action, &target);
    }
    // Keep the established digest for valid names. The non-UTF8 marker separates
    // malformed-name refusals from every valid normalized Character name.
    let mut hasher = Sha256::new();
    hasher.update([
        match action {
            HelpAction::Mute => 1,
            HelpAction::Unmute => 2,
        },
        0xff,
    ]);
    hasher.update(target.as_bytes());
    hasher.finalize().into()
}

async fn lock_command(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command_id: u64,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex')||':'||$2,717))")
        .bind(fence.game_session_id.as_bytes().as_slice())
        .bind(command_id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn retained_result(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command_id: u64,
    digest: &[u8; 32],
) -> Result<Option<ChatIntentResult>, DurabilityError> {
    let row = sqlx::query(
        "SELECT request_digest,disposition,wait_seconds FROM game_chat_help_receipts \
        WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(command_id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<Vec<u8>, _>("request_digest")?.as_slice() != digest {
        return Ok(Some(result(ChatDisposition::Rejected)));
    }
    let disposition = match row.try_get::<i16, _>("disposition")? {
        1 => ChatDisposition::Ok,
        2 => ChatDisposition::Muted,
        3 => ChatDisposition::Exhausted,
        4 => ChatDisposition::LevelTooLow,
        5 => ChatDisposition::NotOnline,
        6 => ChatDisposition::NoVocation,
        7 => ChatDisposition::RoomNotOpen,
        8 => ChatDisposition::ChatUnavailable,
        9 => ChatDisposition::Rejected,
        10 => ChatDisposition::NotAllowed,
        11 => ChatDisposition::NotInvited,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(Some(ChatIntentResult {
        disposition,
        wait_seconds: u32::try_from(row.try_get::<i32, _>("wait_seconds")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    }))
}

async fn record_result(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command_id: u64,
    digest: &[u8; 32],
    result: ChatIntentResult,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_chat_help_receipts(game_session_id,command_id,connection_generation, \
        request_digest,disposition,wait_seconds) VALUES(encode($1,'hex')::uuid,$2::text::numeric, \
        $3::text::numeric,$4,$5,$6)",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(command_id.to_string())
    .bind(fence.connection_generation.get().to_string())
    .bind(digest.as_slice())
    .bind(result.disposition as i16)
    .bind(i32::try_from(result.wait_seconds).map_err(|_| DurabilityError::InvalidStoredState)?)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl DurabilityRoot {
    /// Settles an uncertain Help commit without re-running mutable channel policy.
    /// The original session/connection fence remains mandatory even for a receipt.
    pub async fn read_help_channel_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        action: HelpAction,
        target: String,
    ) -> Result<HelpReceipt, DurabilityError> {
        if command_id == 0 {
            return Ok(HelpReceipt::StaleFence);
        }
        if target.len() > oteryn_protocol_oteryn::chat::MAX_CHAT_TEXT_BYTES {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let digest = help_request_digest(action, &target);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !active(&mut tx, &fence, &node).await? {
                        return Ok(HelpReceipt::StaleFence);
                    }
                    lock_command(&mut tx, &fence, command_id).await?;
                    let result = retained_result(&mut tx, &fence, command_id, &digest).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(match result {
                        Some(result) => HelpReceipt::Retained(result),
                        None => HelpReceipt::Unrecorded,
                    })
                })
            })
            .await
    }

    /// A recognized moderator command's terminal admission refusal shares the mutation
    /// journal. A lost reply cannot become a successful mute when the refusal clears.
    pub async fn refuse_help_channel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        request: (HelpAction, String),
        refusal: ChatIntentResult,
    ) -> Result<Option<ChatIntentResult>, DurabilityError> {
        let (action, target) = request;
        if command_id == 0
            || refusal.disposition == ChatDisposition::Ok
            || target.len() > oteryn_protocol_oteryn::chat::MAX_CHAT_TEXT_BYTES
            || oteryn_protocol_oteryn::chat::encode_chat_intent_result(&refusal).is_err()
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let digest = help_request_digest(action, &target);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !active(&mut tx, &fence, &node).await? {
                        return Ok(None);
                    }
                    lock_command(&mut tx, &fence, command_id).await?;
                    let result = match retained_result(&mut tx, &fence, command_id, &digest).await?
                    {
                        Some(result) => result,
                        None => {
                            record_result(&mut tx, &fence, command_id, &digest, refusal).await?;
                            refusal
                        }
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(result))
                })
            })
            .await
    }

    /// Re-proves the original connection generation; no session-id-only authority refresh.
    pub async fn read_public_chat_access(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<Option<PublicChatAccess>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !active(&mut tx, &fence, &node).await? {
                        return Ok(None);
                    }
                    let character = fence.character_id.as_bytes().as_slice();
                    let role = role(&mut tx, character).await?;
                    // Point lookup on the existing Guild membership owner, bounded by character PK.
                    let guild = sqlx::query(
                        "SELECT m.rank_level,g.name FROM game_guild_members m \
                JOIN game_guilds g USING(guild_id) WHERE m.character_id=encode($1,'hex')::uuid \
                AND g.world_id=encode($2,'hex')::uuid AND g.state IN(1,2) LIMIT 1 FOR SHARE OF m,g",
                    )
                    .bind(character)
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let (guild_leader, guild_name) = match guild {
                        Some(guild) => (
                            guild.try_get::<i16, _>("rank_level")?
                                == i16::from(crate::domain::guild::GuildRank::LEADER.level()),
                            Some(guild.try_get("name")?),
                        ),
                        None => (false, None),
                    };
                    let now = db_now(&mut tx).await?;
                    let help_wait_seconds =
                        help_mute(&mut tx, character).await?.remaining_seconds(now);
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(PublicChatAccess {
                        role,
                        guild_leader,
                        guild_name,
                        help_wait_seconds,
                    }))
                })
            })
            .await
    }

    /// Mute/unmute, role checks, online target, dual deadlines and retained command result
    /// share one transaction. No message text or private chat content is persisted.
    pub async fn mutate_help_channel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        action: HelpAction,
        target: CharacterName,
    ) -> Result<Option<HelpCommandOutcome>, DurabilityError> {
        if command_id == 0 {
            return Ok(None);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let target_key = target.comparison_key();
        let digest = help_digest(action, &target);
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !active(&mut tx, &fence, &node).await? { return Ok(None); }
            lock_command(&mut tx, &fence, command_id).await?;
            if let Some(result) = retained_result(&mut tx, &fence, command_id, &digest).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Some(HelpCommandOutcome { result, target_name: None, applied: false }));
            }
            let moderator = role(&mut tx, fence.character_id.as_bytes()).await?;
            let now = db_now(&mut tx).await?;
            let own_wait = help_mute(&mut tx, fence.character_id.as_bytes()).await?.remaining_seconds(now);
            let mut result = if own_wait > 0 {
                ChatIntentResult { disposition: ChatDisposition::Muted, wait_seconds: own_wait }
            } else if moderator.group_id < 2 { result(ChatDisposition::NotAllowed) }
            else { result(ChatDisposition::NotOnline) };
            let mut target_name = None;
            let mut applied = false;
            if own_wait == 0 && moderator.group_id >= 2 {
                let row = sqlx::query("SELECT uuid_send(r.character_id) AS character,r.name \
                    FROM game_character_roots r JOIN game_durability_admission_character_guards g USING(character_id) \
                    JOIN game_durability_reconnect_sessions s ON s.game_session_id=g.holder_game_session_id \
                    WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND r.name_key=$2 \
                    AND g.eligible AND g.world_id=r.world_id AND s.character_id=r.character_id AND s.world_id=r.world_id \
                    AND s.session_state=2 AND s.current_transport_ref IS NOT NULL AND s.control_loss_epoch IS NULL \
                    LIMIT 1 FOR SHARE OF r,g,s")
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(&target_key)
                    .fetch_optional(&mut *tx).await?;
                if let Some(row) = row {
                    let target: Vec<u8> = row.try_get("character")?;
                    target_name = Some(row.try_get("name")?);
                    let target_role = role(&mut tx, &target).await?;
                    let actor = fence.character_id.as_bytes().as_slice();
                    let lock_characters = if actor <= target.as_slice() { [actor, target.as_slice()] }
                        else { [target.as_slice(), actor] };
                    for character in lock_characters {
                        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('help:'||encode($1,'hex'),717))")
                            .bind(character).execute(&mut *tx).await?;
                    }
                    let now = db_now(&mut tx).await?;
                    let moderator_wait = help_mute(&mut tx, actor).await?.remaining_seconds(now);
                    let current = help_mute(&mut tx, &target).await?;
                    let changed = if moderator_wait > 0 {
                        result = ChatIntentResult { disposition: ChatDisposition::Muted, wait_seconds: moderator_wait };
                        None
                    } else { Some(current.apply(moderator, target_role, action, now)) };
                    match changed {
                        None => {},
                        Some(Ok(next)) => {
                            sqlx::query("INSERT INTO game_chat_help_mutes(character_id,world_id,condition_until_ms, \
                                speech_until_ms,moderator_character_id,last_game_session_id,last_command_id) \
                                VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid, \
                                encode($6,'hex')::uuid,$7::text::numeric) ON CONFLICT(character_id) DO UPDATE \
                                SET condition_until_ms=EXCLUDED.condition_until_ms,speech_until_ms=EXCLUDED.speech_until_ms, \
                                moderator_character_id=EXCLUDED.moderator_character_id,last_game_session_id=EXCLUDED.last_game_session_id, \
                                last_command_id=EXCLUDED.last_command_id")
                                .bind(&target).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                                .bind(i64::try_from(next.condition_until_ms).map_err(|_| DurabilityError::InvalidStoredState)?)
                                .bind(i64::try_from(next.speech_until_ms).map_err(|_| DurabilityError::InvalidStoredState)?)
                                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                                .bind(command_id.to_string()).execute(&mut *tx).await?;
                            result = self::result(ChatDisposition::Ok);
                            applied = true;
                        }
                        Some(Err(HelpRefusal::NotAuthorized)) => result = self::result(ChatDisposition::NotAllowed),
                        Some(Err(HelpRefusal::AlreadyMuted | HelpRefusal::NotMuted)) => result = self::result(ChatDisposition::Rejected),
                    }
                }
            }
            record_result(&mut tx, &fence, command_id, &digest, result).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(HelpCommandOutcome { result, target_name, applied }))
        })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_receipt_digest_preserves_valid_names_and_binds_invalid_requests()
    -> Result<(), Box<dyn std::error::Error>> {
        let name =
            CharacterName::parse("Aela").map_err(|error| format!("name fixture: {error:?}"))?;
        assert_eq!(
            help_request_digest(HelpAction::Mute, name.as_str()),
            help_digest(HelpAction::Mute, &name)
        );
        assert_ne!(
            help_request_digest(HelpAction::Mute, ""),
            help_digest(HelpAction::Mute, &name)
        );
        assert_ne!(
            help_request_digest(HelpAction::Mute, ""),
            help_request_digest(HelpAction::Unmute, "")
        );
        assert_ne!(
            help_request_digest(HelpAction::Mute, ""),
            help_request_digest(HelpAction::Mute, " ")
        );
        Ok(())
    }
}
