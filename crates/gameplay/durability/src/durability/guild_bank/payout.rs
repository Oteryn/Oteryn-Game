use super::*;
pub(in crate::durability) enum PayoutProgress {
    Blocked,
    Paid,
    AlreadyPaid,
}

/// Only the Guild owner can construct this genuine, current physical transaction capability.
pub(in crate::durability) async fn ensure_disband_payout(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
) -> Result<PayoutProgress> {
    context.assert_current(tx).await?;
    let installed: bool = sqlx::query_scalar(
        "SELECT to_regprocedure('game_guild_bank_release_ready(uuid,uuid)') IS NOT NULL",
    )
    .fetch_one(&mut **tx)
    .await?;
    if !installed {
        return Ok(PayoutProgress::Blocked);
    }
    let guild = *context.guild_id().as_bytes();
    let world = *context.world_id().as_bytes();
    let ready: bool = sqlx::query_scalar(
        "SELECT game_guild_bank_release_ready(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(guild.as_slice())
    .bind(world.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if !ready {
        return Ok(PayoutProgress::Blocked);
    }
    // The already locked Guild serializes every payout attempt and membership cleanup.
    if let Some(old)=sqlx::query("SELECT account_id::text,world_id::text,operation_id::text FROM game_guild_bank_operations WHERE guild_id=encode($1,'hex')::uuid AND kind='GUILD_DISBAND_PAYOUT'")
        .bind(guild.as_slice()).fetch_optional(&mut **tx).await? {
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_disband_claims WHERE guild_id=encode($1,'hex')::uuid AND account_id=$2::text::uuid AND world_id=$3::text::uuid AND payout_id=$4::text::uuid) AND EXISTS(SELECT 1 FROM game_guild_bank_balances WHERE guild_id=encode($1,'hex')::uuid AND balance=0)")
            .bind(guild.as_slice()).bind(old.try_get::<String,_>("account_id")?).bind(old.try_get::<String,_>("world_id")?).bind(old.try_get::<String,_>("operation_id")?).fetch_one(&mut **tx).await?;
        if !valid {return Err(invalid());}return Ok(PayoutProgress::AlreadyPaid);
    }
    // Current locked leader supplies immutable recipient identity, never a caller-selected Account.
    let leader=sqlx::query("SELECT m.character_id::text,r.account_id::text FROM game_guild_members m JOIN game_character_roots r USING(character_id) WHERE m.guild_id=encode($1,'hex')::uuid AND m.rank_level=1 AND r.world_id=encode($2,'hex')::uuid FOR UPDATE OF m")
        .bind(guild.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?;
    let actor = uuid_text(&leader.try_get::<String, _>("character_id")?)?;
    let account = uuid_text(&leader.try_get::<String, _>("account_id")?)?;
    let guild_row = lock_guild_balance(tx, guild).await?;
    let claim:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_disband_claims WHERE guild_id=encode($1,'hex')::uuid)")
        .bind(guild.as_slice()).fetch_one(&mut **tx).await?;
    if claim {
        return Err(invalid());
    }
    let bank = lock_account_balance(tx, account, world).await?;
    let balance = nonnegative(bank.try_get("balance")?)?;
    let amount = nonnegative(guild_row.try_get("balance")?)?;
    let room = headroom(tx, account, world).await?;
    let (credit, claim) = rules::split_payout(amount, balance, room).ok_or_else(invalid)?;
    let revision: String = sqlx::query_scalar(
        "SELECT revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
    )
    .bind(guild.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let now: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
    )
    .fetch_one(&mut **tx)
    .await?;
    let op = Operation {
        operation: derived(guild, b"payout"),
        guild,
        world,
        account,
        actor,
        kind: audit::PAYOUT,
        binding: Sha256::digest(guild).into(),
        command: None,
        revision: revision.parse().map_err(|_| invalid())?,
        amount,
        occurred_at: now,
        values: Values {
            bank_before: balance,
            bank_after: balance + credit,
            guild_before: amount,
            guild_after: 0,
            claim_before: 0,
            claim_after: claim,
        },
    };
    // Recheck same xid/recovery/assignment/Guild immediately before financial mutation.
    context.assert_current(tx).await?;
    store_success(tx, &op, &guild_row, &bank).await?;
    Ok(PayoutProgress::Paid)
}
