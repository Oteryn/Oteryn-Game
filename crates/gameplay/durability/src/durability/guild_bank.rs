//! GUILD0 §5 native Guild bank. Recovery -> occurrence -> Character -> Guild/member ->
//! Guild balance/claim -> Account balance. Every value move commits a receipt and paired chains.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_item_only_fence, numeric_u64, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::guild::{GuildDisbandTransaction, GuildError};
use super::guild_bank_audit::{self as audit, Values};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_economy::guild_bank::{self as rules, TransferDirection};
use oteryn_protocol_oteryn::guild_bank::{
    self as wire, GuildBankAction, GuildBankIntent, GuildBankQuery, GuildBankReply,
};
use sha2::{Digest, Sha256};
use sqlx::{Acquire, Postgres, Row, Transaction};
type Result<T> = std::result::Result<T, GuildError>;
mod payout;
mod query;
mod storage;
pub(super) use payout::{PayoutProgress, ensure_disband_payout};
use storage::*;

#[derive(Debug, Clone)]
pub struct GuildBankRequest {
    pub operation_id: [u8; 16],
    pub command: CommandRef,
    pub occurred_at: i64,
    pub wire_digest: [u8; 32],
    pub intent: GuildBankIntent,
}
impl DurabilityRoot {
    pub async fn commit_guild_bank(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildBankRequest,
    ) -> Result<GuildBankReply> {
        let encoded = wire::encode_guild_bank_intent(&request.intent)
            .map_err(|_| GuildError::InvalidInput)?;
        if request.command.game_session_id() != fence.game_session_id
            || request.occurred_at <= 0
            || matches!(request.intent.action, GuildBankAction::Disband(_))
        {
            return Err(GuildError::InvalidInput);
        }
        super::item_mint_audit::check_uuid_v7(&request.operation_id)
            .map_err(|_| GuildError::InvalidInput)?;
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
                        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('guild-bank:'||encode($1,'hex'),0))")
                            .bind(request.operation_id.as_slice())
                            .execute(&mut *tx)
                            .await?;
                        let account = lock_actor(&mut tx, &node, &fence).await?;
                        let mut digest = Sha256::new();
                        digest.update(b"oteryn:guild-bank-binding:v1");
                        digest.update(fence.character_id.as_bytes());
                        digest.update(account);
                        digest.update(fence.runtime_scope.world_id().as_bytes());
                        digest.update(fence.game_session_id.as_bytes());
                        digest.update(request.command.command_id().get().to_be_bytes());
                        digest.update(encoded);
                        digest.update(request.wire_digest);
                        let binding: [u8; 32] = digest.finalize().into();
                        if let Some(row) = sqlx::query("SELECT request_binding,reply FROM game_guild_bank_operations WHERE operation_id=encode($1,'hex')::uuid")
                            .bind(request.operation_id.as_slice())
                            .fetch_optional(&mut *tx)
                            .await?
                        {
                            if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
                                return Err(GuildError::ConflictingOccurrence);
                            }
                            return wire::decode_guild_bank_reply(&row.try_get::<Vec<u8>, _>("reply")?)
                                .map_err(|_| invalid());
                        }
                        execute(&mut tx, &fence, &request, account, binding).await
                    }
                    .await;
                    match result {
                        Ok(reply) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(reply))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
async fn lock_actor(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
) -> Result<[u8; 16]> {
    assert_item_only_fence(tx, fence, node)
        .await?
        .map_err(|_| GuildError::NotCurrent)?;
    let account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    Ok(uuid_text(&account)?)
}
fn invalid() -> GuildError {
    GuildError::Unavailable(DurabilityError::InvalidStoredState)
}
fn refused(code: u8) -> GuildBankReply {
    GuildBankReply {
        code,
        bank_balance: 0,
        guild_balance: 0,
        claim: 0,
        entries: Vec::new(),
        next: None,
        claims: Vec::new(),
        next_claim: None,
    }
}
fn derived(id: [u8; 16], tag: &[u8]) -> [u8; 16] {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:guild-bank-id:v1");
    hash.update(tag);
    hash.update(id);
    let digest = hash.finalize();
    let mut out = [0; 16];
    out.copy_from_slice(&digest[..16]);
    out[6] = (out[6] & 15) | 0x70;
    out[8] = (out[8] & 63) | 0x80;
    out
}
async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    r: &GuildBankRequest,
    account: [u8; 16],
    binding: [u8; 32],
) -> Result<GuildBankReply> {
    let guild = r.intent.guild;
    let row=sqlx::query("SELECT world_id::text,state,revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild.as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildError::InvalidInput)?;
    if uuid_text(&row.try_get::<String, _>("world_id")?)?
        != *fence.runtime_scope.world_id().as_bytes()
    {
        return Err(GuildError::InvalidInput);
    }
    let revision = numeric_u64(&row, "revision")?;
    let (kind, amount) = match r.intent.action {
        GuildBankAction::Deposit(n) => (audit::DEPOSIT, n),
        GuildBankAction::Withdraw(n) => (audit::WITHDRAW, n),
        GuildBankAction::Claim => (audit::CLAIM, 0),
        GuildBankAction::Disband(_) => return Err(GuildError::InvalidInput),
    };
    let mut op = Operation {
        operation: r.operation_id,
        guild,
        world: *fence.runtime_scope.world_id().as_bytes(),
        account,
        actor: *fence.character_id.as_bytes(),
        kind,
        binding,
        command: Some(r.command),
        revision,
        amount,
        occurred_at: r.occurred_at,
        values: Values::default(),
    };
    // State gate before membership/balances. Claims deliberately survive DISBANDED and cleanup.
    let mut acting_rank = 0;
    if kind != audit::CLAIM {
        if !matches!(row.try_get::<i16, _>("state")?, 1 | 2) {
            return store_refusal(tx, &op, 3).await;
        }
        let rank:Option<i16>=sqlx::query_scalar("SELECT rank_level FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(guild.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some(rank) = rank else {
            return store_refusal(tx, &op, 2).await;
        };
        acting_rank = u8::try_from(rank).map_err(|_| invalid())?;
        if kind == audit::WITHDRAW && rank > 2 {
            return store_refusal(tx, &op, 2).await;
        }
    }
    if super::bank::fee_payer_is_junior(fence.character_id) {
        return store_refusal(tx, &op, 6).await;
    }
    // Lazy balance creation is part of the value plan. A refusal retains only its
    // replay receipt, never an empty balance created while checking that plan.
    let mut balances = tx.begin().await?;
    let guild_balance = lock_guild_balance(&mut balances, guild).await?;
    let claim=sqlx::query("SELECT account_id::text,world_id::text,amount FROM game_guild_disband_claims WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild.as_slice()).fetch_optional(&mut *balances).await?;
    let claim_amount = claim
        .as_ref()
        .map(|c| c.try_get::<i64, _>("amount"))
        .transpose()?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| invalid())?
        .unwrap_or(0);
    if kind == audit::CLAIM
        && (claim.as_ref().is_none_or(|c| {
            c.try_get::<String, _>("account_id")
                .ok()
                .and_then(|s| uuid_text(&s).ok())
                != Some(account)
                || c.try_get::<String, _>("world_id")
                    .ok()
                    .and_then(|s| uuid_text(&s).ok())
                    != Some(op.world)
        }) || claim_amount == 0)
    {
        balances.rollback().await?;
        return store_refusal(tx, &op, 7).await;
    }
    let bank = lock_account_balance(&mut balances, account, op.world).await?;
    let bank_before = nonnegative(bank.try_get("balance")?)?;
    let guild_before = nonnegative(guild_balance.try_get("balance")?)?;
    let headroom = headroom(&mut balances, account, op.world).await?;
    op.values = Values {
        bank_before,
        bank_after: bank_before,
        guild_before,
        guild_after: guild_before,
        claim_before: claim_amount,
        claim_after: claim_amount,
    };
    if kind == audit::CLAIM {
        let (credit, remaining) =
            rules::split_payout(claim_amount, bank_before, headroom).ok_or_else(invalid)?;
        if credit == 0 {
            balances.rollback().await?;
            return store_refusal(tx, &op, 5).await;
        }
        op.amount = credit;
        op.values.bank_after = bank_before + credit;
        op.values.claim_after = remaining;
    } else {
        // Rank/state were checked under the Guild/member locks. L3 checks the value plan too.
        let direction = if kind == audit::DEPOSIT {
            TransferDirection::Deposit
        } else {
            TransferDirection::Withdraw
        };
        let room = if kind == audit::DEPOSIT {
            rules::GUILD_BALANCE_MAX - guild_before
        } else {
            headroom
        };
        let plan = rules::plan_transfer(
            direction,
            acting_rank,
            super::bank::StarterIslandFact::NoStarterIsland,
            bank_before,
            guild_before,
            amount,
            room,
        );
        match plan {
            Ok(plan) => {
                op.values.bank_after = plan.bank_after;
                op.values.guild_after = plan.guild_after;
            }
            Err(e) => {
                balances.rollback().await?;
                return store_refusal(
                    tx,
                    &op,
                    match e {
                        rules::Refusal::NotAllowed => 2,
                        rules::Refusal::Junior => 6,
                        rules::Refusal::InsufficientBalance => 4,
                        rules::Refusal::BalanceLimit => 5,
                    },
                )
                .await;
            }
        }
    }
    let reply = store_success(&mut balances, &op, &guild_balance, &bank).await?;
    balances.commit().await?;
    Ok(reply)
}
fn nonnegative(value: i64) -> Result<u64> {
    u64::try_from(value).map_err(|_| invalid())
}
