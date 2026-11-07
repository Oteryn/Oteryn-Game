//! Bounded member, invitation and leadership cleanup in the locked Guild transaction.
use super::*;

const GUILD_DISBAND_ROWS_PER_STEP: usize = 100;

async fn assert_core_released(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
) -> Result<()> {
    sqlx::query(
        "SELECT game_guild_core_assert_released(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(context.guild.as_bytes().as_slice())
    .bind(context.world.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
pub(super) async fn advance_disband_transaction(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
) -> Result<GuildDisbandOutcome> {
    let latest: Option<i32> = sqlx::query_scalar("SELECT step FROM game_guild_disband_steps WHERE guild_id=encode($1,'hex')::uuid ORDER BY step DESC LIMIT 1")
        .bind(context.guild.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let step = latest.unwrap_or(1).checked_add(1).ok_or_else(invalid)?;
    let members: Vec<String> = sqlx::query_scalar("SELECT character_id::text FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND rank_level<>1 ORDER BY character_id LIMIT 100 FOR UPDATE")
        .bind(context.guild.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if !members.is_empty() {
        let leadership = lock_batch_leadership(tx, context, &members).await?;
        return commit_disband_step(tx, context, step, 4, &members, &[], &leadership).await;
    }
    let invitations: Vec<String> = sqlx::query_scalar("SELECT character_id::text FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid ORDER BY character_id LIMIT 100 FOR UPDATE")
        .bind(context.guild.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if !invitations.is_empty() {
        return commit_disband_step(tx, context, step, 5, &[], &invitations, &[]).await;
    }
    let leader: Vec<String> = sqlx::query_scalar("SELECT character_id::text FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid ORDER BY character_id LIMIT 2 FOR UPDATE")
        .bind(context.guild.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if leader.len() != 1 {
        return Err(invalid());
    }
    let leadership = lock_batch_leadership(tx, context, &leader).await?;
    // Rank definitions are capped at 20; the final step removes this small locked set.
    sqlx::query("SELECT level FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid ORDER BY level LIMIT 21 FOR UPDATE")
        .bind(context.guild.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    commit_disband_step(tx, context, step, 6, &leader, &[], &leadership).await
}

async fn commit_disband_step(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
    step: i32,
    phase: i16,
    members: &[String],
    invitations: &[String],
    leadership: &[String],
) -> Result<GuildDisbandOutcome> {
    context.assert_current(tx).await?;
    assert_core_released(tx, context).await?;
    if members.len() + invitations.len() > GUILD_DISBAND_ROWS_PER_STEP
        || leadership.len() > members.len()
    {
        return Err(invalid());
    }
    let revision = context.revision.checked_add(1).ok_or_else(invalid)?;
    // The owner guard reconstructs this whole payload from the exact immutable step; row IDs
    // are retained in the step, while the bounded private event contains counts and bindings.
    sqlx::query("INSERT INTO game_guild_disband_steps(guild_id,step,phase,world_id,previous_revision,revision,payout_transaction_id,member_ids,invitation_ids,leadership_accounts,occurred_at) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,encode($7,'hex')::uuid,ARRAY(SELECT x::uuid FROM unnest($8::text[]) x),ARRAY(SELECT x::uuid FROM unnest($9::text[]) x),ARRAY(SELECT x::uuid FROM unnest($10::text[]) x),floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)")
        .bind(context.guild.as_bytes().as_slice()).bind(step).bind(phase).bind(context.world.as_bytes().as_slice())
        .bind(context.revision.to_string()).bind(revision.to_string()).bind(Option::<Vec<u8>>::None)
        .bind(members).bind(invitations).bind(leadership).execute(&mut **tx).await?;
    if !members.is_empty() {
        sqlx::query("DELETE FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND character_id IN(SELECT x::uuid FROM unnest($2::text[]) x)")
            .bind(context.guild.as_bytes().as_slice()).bind(members).execute(&mut **tx).await?;
    }
    if !invitations.is_empty() {
        sqlx::query("DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id IN(SELECT x::uuid FROM unnest($2::text[]) x)")
            .bind(context.guild.as_bytes().as_slice()).bind(invitations).execute(&mut **tx).await?;
    }
    if !leadership.is_empty() {
        sqlx::query("DELETE FROM game_account_guild_leadership WHERE guild_id=encode($1,'hex')::uuid AND account_id IN(SELECT x::uuid FROM unnest($2::text[]) x)")
            .bind(context.guild.as_bytes().as_slice()).bind(leadership).execute(&mut **tx).await?;
    }
    if phase == 6 {
        sqlx::query("DELETE FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid")
            .bind(context.guild.as_bytes().as_slice())
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query("UPDATE game_guilds SET state=CASE WHEN $3 THEN 4 ELSE 3 END,revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
        .bind(context.guild.as_bytes().as_slice()).bind(revision.to_string()).bind(phase==6).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_guild_disband_step_events(guild_id,step,payload,payload_sha256) SELECT guild_id,step,convert_to(game_guild_disband_step_payload(s)::text,'UTF8'),sha256(convert_to(game_guild_disband_step_payload(s)::text,'UTF8')) FROM game_guild_disband_steps s WHERE guild_id=encode($1,'hex')::uuid AND step=$2")
        .bind(context.guild.as_bytes().as_slice()).bind(step).execute(&mut **tx).await?;
    Ok(if phase == 6 {
        GuildDisbandOutcome::Disbanded { revision }
    } else {
        GuildDisbandOutcome::Advanced {
            step,
            removed: members.len() + invitations.len(),
        }
    })
}

async fn lock_batch_leadership(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
    members: &[String],
) -> Result<Vec<String>> {
    // Guild serialization fixes membership; lock only this step's accounts in stable order.
    Ok(sqlx::query_scalar("SELECT account_id::text FROM game_account_guild_leadership WHERE guild_id=encode($1,'hex')::uuid AND character_id IN(SELECT x::uuid FROM unnest($2::text[]) x) ORDER BY account_id LIMIT 101 FOR UPDATE")
        .bind(context.guild.as_bytes().as_slice()).bind(members).fetch_all(&mut **tx).await?)
}
