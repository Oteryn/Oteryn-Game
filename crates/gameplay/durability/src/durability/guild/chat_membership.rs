use super::*;
impl DurabilityRoot {
    /// Server-selected current membership for send/login; absence is not permission to send.
    pub async fn read_guild_chat_membership(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<Option<GuildChatMembership>> {
        self.read_guild_chat_data(authority, node, fence, None)
            .await
    }

    /// Checks this recipient Character against the authenticated payload GuildId.
    /// Never resolves the sender's subsequent Guild or reroutes an old payload.
    /// A successful read is DATA only: delivery linearization remains the CHAT owner's gate.
    pub async fn read_guild_chat_recipient(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        payload_guild: GuildId,
    ) -> Result<Option<GuildChatMembership>> {
        self.read_guild_chat_data(authority, node, fence, Some(payload_guild))
            .await
    }

    pub(super) async fn read_guild_chat_data(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        payload_guild: Option<GuildId>,
    ) -> Result<Option<GuildChatMembership>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match read_current_guild_chat_data(
                        &mut tx,
                        &recovery,
                        &node,
                        &fence,
                        payload_guild,
                    )
                    .await
                    {
                        Ok(data) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(data))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildChatMembership {
    pub guild_id: GuildId,
    pub rank: GuildRank,
    pub motd: String,
    pub state: GuildState,
}
pub async fn read_current_guild_chat_data(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    payload_guild: Option<GuildId>,
) -> Result<Option<GuildChatMembership>> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let account = assert_session_fence(tx, fence, node)
        .await?
        .map_err(|_| GuildError::NotCurrent)?;
    let root = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
        .ok_or(GuildError::NotCurrent)?;
    if root.try_get::<String, _>("account_id")? != account
        || root.try_get::<i16, _>("lifecycle")? != 1
        || uuid_text(&root.try_get::<String, _>("world_id")?)?
            != *fence.runtime_scope.world_id().as_bytes()
    {
        return Err(GuildError::NotCurrent);
    }
    // Discovery takes no member lock before the Guild. Re-read after the Guild lock;
    // concurrent exclusion/movement cannot turn this discovery hint into authority.
    let discovered: Option<String> = sqlx::query_scalar(
        "SELECT guild_id::text FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(discovered) = discovered else {
        return Ok(None);
    };
    let guild_id = GuildId::from_bytes(uuid_text(&discovered)?).map_err(|_| invalid())?;
    if payload_guild.is_some_and(|payload| payload != guild_id) {
        return Ok(None);
    }
    let guild = sqlx::query("SELECT world_id::text,motd,state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or_else(invalid)?;
    if uuid_text(&guild.try_get::<String, _>("world_id")?)?
        != *fence.runtime_scope.world_id().as_bytes()
    {
        return Err(GuildError::NotCurrent);
    }
    let member = sqlx::query("SELECT guild_id::text,rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(member) = member else {
        return Ok(None);
    };
    if uuid_text(&member.try_get::<String, _>("guild_id")?)? != *guild_id.as_bytes() {
        return Err(GuildError::NotCurrent);
    }
    let rank = u8::try_from(member.try_get::<i16, _>("rank_level")?).map_err(|_| invalid())?;
    let state = match guild.try_get::<i16, _>("state")? {
        1 => GuildState::Forming,
        2 => GuildState::Active,
        3 => GuildState::Disbanding,
        4 => GuildState::Disbanded,
        _ => return Err(invalid()),
    };
    Ok(Some(GuildChatMembership {
        guild_id,
        rank: GuildRank::new(rank).map_err(|_| invalid())?,
        motd: guild.try_get("motd")?,
        state,
    }))
}
