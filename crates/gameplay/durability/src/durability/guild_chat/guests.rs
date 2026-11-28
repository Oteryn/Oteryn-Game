use super::*;
use crate::domain::{
    CharacterId,
    guild::{GuildRank, GuildState},
};
use crate::durability::{
    DurabilityError, character_progression::uuid_text, db::commit_semantic_transaction,
};
use oteryn_protocol_oteryn::chat::guild_guest::{GuestRoom, MAX_GUEST_ROOMS};
use sqlx::{Postgres, Row, Transaction};

#[derive(Clone, Copy)]
pub enum GuildGuestEdit {
    Invite(CharacterId),
    Exclude(CharacterId),
}

impl DurabilityRoot {
    pub async fn edit_guild_chat_guest(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        edit: GuildGuestEdit,
    ) -> Result<bool, GuildError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let member =
                        match read_current_guild_chat_data(&mut tx, &recovery, &node, &fence, None)
                            .await
                        {
                            Ok(Some(member)) => member,
                            Ok(None) => return Ok(Ok(false)),
                            Err(GuildError::Unavailable(error)) => return Err(error),
                            Err(error) => return Ok(Err(error)),
                        };
                    if !matches!(member.rank, GuildRank::LEADER | GuildRank::VICE)
                        || !matches!(member.state, GuildState::Forming | GuildState::Active)
                        || !active(&mut tx, fence).await?
                    {
                        return Ok(Ok(false));
                    }
                    let edited = edit_guest(&mut tx, fence, member.guild_id, edit).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(edited))
                })
            })
            .await?
    }

    pub async fn guild_chat_guest_rooms(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        after: Option<[u8; 16]>,
    ) -> Result<Vec<GuestRoom>, GuildError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match read_current_guild_chat_data(&mut tx, &recovery, &node, &fence, None)
                        .await
                    {
                        Ok(_) => {}
                        Err(GuildError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    if !active(&mut tx, fence).await? {
                        return Ok(Err(GuildError::NotCurrent));
                    }
                    let rows = sqlx::query(
                        "SELECT g.guild_id::text,g.name FROM game_guild_chat_guests a
                     JOIN game_guilds g USING(guild_id)
                     WHERE a.game_session_id=encode($1,'hex')::uuid
                     AND g.world_id=encode($2,'hex')::uuid AND g.state IN(1,2)
                     AND ($3::bytea IS NULL OR g.guild_id>encode($3,'hex')::uuid)
                     AND NOT EXISTS (SELECT 1 FROM game_guild_members m
                                     WHERE m.character_id=encode($4,'hex')::uuid AND m.guild_id=g.guild_id)
                     ORDER BY g.guild_id LIMIT $5",
                    )
                    .bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .bind(after.as_ref().map(<[u8; 16]>::as_slice))
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(MAX_GUEST_ROOMS as i64)
                    .fetch_all(&mut *tx)
                    .await?;
                    let rooms = rows
                        .into_iter()
                        .map(|row| {
                            Ok(GuestRoom {
                                guild: uuid_text(&row.try_get::<String, _>("guild_id")?)?,
                                name: row.try_get("name")?,
                            })
                        })
                        .collect::<Result<Vec<_>, DurabilityError>>()?;
                    commit_semantic_read(tx, deadline).await?;
                    Ok(Ok(rooms))
                })
            })
            .await?
    }
}

async fn active(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT session_state=2 AND current_transport_ref IS NOT NULL
         FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await
}

async fn edit_guest(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    guild: GuildId,
    edit: GuildGuestEdit,
) -> Result<bool, sqlx::Error> {
    let (GuildGuestEdit::Invite(target) | GuildGuestEdit::Exclude(target)) = edit;
    // Skip a target already taking its own session fence then this Guild's lock.
    // Taking the session lock also serializes the insert with terminal cleanup.
    let target: Option<String> = sqlx::query_scalar(
        "SELECT s.game_session_id::text FROM game_durability_reconnect_sessions s
         WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid
         AND s.session_state IN(1,2) AND (NOT $4 OR (s.session_state=2 AND s.current_transport_ref IS NOT NULL))
         AND NOT EXISTS (SELECT 1 FROM game_guild_members m
                         WHERE m.character_id=s.character_id AND m.guild_id=encode($3,'hex')::uuid)
         LIMIT 1 FOR SHARE OF s SKIP LOCKED",
    )
    .bind(target.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(guild.as_bytes().as_slice())
    .bind(matches!(edit, GuildGuestEdit::Invite(_)))
    .fetch_optional(&mut **tx).await?;
    let Some(target) = target else {
        return Ok(false);
    };
    let sql = match edit {
        GuildGuestEdit::Invite(_) => "INSERT INTO game_guild_chat_guests(game_session_id,guild_id,granted_at_us)
            VALUES($1::uuid,encode($2,'hex')::uuid,floor(extract(epoch FROM clock_timestamp())*1000000)::bigint)
            ON CONFLICT DO NOTHING",
        GuildGuestEdit::Exclude(_) => "DELETE FROM game_guild_chat_guests
            WHERE game_session_id=$1::uuid AND guild_id=encode($2,'hex')::uuid",
    };
    sqlx::query(sql)
        .bind(target)
        .bind(guild.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(true)
}

pub(super) async fn granted(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    guild: GuildId,
    sent_at_micros: Option<u64>,
) -> Result<bool, DurabilityError> {
    let state: Option<i16> = sqlx::query_scalar(
        "SELECT state FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid
         AND world_id=encode($2,'hex')::uuid AND state IN(1,2) FOR SHARE",
    )
    .bind(guild.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if state.is_none() {
        return Ok(false);
    }
    // The Guild read lock serializes invite/exclude; the current session lock
    // serializes terminal cleanup. No write privilege on the grant row is needed.
    let grant: Option<i64> = sqlx::query_scalar(
        "SELECT granted_at_us FROM game_guild_chat_guests WHERE game_session_id=encode($1,'hex')::uuid
         AND guild_id=encode($2,'hex')::uuid",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(guild.as_bytes().as_slice())
    .fetch_optional(&mut **tx).await?;
    Ok(grant.is_some_and(|at| sent_at_micros.is_none_or(|sent| (at as u64) < sent)))
}
