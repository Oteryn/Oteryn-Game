use super::*;
use crate::durability::world_event::bank_robbery::bank_suspended;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub struct GuildBankTransfer {
    pub recipient: String,
    pub amount: u64,
    pub npc: EntityRef,
    pub npc_key: String,
    pub occurred_at: i64,
}

pub enum GuildBankTransferResult {
    Completed { npc: EntityRef, code: u8 },
    Suspended,
}

impl DurabilityRoot {
    /// The NPC route has already checked the current conversation and kept the
    /// Channel owner's lock. Reconciliation passes no new transfer intent.
    pub async fn transfer_guild_bank(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        transfer: Option<GuildBankTransfer>,
    ) -> Result<Option<GuildBankTransferResult>> {
        if command.game_session_id() != fence.game_session_id {
            return Err(GuildError::NotCurrent);
        }
        let recovery = authority.record_for(self).map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                let result = async {
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let account = lock_actor(&mut tx, &node, &fence).await?;
                    let outcome = lookup(&mut tx, &fence, command, transfer.as_ref()).await?;
                    if outcome.is_some() {
                        return Ok(outcome);
                    }
                    match transfer {
                        Some(transfer) => apply(&mut tx, &fence, account, command, &transfer).await.map(Some),
                        None => Ok(None),
                    }
                }.await;
                match result {
                    Ok(outcome) => {
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(Ok(outcome))
                    }
                    Err(GuildError::Unavailable(error)) => Err(error),
                    Err(error) => Ok(Err(error)),
                }
            }))
            .await?
    }
}

fn binding(transfer: &GuildBankTransfer) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:guild-bank-peer-transfer:v1");
    digest.update(transfer.amount.to_be_bytes());
    digest.update(transfer.npc.identity);
    digest.update(transfer.npc.generation.to_be_bytes());
    digest.update(transfer.recipient.as_bytes());
    digest.finalize().into()
}

async fn lookup(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    transfer: Option<&GuildBankTransfer>,
) -> Result<Option<GuildBankTransferResult>> {
    let row = sqlx::query(
        "SELECT result,request_binding,transfer_npc_identity,transfer_npc_generation::text AS generation
         FROM game_guild_bank_operations WHERE game_session_id=encode($1,'hex')::uuid
         AND command_id=$2::text::numeric AND actor=encode($3,'hex')::uuid AND kind='GUILD_TRANSFER'",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else { return Ok(None); };
    if let Some(transfer) = transfer
        && row.try_get::<Vec<u8>, _>("request_binding")? != binding(transfer)
    {
        return Err(GuildError::ConflictingOccurrence);
    }
    Ok(Some(GuildBankTransferResult::Completed {
        npc: EntityRef {
            identity: row.try_get::<Vec<u8>, _>("transfer_npc_identity")?.try_into().map_err(|_| invalid())?,
            generation: numeric_u64(&row, "generation")?,
        },
        code: u8::try_from(row.try_get::<i16, _>("result")?).map_err(|_| invalid())?,
    }))
}

async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    account: [u8; 16],
    command: CommandRef,
    transfer: &GuildBankTransfer,
) -> Result<GuildBankTransferResult> {
    if !(1..=rules::GUILD_BALANCE_MAX).contains(&transfer.amount)
        || transfer.recipient.is_empty()
        || transfer.recipient.chars().count() > 29
        || transfer.recipient.contains('\0')
        || transfer.npc.generation == 0
        || transfer.occurred_at <= 0
    {
        return Err(GuildError::InvalidInput);
    }
    let world = fence.runtime_scope.world_id();
    if bank_suspended(tx, world, Some(&transfer.npc_key)).await? {
        return Ok(GuildBankTransferResult::Suspended);
    }
    let finish = |code| GuildBankTransferResult::Completed { npc: transfer.npc, code };
    let guilds = sqlx::query(
        "SELECT g.guild_id::text, g.state,g.revision::text, g.guild_id=m.guild_id AS source
         FROM game_guild_members m JOIN game_guilds g ON g.world_id=encode($1,'hex')::uuid
          AND (g.guild_id=m.guild_id OR (g.state IN (1,2) AND g.name_key=game_character_name_key($3)))
         WHERE m.character_id=encode($2,'hex')::uuid ORDER BY g.guild_id FOR UPDATE OF g",
    )
    .bind(world.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&transfer.recipient)
    .fetch_all(&mut **tx)
    .await?;
    let mut source = None;
    let mut target = None;
    for row in guilds {
        if row.try_get("source")? {
            source = Some(row);
        } else {
            target = Some(uuid_text(&row.try_get::<String, _>("guild_id")?)?);
        }
    }
    let Some(source) = source else { return Ok(finish(2)); };
    let guild = uuid_text(&source.try_get::<String, _>("guild_id")?)?;
    let rank: Option<i16> = sqlx::query_scalar(
        "SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid
         AND guild_id=encode($2,'hex')::uuid FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(guild.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let id: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx).await?;
    let op = Operation {
        operation: uuid_text(&id)?,
        guild,
        world: *world.as_bytes(),
        account,
        actor: *fence.character_id.as_bytes(),
        kind: audit::TRANSFER,
        binding: binding(transfer),
        command: Some(command),
        revision: numeric_u64(&source, "revision")?,
        amount: transfer.amount,
        occurred_at: transfer.occurred_at,
        values: Values::default(),
    };
    let refusal = PeerValues { target, ..PeerValues::default() };
    let code = match (source.try_get::<i16, _>("state")?, rank, target) {
        (1 | 2, Some(1 | 2), Some(target)) => {
            if super::super::bank::fee_payer_is_junior(fence.character_id) {
                6
            } else {
                move_value(tx, &op, target, transfer.npc).await?
            }
        }
        (1 | 2, _, _) => 2,
        _ => 3,
    };
    if code != 1 {
        record(tx, &op, refusal, transfer.npc, &refused(code)).await?;
    }
    Ok(finish(code))
}

#[derive(Clone, Copy, Default)]
struct PeerValues {
    target: Option<[u8; 16]>,
    source_before: u64,
    source_after: u64,
    target_before: u64,
    target_after: u64,
}

async fn move_value(
    tx: &mut Transaction<'_, Postgres>,
    op: &Operation,
    target: [u8; 16],
    npc: EntityRef,
) -> Result<u8> {
    // Both Guild rows are locked in UUID order above. Disband and Hall settlement
    // use those same rows before writing balances or changes that reserve credit.
    let mut balances = tx.begin().await?;
    let source_row = lock_guild_balance(&mut balances, op.guild).await?;
    let source_before = nonnegative(source_row.try_get("balance")?)?;
    let Some(source_after) = source_before.checked_sub(op.amount) else {
        balances.rollback().await?;
        return Ok(4);
    };
    let target_row = lock_guild_balance(&mut balances, target).await?;
    let target_before = nonnegative(target_row.try_get("balance")?)?;
    let target_after = target_before + op.amount;
    let reserved: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(escrow_guild_gold),0)::bigint FROM game_house_bids
         WHERE guild_id=encode($1,'hex')::uuid AND state=1",
    )
    .bind(target.as_slice()).fetch_one(&mut *balances).await?;
    if target_after + nonnegative(reserved)? > rules::GUILD_BALANCE_MAX {
        balances.rollback().await?;
        return Ok(5);
    }
    let values = PeerValues { target: Some(target), source_before, source_after, target_before, target_after };
    record(&mut balances, op, values, npc, &GuildBankReply {
        guild_balance: source_after,
        ..refused(1)
    }).await?;
    append_guild_entry(&mut balances, op, op.guild, &source_row, [source_before, source_after], b"guild-entry").await?;
    append_guild_entry(&mut balances, op, target, &target_row, [target_before, target_after], b"recipient-guild-entry").await?;
    store_audits(&mut balances, op).await?;
    balances.commit().await?;
    Ok(1)
}

async fn record(
    tx: &mut Transaction<'_, Postgres>,
    op: &Operation,
    values: PeerValues,
    npc: EntityRef,
    reply: &GuildBankReply,
) -> Result<()> {
    let command = op.command.ok_or_else(invalid)?;
    let bytes = wire::encode_guild_bank_reply(reply).map_err(|_| invalid())?;
    sqlx::query(
        "INSERT INTO game_guild_bank_operations
         (operation_id,guild_id,world_id,account_id,actor,kind,request_binding,game_session_id,
          command_id,guild_revision,amount,result,bank_before,bank_after,guild_before,guild_after,claim_before,claim_after,
          reply,occurred_at,transfer_target,transfer_target_before,transfer_target_after,transfer_npc_identity,transfer_npc_generation)
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
          encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,encode($8,'hex')::uuid,
          $9::text::numeric,$10::text::numeric,$11,$12,0,0,$13,$14,0,0,$15,$16,encode($17,'hex')::uuid,$18,$19,$20,$21::text::numeric)",
    )
    .bind(op.operation.as_slice()).bind(op.guild.as_slice()).bind(op.world.as_slice())
    .bind(op.account.as_slice()).bind(op.actor.as_slice()).bind(op.kind).bind(op.binding.as_slice())
    .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
    .bind(op.revision.to_string()).bind(to_i64(op.amount)?).bind(i16::from(reply.code))
    .bind(to_i64(values.source_before)?).bind(to_i64(values.source_after)?)
    .bind(bytes).bind(op.occurred_at).bind(values.target.as_ref().map(|id| id.as_slice()))
    .bind(to_i64(values.target_before)?).bind(to_i64(values.target_after)?)
    .bind(npc.identity.as_slice()).bind(npc.generation.to_string()).execute(&mut **tx).await?;
    Ok(())
}
