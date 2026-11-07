use super::*;
use sqlx::postgres::PgRow;

pub(super) struct Operation {
    pub operation: [u8; 16],
    pub guild: [u8; 16],
    pub world: [u8; 16],
    pub account: [u8; 16],
    pub actor: [u8; 16],
    pub kind: &'static str,
    pub binding: [u8; 32],
    pub command: Option<CommandRef>,
    pub revision: u64,
    pub amount: u64,
    pub occurred_at: i64,
    pub values: Values,
}
pub(super) async fn lock_guild_balance(
    tx: &mut Transaction<'_, Postgres>,
    guild: [u8; 16],
) -> Result<PgRow> {
    sqlx::query("INSERT INTO game_guild_bank_balances(guild_id,balance) VALUES(encode($1,'hex')::uuid,0) ON CONFLICT DO NOTHING")
        .bind(guild.as_slice()).execute(&mut **tx).await?;
    Ok(sqlx::query("SELECT balance,last_entry_id::text,ordinal FROM game_guild_bank_balances WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild.as_slice()).fetch_one(&mut **tx).await?)
}
pub(super) async fn lock_account_balance(
    tx: &mut Transaction<'_, Postgres>,
    account: [u8; 16],
    world: [u8; 16],
) -> Result<PgRow> {
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0) ON CONFLICT DO NOTHING")
        .bind(account.as_slice()).bind(world.as_slice()).execute(&mut **tx).await?;
    Ok(sqlx::query("SELECT balance,last_entry_id::text FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(account.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?)
}
pub(super) async fn headroom(
    tx: &mut Transaction<'_, Postgres>,
    account: [u8; 16],
    world: [u8; 16],
) -> Result<u64> {
    nonnegative(sqlx::query_scalar("SELECT game_account_bank_credit_headroom(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(account.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?)
}
pub(super) async fn store_refusal(
    tx: &mut Transaction<'_, Postgres>,
    op: &Operation,
    code: u8,
) -> Result<GuildBankReply> {
    let reply = refused(code);
    insert_operation(tx, op, Values::default(), &reply).await?;
    Ok(reply)
}
async fn insert_operation(
    tx: &mut Transaction<'_, Postgres>,
    op: &Operation,
    v: Values,
    reply: &GuildBankReply,
) -> Result<()> {
    let bytes = wire::encode_guild_bank_reply(reply).map_err(|_| invalid())?;
    sqlx::query("INSERT INTO game_guild_bank_operations(operation_id,guild_id,world_id,account_id,actor,kind,request_binding,game_session_id,command_id,guild_revision,amount,result,bank_before,bank_after,guild_before,guild_after,claim_before,claim_after,reply,occurred_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)")
        .bind(op.operation.as_slice()).bind(op.guild.as_slice()).bind(op.world.as_slice()).bind(op.account.as_slice()).bind(op.actor.as_slice())
        .bind(op.kind).bind(op.binding.as_slice()).bind(op.command.map(|c|c.game_session_id().as_bytes().to_vec()))
        .bind(op.command.map(|c|c.command_id().get().to_string())).bind(op.revision.to_string()).bind(to_i64(op.amount)?)
        .bind(i16::from(reply.code)).bind(to_i64(v.bank_before)?).bind(to_i64(v.bank_after)?).bind(to_i64(v.guild_before)?).bind(to_i64(v.guild_after)?)
        .bind(to_i64(v.claim_before)?).bind(to_i64(v.claim_after)?).bind(bytes).bind(op.occurred_at).execute(&mut **tx).await?;
    Ok(())
}
pub(super) async fn store_success(
    tx: &mut Transaction<'_, Postgres>,
    op: &Operation,
    guild: &PgRow,
    bank: &PgRow,
) -> Result<GuildBankReply> {
    let v = op.values;
    if !v.conserved(op.kind, op.amount) {
        return Err(invalid());
    }
    let reply = GuildBankReply {
        code: 1,
        bank_balance: v.bank_after,
        guild_balance: v.guild_after,
        claim: v.claim_after,
        entries: Vec::new(),
        next: None,
        claims: Vec::new(),
        next_claim: None,
    };
    insert_operation(tx, op, v, &reply).await?;
    if v.guild_before != v.guild_after {
        let entry = derived(op.operation, b"guild-entry");
        let ordinal = guild
            .try_get::<i64, _>("ordinal")?
            .checked_add(1)
            .ok_or_else(invalid)?;
        sqlx::query("INSERT INTO game_guild_bank_entries(entry_id,operation_id,guild_id,ordinal,kind,amount,balance_before,balance_after,previous_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7,$8,$9::text::uuid)")
            .bind(entry.as_slice()).bind(op.operation.as_slice()).bind(op.guild.as_slice()).bind(ordinal).bind(op.kind).bind(to_i64(op.amount)?)
            .bind(to_i64(v.guild_before)?).bind(to_i64(v.guild_after)?).bind(guild.try_get::<Option<String>,_>("last_entry_id")?).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_guild_bank_balances SET balance=$2,last_entry_id=encode($3,'hex')::uuid,ordinal=$4 WHERE guild_id=encode($1,'hex')::uuid")
            .bind(op.guild.as_slice()).bind(to_i64(v.guild_after)?).bind(entry.as_slice()).bind(ordinal).execute(&mut **tx).await?;
    }
    if v.bank_before != v.bank_after {
        let entry = derived(op.operation, b"bank-entry");
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,guild_operation_id,guild_kind,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,NULL,$6,$7,$8,$9::text::uuid,encode($10,'hex')::uuid)")
            .bind(entry.as_slice()).bind(op.operation.as_slice()).bind(op.kind).bind(op.account.as_slice()).bind(op.world.as_slice())
            .bind(to_i64(v.bank_before.abs_diff(v.bank_after))?).bind(to_i64(v.bank_before)?).bind(to_i64(v.bank_after)?)
            .bind(bank.try_get::<Option<String>,_>("last_entry_id")?).bind(op.actor.as_slice()).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_account_bank_balances SET balance=$3,last_entry_id=encode($4,'hex')::uuid WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
            .bind(op.account.as_slice()).bind(op.world.as_slice()).bind(to_i64(v.bank_after)?).bind(entry.as_slice()).execute(&mut **tx).await?;
    }
    if op.kind == audit::PAYOUT {
        sqlx::query("INSERT INTO game_guild_disband_claims(guild_id,account_id,world_id,amount,payout_id,last_operation_id,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,encode($5,'hex')::uuid,$6)")
            .bind(op.guild.as_slice()).bind(op.account.as_slice()).bind(op.world.as_slice()).bind(to_i64(v.claim_after)?).bind(op.operation.as_slice()).bind(op.occurred_at).execute(&mut **tx).await?;
    } else if op.kind == audit::CLAIM {
        sqlx::query("UPDATE game_guild_disband_claims SET amount=$2,last_operation_id=encode($3,'hex')::uuid WHERE guild_id=encode($1,'hex')::uuid")
            .bind(op.guild.as_slice()).bind(to_i64(v.claim_after)?).bind(op.operation.as_slice()).execute(&mut **tx).await?;
    }
    // SQL reconstructs the exact closed native proposal from immutable operation columns.
    // Two owner records for transfers/payout, BANK only for custody delivery; all bounded by PK.
    let owners: &[&str] = if op.kind == audit::CLAIM {
        &["BANK"]
    } else {
        &["BANK", "GUILD"]
    };
    for owner in owners {
        sqlx::query("INSERT INTO game_guild_bank_event_outbox(operation_id,owner,payload,payload_sha256,occurred_at,expires_at) SELECT operation_id,$2,convert_to(game_guild_bank_payload(o,$2)::text,'UTF8'),sha256(convert_to(game_guild_bank_payload(o,$2)::text,'UTF8')),occurred_at,occurred_at+2592000000 FROM game_guild_bank_operations o WHERE operation_id=encode($1,'hex')::uuid")
            .bind(op.operation.as_slice()).bind(*owner).execute(&mut **tx).await?;
    }
    Ok(reply)
}
fn to_i64(n: u64) -> Result<i64> {
    i64::try_from(n).map_err(|_| invalid())
}
