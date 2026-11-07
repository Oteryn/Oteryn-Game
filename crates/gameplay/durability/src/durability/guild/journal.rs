use super::*;
pub(super) async fn lock_operation(
    tx: &mut Transaction<'_, Postgres>,
    request: &GuildRequest,
) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-command:' || encode($1,'hex') || ':' || $2,0))")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-operation:' || encode($1,'hex'),0))")
        .bind(request.identity.occurrence.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    request: &GuildRequest,
    binding: &[u8],
) -> Result<Option<GuildOutcome>> {
    if let Some(row)=sqlx::query("SELECT request_binding,result,previous_revision::text,revision::text,transaction_id::text,event_id::text FROM game_guild_operations WHERE occurrence=encode($1,'hex')::uuid")
        .bind(request.identity.occurrence.as_slice()).fetch_optional(&mut **tx).await? {
        if row.try_get::<Vec<u8>,_>("request_binding")? != binding { return Err(GuildError::ConflictingOccurrence); }
        return Ok(Some(GuildOutcome::Replayed(receipt(&row)?)));
    }
    let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_operations WHERE (game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric) OR transaction_id=encode($3,'hex')::uuid OR requested_event_id=encode($4,'hex')::uuid)")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice()).fetch_one(&mut **tx).await?;
    if used {
        return Err(GuildError::ConflictingOccurrence);
    }
    Ok(None)
}

pub(super) async fn remove_invitations(
    tx: &mut Transaction<'_, Postgres>,
    target: CharacterId,
    ids: &[[u8; 16]],
) -> Result<()> {
    for id in ids {
        sqlx::query("DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(id.as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
    }
    Ok(())
}

pub(super) async fn store(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    result: GuildResult,
    revisions: (Option<u64>, Option<u64>),
    audit: Option<&[u8]>,
) -> Result<GuildOutcome> {
    store_operation(
        tx,
        fence,
        request,
        binding,
        result,
        OperationPayload {
            revisions,
            audit,
            found_name: None,
            text: None,
            rank: None,
            resign: None,
            edit_ranks: None,
            disband: None,
        },
    )
    .await
}

pub(super) struct OperationPayload<'a> {
    pub(super) revisions: (Option<u64>, Option<u64>),
    pub(super) audit: Option<&'a [u8]>,
    pub(super) found_name: Option<&'a str>,
    pub(super) text: Option<&'a GuildTextRequest>,
    pub(super) rank: Option<(&'a GuildRankRequest, u8)>,
    pub(super) resign: Option<&'a GuildResignRequest>,
    pub(super) edit_ranks: Option<&'a GuildEditRanksRequest>,
    pub(super) disband: Option<&'a GuildDisbandRequest>,
}

pub(super) async fn store_operation(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    result: GuildResult,
    payload: OperationPayload<'_>,
) -> Result<GuildOutcome> {
    let OperationPayload {
        revisions: (previous, revision),
        audit,
        found_name,
        text,
        rank,
        resign,
        edit_ranks,
        disband,
    } = payload;
    let event = audit.map(|_| request.identity.event_id);
    sqlx::query("INSERT INTO game_guild_operations(occurrence,transaction_id,requested_event_id,game_session_id,command_id,actor,world_id,guild_id,target,action,request_binding,result,previous_revision,revision,event_id,audit_sha256,found_name,expected_revision,edit_text,previous_rank,edit_rank,edit_rank_names,disband_confirmation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13::text::numeric,$14::text::numeric,encode($15,'hex')::uuid,sha256($16),$17,$18::text::numeric,$19,$20,$21,$22,$23)")
        .bind(request.identity.occurrence.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice())
        .bind(request.operation.target(fence.character_id).as_bytes().as_slice()).bind(request.operation.code()).bind(binding).bind(result.code())
        .bind(previous.map(|x|x.to_string())).bind(revision.or(previous).map(|x|x.to_string())).bind(event.map(|x|x.to_vec())).bind(audit).bind(found_name).bind(text.map(|t| t.expected_revision).or_else(||rank.map(|(r,_)|r.expected_revision)).or_else(||resign.map(|r|r.expected_revision)).or_else(||edit_ranks.map(|r|r.expected_revision)).map(|r|r.to_string())).bind(text.map(|t| t.text.as_str())).bind(rank.map(|(_,before)|i16::from(before)).or_else(||resign.map(|_|2_i16))).bind(rank.map(|(r,_)|i16::from(r.level.level())).or_else(||resign.map(|_|1_i16))).bind(edit_ranks.map(|r|r.names.as_slice())).bind(disband.map(|r|r.confirmation.as_str()))
        .execute(&mut **tx).await?;
    if let Some(audit) = audit {
        sqlx::query("INSERT INTO game_guild_event_outbox(event_id,occurrence,guild_id,occurred_at,payload,payload_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,sha256($5))")
            .bind(request.identity.event_id.as_slice()).bind(request.identity.occurrence.as_slice()).bind(request.guild_id.as_bytes().as_slice())
            .bind(request.identity.occurred_at_unix_ms).bind(audit).execute(&mut **tx).await?;
    }
    Ok(GuildOutcome::Committed(GuildReceipt {
        result,
        previous_revision: previous,
        revision: revision.or(previous),
        transaction_id: request.identity.transaction_id,
        event_id: event,
    }))
}

pub(super) fn receipt(row: &sqlx::postgres::PgRow) -> Result<GuildReceipt> {
    let number = |name| -> Result<Option<u64>> {
        row.try_get::<Option<String>, _>(name)?
            .map(|s| s.parse().map_err(|_| invalid()))
            .transpose()
    };
    Ok(GuildReceipt {
        result: GuildResult::from_code(row.try_get("result")?).ok_or_else(invalid)?,
        previous_revision: number("previous_revision")?,
        revision: number("revision")?,
        transaction_id: uuid_text(&row.try_get::<String, _>("transaction_id")?)?,
        event_id: row
            .try_get::<Option<String>, _>("event_id")?
            .map(|s| uuid_text(&s))
            .transpose()?,
    })
}
