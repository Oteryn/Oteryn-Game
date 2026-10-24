use super::*;
impl DurabilityRoot {
    pub async fn query_guild_bank(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        query: GuildBankQuery,
    ) -> Result<GuildBankReply> {
        wire::encode_guild_bank_query(&query).map_err(|_| GuildError::InvalidInput)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        assert_recovery_fence(&mut tx, &recovery).await?;
                        lock_admission_relations(&mut tx).await?;
                        let account = lock_actor(&mut tx, &node, &fence).await?;
                        answer(&mut tx, &fence, account, query).await
                    }
                    .await;
                    match result {
                        Ok(reply) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(reply))
                        }
                        Err(GuildError::Unavailable(e)) => Err(e),
                        Err(e) => Ok(Err(e)),
                    }
                })
            })
            .await?
    }
}
async fn answer(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    account: [u8; 16],
    query: GuildBankQuery,
) -> Result<GuildBankReply> {
    let Some(guild) = query.guild else {
        return claims(tx, fence, account, query.after_claim).await;
    };
    let world = *fence.runtime_scope.world_id().as_bytes();
    let exists = sqlx::query(
        "SELECT world_id::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR SHARE",
    )
    .bind(guild.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if exists.as_ref().is_none_or(|row| {
        row.try_get::<String, _>("world_id")
            .ok()
            .and_then(|s| uuid_text(&s).ok())
            != Some(world)
    }) {
        return Ok(refused(2));
    }
    let member:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid)")
        .bind(guild.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let claim:Option<i64>=sqlx::query_scalar("SELECT amount FROM game_guild_disband_claims WHERE guild_id=encode($1,'hex')::uuid AND account_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid")
        .bind(guild.as_slice()).bind(account.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?;
    if !member && claim.is_none() {
        return Ok(refused(2));
    }
    let bank:Option<i64>=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(account.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?;
    let balance: Option<i64> = sqlx::query_scalar(
        "SELECT balance FROM game_guild_bank_balances WHERE guild_id=encode($1,'hex')::uuid",
    )
    .bind(guild.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let mut reply = GuildBankReply {
        code: 1,
        bank_balance: nonnegative(bank.unwrap_or(0))?,
        guild_balance: if member {
            nonnegative(balance.unwrap_or(0))?
        } else {
            0
        },
        claim: nonnegative(claim.unwrap_or(0))?,
        entries: Vec::new(),
        next: None,
        claims: Vec::new(),
        next_claim: None,
    };
    if member {
        let rows=sqlx::query("SELECT e.ordinal,e.kind,e.amount,e.balance_before,e.balance_after,o.actor::text FROM game_guild_bank_entries e JOIN game_guild_bank_operations o USING(operation_id) WHERE e.guild_id=encode($1,'hex')::uuid AND ($2::bigint IS NULL OR e.ordinal<$2) AND o.occurred_at>=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint-2592000000 ORDER BY e.ordinal DESC LIMIT 101")
            .bind(guild.as_slice()).bind(query.before.map(i64::try_from).transpose().map_err(|_|GuildError::InvalidInput)?).fetch_all(&mut **tx).await?;
        let more = rows.len() > wire::MAX_LEDGER_ENTRIES;
        for row in rows.into_iter().take(wire::MAX_LEDGER_ENTRIES) {
            reply.entries.push(wire::GuildBankEntry {
                ordinal: nonnegative(row.try_get("ordinal")?)?,
                kind: match row.try_get::<String, _>("kind")?.as_str() {
                    audit::DEPOSIT => 1,
                    audit::WITHDRAW => 2,
                    audit::PAYOUT => 3,
                    _ => return Err(invalid()),
                },
                amount: nonnegative(row.try_get("amount")?)?,
                before: nonnegative(row.try_get("balance_before")?)?,
                after: nonnegative(row.try_get("balance_after")?)?,
                actor: uuid_text(&row.try_get::<String, _>("actor")?)?,
            });
        }
        if more {
            reply.next = reply.entries.last().map(|e| e.ordinal);
        }
    }
    Ok(reply)
}

async fn claims(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    account: [u8; 16],
    after: Option<[u8; 16]>,
) -> Result<GuildBankReply> {
    let rows = sqlx::query("SELECT guild_id::text,amount FROM game_guild_disband_claims WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND amount>0 AND ($3::bytea IS NULL OR guild_id>encode($3,'hex')::uuid) ORDER BY guild_id LIMIT 101")
        .bind(account.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(after.map(|id| id.to_vec())).fetch_all(&mut **tx).await?;
    let more = rows.len() > wire::MAX_LEDGER_ENTRIES;
    let mut reply = refused(1);
    for row in rows.into_iter().take(wire::MAX_LEDGER_ENTRIES) {
        reply.claims.push(wire::GuildBankClaim {
            guild: uuid_text(&row.try_get::<String, _>("guild_id")?)?,
            amount: nonnegative(row.try_get("amount")?)?,
        });
    }
    if more {
        reply.next_claim = reply.claims.last().map(|c| c.guild);
    }
    Ok(reply)
}
