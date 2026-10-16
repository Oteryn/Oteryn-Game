//! GUILD0 §5 native transaction proposal, unregistered and UNINSTALLED.
//!
//! The private DDL is a slot proposal, not a migration or an assertion that GUILD1
//! tables/current writers exist. Ordinary transfers use native recovery/session/root
//! fences. Disband payout is an in-transaction participant: GUILD1 must invoke it in
//! its disband transaction only after the real property/bid release step. No method
//! here creates a disband phase, releases a property, invents a current leader, or
//! marks a guild DISBANDED. Guildhall writers must share these held-escrow locks and
//! deferred headroom proofs. Candidate audit bytes require normative CP admission.
use super::bank::fee_payer_is_junior;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_item_only_fence, numeric_u64, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::guild_bank_audit::{BANK_MAX, GuildBankAuditProposal, HARD_CEILING};
use super::item_mint_audit::check_uuid_v7;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{RuntimeScopeRefV1, WorldId};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, GuildBankError>;

#[derive(Debug)]
pub enum GuildBankError {
    InvalidInput,
    NotCurrent,
    GuildDisbanding,
    NotMember,
    NotAuthorized,
    JuniorAccount,
    InsufficientBalance,
    BalanceLimit,
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for GuildBankError {
    fn from(e: DurabilityError) -> Self {
        Self::Unavailable(e)
    }
}
impl From<sqlx::Error> for GuildBankError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unavailable(e.into())
    }
}
impl std::fmt::Display for GuildBankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "guild bank: {self:?}")
    }
}
impl std::error::Error for GuildBankError {}

/// Local identity validation only; not guild authority or a replacement native ID registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuildBankGuildId([u8; 16]);
impl GuildBankGuildId {
    pub fn from_bytes(id: [u8; 16]) -> Result<Self> {
        check_uuid_v7(&id).map_err(|_| GuildBankError::InvalidInput)?;
        Ok(Self(id))
    }
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildBankCommand {
    Deposit,
    Withdraw,
    Claim,
}
impl GuildBankCommand {
    fn cause(self) -> &'static str {
        match self {
            Self::Deposit => "GUILD_DEPOSIT",
            Self::Withdraw => "GUILD_WITHDRAW",
            Self::Claim => "GUILD_DISBAND_CLAIM",
        }
    }
}
/// Identities fixed before the first attempt. No identity is generated inside a retry.
#[derive(Clone, Debug)]
pub struct GuildBankIdentity {
    pub occurrence: [u8; 16],
    pub transaction_id: [u8; 16],
    pub guild_event_id: [u8; 16],
    pub bank_event_id: [u8; 16],
    pub guild_entry_id: [u8; 16],
    pub bank_entry_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
}
impl GuildBankIdentity {
    fn validate(&self) -> Result<()> {
        let ids = [
            self.occurrence,
            self.transaction_id,
            self.guild_event_id,
            self.bank_event_id,
            self.guild_entry_id,
            self.bank_entry_id,
        ];
        if self.occurred_at_unix_ms <= 0
            || ids
                .iter()
                .enumerate()
                .any(|(n, id)| check_uuid_v7(id).is_err() || ids[..n].contains(id))
        {
            return Err(GuildBankError::InvalidInput);
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct GuildBankRequest {
    pub identity: GuildBankIdentity,
    pub guild_id: GuildBankGuildId,
    pub command: GuildBankCommand,
    /// Exact confirmed amount for Deposit/Withdraw; Claim must use 0 (means current whole claim,
    /// bounded by actual locked headroom, never an independently supplied claimed balance).
    pub amount: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildBankReceipt {
    pub transaction_id: [u8; 16],
    pub credited: u64,
    pub claim_remaining: u64,
    pub replayed: bool,
}

impl DurabilityRoot {
    pub async fn guild_bank_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildBankRequest,
    ) -> Result<GuildBankReceipt> {
        request.identity.validate()?;
        if (request.command == GuildBankCommand::Claim && request.amount != 0)
            || (request.command != GuildBankCommand::Claim
                && !(1..=BANK_MAX).contains(&request.amount))
        {
            return Err(GuildBankError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildBankError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match command_in_transaction(&mut tx, &node, fence, &request).await {
                        Ok(receipt) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        Err(GuildBankError::Unavailable(e)) => Err(e),
                        Err(e) => Ok(Err(e)),
                    }
                })
            })
            .await?
    }
}

fn binding(
    cause: &str,
    guild: &[u8; 16],
    world: &[u8; 16],
    actor: &[u8; 16],
    amount: u64,
) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(b"GUILD_BANK_REQUEST_V1\0");
    h.update(cause.as_bytes());
    h.update(guild);
    h.update(world);
    h.update(actor);
    h.update(amount.to_be_bytes());
    h.finalize().to_vec()
}
async fn occurrence_lock(tx: &mut Transaction<'_, Postgres>, id: &[u8; 16]) -> Result<()> {
    let key = format!("oteryn:guild-bank:{}", hex(id));
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    identity: &GuildBankIdentity,
    binding: &[u8],
) -> Result<Option<GuildBankReceipt>> {
    let row=sqlx::query("SELECT transaction_id::text,request_binding,credited,claim_remaining FROM game_guild_bank_operations WHERE occurrence_id=encode($1,'hex')::uuid")
        .bind(identity.occurrence.as_slice()).fetch_optional(&mut **tx).await?;
    if let Some(row) = row {
        if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
            return Err(GuildBankError::ConflictingOccurrence);
        }
        return Ok(Some(GuildBankReceipt {
            transaction_id: uuid_text(&row.try_get::<String, _>("transaction_id")?)?,
            credited: u64::try_from(row.try_get::<i64, _>("credited")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            claim_remaining: u64::try_from(row.try_get::<i64, _>("claim_remaining")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            replayed: true,
        }));
    }
    Ok(None)
}
async fn command_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: &GuildBankRequest,
) -> Result<GuildBankReceipt> {
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return Err(GuildBankError::NotCurrent);
    };
    let b = binding(
        request.command.cause(),
        request.guild_id.as_bytes(),
        world_id.as_bytes(),
        fence.character_id.as_bytes(),
        request.amount,
    );
    occurrence_lock(tx, &request.identity.occurrence).await?;
    // Replay is retained outcome data, never current authorization or another value grant.
    if let Some(receipt) = replay(tx, &request.identity, &b).await? {
        return Ok(receipt);
    }
    if assert_item_only_fence(tx, &fence, node).await?.is_err() {
        return Err(GuildBankError::NotCurrent);
    }
    if fee_payer_is_junior(fence.character_id) {
        return Err(GuildBankError::JuniorAccount);
    }
    let account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let account = uuid_text(&account)?;
    if request.command != GuildBankCommand::Claim {
        let guild=sqlx::query("SELECT state,world_id::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(request.guild_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildBankError::NotMember)?;
        if guild.try_get::<String, _>("world_id")? != canonical(world_id.as_bytes()) {
            return Err(GuildBankError::NotMember);
        }
        if ![1_i16, 2].contains(&guild.try_get::<i16, _>("state")?) {
            return Err(GuildBankError::GuildDisbanding);
        }
        let rank:Option<i16>=sqlx::query_scalar("SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid AND guild_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let rank = rank.ok_or(GuildBankError::NotMember)?;
        if request.command == GuildBankCommand::Withdraw && rank > 2 {
            return Err(GuildBankError::NotAuthorized);
        }
    }
    let mut state = lock_state(
        tx,
        request.guild_id,
        world_id,
        account,
        request.command == GuildBankCommand::Claim,
    )
    .await?;
    let before = state;
    let amount = match request.command {
        GuildBankCommand::Deposit => {
            if state.account < request.amount {
                return Err(GuildBankError::InsufficientBalance);
            }
            if state
                .guild
                .checked_add(request.amount)
                .and_then(|n| n.checked_add(state.guild_held))
                .is_none_or(|n| n > BANK_MAX)
            {
                return Err(GuildBankError::BalanceLimit);
            }
            state.account -= request.amount;
            state.guild += request.amount;
            request.amount
        }
        GuildBankCommand::Withdraw => {
            if state.guild < request.amount {
                return Err(GuildBankError::InsufficientBalance);
            }
            if state
                .account
                .checked_add(request.amount)
                .is_none_or(|n| n > BANK_MAX)
                || request.amount > headroom(state.account, state.account_held)?
            {
                return Err(GuildBankError::BalanceLimit);
            }
            state.guild -= request.amount;
            state.account += request.amount;
            request.amount
        }
        GuildBankCommand::Claim => {
            let headroom = headroom(state.account, state.account_held)?;
            let amount = state.claim.min(headroom);
            if amount == 0 {
                return Err(GuildBankError::BalanceLimit);
            }
            state.claim -= amount;
            state.account += amount;
            amount
        }
    };
    persist(
        tx,
        &request.identity,
        request.guild_id,
        world_id,
        account,
        Some(*fence.character_id.as_bytes()),
        request.command.cause(),
        b,
        before,
        state,
        amount,
    )
    .await
}
fn canonical(id: &[u8; 16]) -> String {
    let h = hex(id);
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
#[derive(Clone, Copy)]
struct State {
    guild: u64,
    account: u64,
    claim: u64,
    guild_held: u64,
    account_held: u64,
    guild_last: Option<[u8; 16]>,
    account_last: Option<[u8; 16]>,
    claim_last: Option<[u8; 16]>,
}
fn value(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64> {
    u64::try_from(row.try_get::<i64, _>(column)?)
        .map_err(|_| DurabilityError::InvalidStoredState.into())
}
fn last(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<[u8; 16]>> {
    row.try_get::<Option<String>, _>(column)?
        .map(|s| uuid_text(&s).map_err(Into::into))
        .transpose()
}
fn headroom(balance: u64, held: u64) -> Result<u64> {
    HARD_CEILING
        .checked_sub(balance)
        .and_then(|n| n.checked_sub(held))
        .ok_or(GuildBankError::BalanceLimit)
}
async fn lock_state(
    tx: &mut Transaction<'_, Postgres>,
    guild: GuildBankGuildId,
    world: WorldId,
    account: [u8; 16],
    claim_only: bool,
) -> Result<State> {
    let mut s = State {
        guild: 0,
        account: 0,
        claim: 0,
        guild_held: 0,
        account_held: 0,
        guild_last: None,
        account_last: None,
        claim_last: None,
    };
    if !claim_only {
        sqlx::query("INSERT INTO game_guild_bank_balances(guild_id,balance) VALUES(encode($1,'hex')::uuid,0) ON CONFLICT DO NOTHING").bind(guild.as_bytes().as_slice()).execute(&mut **tx).await?;
        let row=sqlx::query("SELECT balance,last_entry_id::text FROM game_guild_bank_balances WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE").bind(guild.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        s.guild = value(&row, "balance")?;
        s.guild_last = last(&row, "last_entry_id")?;
        s.guild_held = held(tx, "guild", guild.as_bytes(), world.as_bytes()).await?;
    }
    let claim=sqlx::query("SELECT amount,last_entry_id::text,exleader_account_id::text,world_id::text FROM game_guild_disband_claims WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if let Some(row) = claim {
        if row.try_get::<String, _>("exleader_account_id")? != canonical(&account)
            || row.try_get::<String, _>("world_id")? != canonical(world.as_bytes())
        {
            if claim_only {
                return Err(GuildBankError::NotAuthorized);
            }
        } else {
            s.claim = value(&row, "amount")?;
            s.claim_last = last(&row, "last_entry_id")?;
        }
    } else if claim_only {
        return Err(GuildBankError::NotAuthorized);
    }
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0) ON CONFLICT DO NOTHING")
        .bind(account.as_slice()).bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
    let row=sqlx::query("SELECT balance,last_entry_id::text FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(account.as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    s.account = value(&row, "balance")?;
    s.account_last = last(&row, "last_entry_id")?;
    s.account_held = held(tx, "account", &account, world.as_bytes()).await?;
    Ok(s)
}
async fn held(
    tx: &mut Transaction<'_, Postgres>,
    source: &str,
    id: &[u8; 16],
    world: &[u8; 16],
) -> Result<u64> {
    // Candidate coordinated custody source; never a caller-supplied escrow number/boolean.
    let row=sqlx::query("SELECT coalesce(sum(CASE WHEN $1='guild' THEN escrow_guild_gold ELSE escrow_account_gold END),0)::text AS amount FROM game_guildhall_bids WHERE (($1='guild' AND guild_id=encode($2,'hex')::uuid) OR ($1='account' AND funding_account_id=encode($2,'hex')::uuid)) AND world_id=encode($3,'hex')::uuid AND state='HELD'")
        .bind(source).bind(id.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?;
    Ok(numeric_u64(&row, "amount")?)
}

/// GUILD1 disband-transaction participant, not an independently callable payout job.
/// Caller must already hold recovery/admission, occurrence then the actual guild/member locks
/// in GUILD0-LO-01 order; no Character root/session is acquired by this World-job path.
/// The candidate verifies actual DISBANDING state, same World and the current leader Account.
/// Actual property/bid release and transition orchestration remain mandatory GUILD1 CP seams.
pub(super) async fn payout_in_disband_transaction(
    tx: &mut Transaction<'_, Postgres>,
    identity: &GuildBankIdentity,
    guild: GuildBankGuildId,
    world: WorldId,
) -> Result<GuildBankReceipt> {
    identity.validate()?;
    occurrence_lock(tx, &identity.occurrence).await?;
    let g=sqlx::query("SELECT state,world_id::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildBankError::NotCurrent)?;
    if g.try_get::<String, _>("world_id")? != canonical(world.as_bytes()) {
        return Err(GuildBankError::NotCurrent);
    }
    // The retained payout is readable after member cleanup and DISBANDED. It grants nothing.
    if let Some(row)=sqlx::query("SELECT transaction_id::text,credited,claim_remaining FROM game_guild_bank_operations WHERE guild_id=encode($1,'hex')::uuid AND cause='GUILD_DISBAND_PAYOUT'")
        .bind(guild.as_bytes().as_slice()).fetch_optional(&mut **tx).await? {
        return Ok(GuildBankReceipt{transaction_id:uuid_text(&row.try_get::<String,_>("transaction_id")?)?,credited:value(&row,"credited")?,claim_remaining:value(&row,"claim_remaining")?,replayed:true})
    }
    if g.try_get::<i16, _>("state")? != 3 {
        return Err(GuildBankError::NotCurrent);
    }
    // GUILD1 keeps leader membership until after the payout step. No leader Character lock.
    let leader=sqlx::query("SELECT m.character_id::text,c.account_id::text FROM game_guild_members m JOIN game_character_roots c USING(character_id) WHERE m.guild_id=encode($1,'hex')::uuid AND m.rank_level=1 AND c.world_id=encode($2,'hex')::uuid FOR UPDATE OF m")
        .bind(guild.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let account = uuid_text(&leader.try_get::<String, _>("account_id")?)?;
    let b = binding(
        "GUILD_DISBAND_PAYOUT",
        guild.as_bytes(),
        world.as_bytes(),
        &account,
        0,
    );
    if let Some(receipt) = replay(tx, identity, &b).await? {
        return Ok(receipt);
    }
    // One payout forever per guild, independent of occurrence/session/restart.
    if let Some(row)=sqlx::query("SELECT transaction_id::text,credited,claim_remaining FROM game_guild_bank_operations WHERE guild_id=encode($1,'hex')::uuid AND cause='GUILD_DISBAND_PAYOUT'")
        .bind(guild.as_bytes().as_slice()).fetch_optional(&mut **tx).await? {
        return Ok(GuildBankReceipt{transaction_id:uuid_text(&row.try_get::<String,_>("transaction_id")?)?,credited:value(&row,"credited")?,claim_remaining:value(&row,"claim_remaining")?,replayed:true})
    }
    let before = lock_state(tx, guild, world, account, false).await?;
    if before.guild_held != 0 {
        return Err(GuildBankError::NotCurrent);
    }
    let credit = before
        .guild
        .min(headroom(before.account, before.account_held)?);
    let after = State {
        guild: 0,
        account: before.account + credit,
        claim: before.guild - credit,
        ..before
    };
    persist(
        tx,
        identity,
        guild,
        world,
        account,
        None,
        "GUILD_DISBAND_PAYOUT",
        b,
        before,
        after,
        credit,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    identity: &GuildBankIdentity,
    guild: GuildBankGuildId,
    world: WorldId,
    account: [u8; 16],
    actor: Option<[u8; 16]>,
    cause: &str,
    binding: Vec<u8>,
    before: State,
    after: State,
    credited: u64,
) -> Result<GuildBankReceipt> {
    let audit = GuildBankAuditProposal {
        transaction_id: identity.transaction_id.to_vec(),
        guild_id: guild.as_bytes().to_vec(),
        account_id: account.to_vec(),
        world_id: world.as_bytes().to_vec(),
        cause: cause.into(),
        guild_before: before.guild,
        guild_after: after.guild,
        account_before: before.account,
        account_after: after.account,
        claim_before: before.claim,
        claim_after: after.claim,
    };
    let payload = audit.checked_bytes().ok_or(GuildBankError::InvalidInput)?;
    let digest = Sha256::digest(&payload).to_vec();
    sqlx::query("INSERT INTO game_guild_bank_operations(occurrence_id,transaction_id,guild_id,world_id,account_id,acting_character_id,cause,request_binding,credited,claim_remaining,guild_before,guild_after,account_before,account_after,claim_before,claim_after,occurred_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::uuid,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)")
        .bind(identity.occurrence.as_slice()).bind(identity.transaction_id.as_slice()).bind(guild.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(account.as_slice()).bind(actor.as_ref().map(canonical)).bind(cause).bind(binding).bind(credited as i64).bind(after.claim as i64).bind(before.guild as i64).bind(after.guild as i64).bind(before.account as i64).bind(after.account as i64).bind(before.claim as i64).bind(after.claim as i64).bind(identity.occurred_at_unix_ms).execute(&mut **tx).await?;
    if before.guild != after.guild {
        let delta = i64::try_from(after.guild).map_err(|_| GuildBankError::InvalidInput)?
            - before.guild as i64;
        sqlx::query("INSERT INTO game_guild_bank_entries(entry_id,transaction_id,guild_id,amount,balance_before,balance_after,previous_entry_id,counterpart_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7::uuid,$8::uuid)")
            .bind(identity.guild_entry_id.as_slice()).bind(identity.transaction_id.as_slice()).bind(guild.as_bytes().as_slice()).bind(delta).bind(before.guild as i64).bind(after.guild as i64).bind(before.guild_last.as_ref().map(canonical)).bind((before.account!=after.account).then(||canonical(&identity.bank_entry_id))).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_guild_bank_balances SET balance=$2,last_entry_id=encode($3,'hex')::uuid WHERE guild_id=encode($1,'hex')::uuid").bind(guild.as_bytes().as_slice()).bind(after.guild as i64).bind(identity.guild_entry_id.as_slice()).execute(&mut **tx).await?;
    }
    if before.claim != after.claim || cause == "GUILD_DISBAND_PAYOUT" {
        sqlx::query("INSERT INTO game_guild_disband_claims(guild_id,exleader_account_id,world_id,amount,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid) ON CONFLICT(guild_id) DO UPDATE SET amount=excluded.amount,last_entry_id=excluded.last_entry_id")
            .bind(guild.as_bytes().as_slice()).bind(account.as_slice()).bind(world.as_bytes().as_slice()).bind(after.claim as i64).bind(identity.transaction_id.as_slice()).execute(&mut **tx).await?;
    }
    if before.account != after.account {
        let amount = before.account.abs_diff(after.account);
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,guild_transaction_id,account_id,world_id,kind,guild_kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,NULL,$5,$6,$7,$8,$9::uuid,$10::uuid)")
            .bind(identity.bank_entry_id.as_slice()).bind(identity.transaction_id.as_slice()).bind(account.as_slice()).bind(world.as_bytes().as_slice()).bind(cause).bind(amount as i64).bind(before.account as i64).bind(after.account as i64).bind(before.account_last.as_ref().map(canonical)).bind(actor.as_ref().map(canonical)).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_account_bank_balances SET balance=$3,last_entry_id=encode($4,'hex')::uuid WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
            .bind(account.as_slice()).bind(world.as_bytes().as_slice()).bind(after.account as i64).bind(identity.bank_entry_id.as_slice()).execute(&mut **tx).await?;
    }
    // Claim is one bank event only; other accepted operations require bank+guild events,
    // including a zero-balance disband (no zero ledger entry is manufactured).
    for (purpose, id) in [
        ("BANK_OPERATION", identity.bank_event_id),
        ("GUILD_OPERATION", identity.guild_event_id),
    ] {
        if cause == "GUILD_DISBAND_CLAIM" && purpose == "GUILD_OPERATION" {
            continue;
        }
        sqlx::query("INSERT INTO game_guild_bank_audit_proposals(event_id,transaction_id,purpose,payload,payload_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5)")
            .bind(id.as_slice()).bind(identity.transaction_id.as_slice()).bind(purpose).bind(&payload).bind(&digest).execute(&mut **tx).await?;
    }
    Ok(GuildBankReceipt {
        transaction_id: identity.transaction_id,
        credited,
        claim_remaining: after.claim,
        replayed: false,
    })
}

/// Same-physical-transaction participant for the independently owned, closed GUILDHALL
/// operation. The owning caller has already proven recovery/admission and current
/// Character/leader (or World-job) authority, then locked guild/member/property/bid in
/// GUILD0-LO-01 order. Immutable operation bytes are not that current authority.
/// This function resolves current balances itself; no caller-supplied custody deltas.
pub(super) async fn apply_guildhall_escrow_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    transaction_id: [u8; 16],
) -> Result<()> {
    check_uuid_v7(&transaction_id).map_err(|_| GuildBankError::InvalidInput)?;
    let op=sqlx::query("SELECT guild_id::text,world_id::text,funding_account_id::text,funding_character_id::text,kind,guild_balance_before,guild_balance_after,account_balance_before,account_balance_after,escrow_guild_before,escrow_guild_after,escrow_account_before,escrow_account_after,guild_entry_id::text,account_entry_id::text FROM game_guildhall_operations WHERE transaction_id=encode($1,'hex')::uuid AND created_xact_id=pg_current_xact_id()")
        .bind(transaction_id.as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildBankError::NotCurrent)?;
    let guild = GuildBankGuildId::from_bytes(uuid_text(&op.try_get::<String, _>("guild_id")?)?)?;
    let world = WorldId::decode(&uuid_text(&op.try_get::<String, _>("world_id")?)?)
        .map_err(|_| GuildBankError::InvalidInput)?;
    let account = uuid_text(&op.try_get::<String, _>("funding_account_id")?)?;
    let actor = uuid_text(&op.try_get::<String, _>("funding_character_id")?)?;
    let kind: String = op.try_get("kind")?;
    let reserve = match kind.as_str() {
        "BID_RESERVE" => true,
        "BID_LOWER" | "BID_RELEASE" => false,
        _ => return Err(GuildBankError::InvalidInput),
    };
    let gb = value(&op, "guild_balance_before")?;
    let ga = value(&op, "guild_balance_after")?;
    let ab = value(&op, "account_balance_before")?;
    let aa = value(&op, "account_balance_after")?;
    let eb = value(&op, "escrow_guild_before")?;
    let ea = value(&op, "escrow_guild_after")?;
    let fb = value(&op, "escrow_account_before")?;
    let fa = value(&op, "escrow_account_after")?;
    if ga > BANK_MAX
        || aa > HARD_CEILING
        || gb.checked_add(eb) != ga.checked_add(ea)
        || ab.checked_add(fb) != aa.checked_add(fa)
        || (reserve && (ga > gb || aa > ab))
        || (!reserve && (ga < gb || aa < ab))
    {
        return Err(GuildBankError::InvalidInput);
    }
    sqlx::query("INSERT INTO game_guild_bank_balances(guild_id,balance) VALUES(encode($1,'hex')::uuid,0) ON CONFLICT DO NOTHING").bind(guild.as_bytes().as_slice()).execute(&mut **tx).await?;
    let g=sqlx::query("SELECT balance,last_entry_id::text FROM game_guild_bank_balances WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE").bind(guild.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0) ON CONFLICT DO NOTHING").bind(account.as_slice()).bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
    let a=sqlx::query("SELECT balance,last_entry_id::text FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE").bind(account.as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if value(&g, "balance")? != gb || value(&a, "balance")? != ab {
        return Err(GuildBankError::NotCurrent);
    }
    // The bid transition has already made the current HELD sums reflect the operation's after.
    if ga
        .checked_add(held(tx, "guild", guild.as_bytes(), world.as_bytes()).await?)
        .is_none_or(|v| v > BANK_MAX)
        || headroom(aa, held(tx, "account", &account, world.as_bytes()).await?).is_err()
    {
        return Err(GuildBankError::BalanceLimit);
    }
    let ge = last(&op, "guild_entry_id")?;
    let ae = last(&op, "account_entry_id")?;
    if (ga != gb) != ge.is_some() || (aa != ab) != ae.is_some() {
        return Err(GuildBankError::InvalidInput);
    }
    if let Some(entry) = ge {
        sqlx::query("INSERT INTO game_guild_bank_entries(entry_id,guildhall_transaction_id,guild_id,amount,balance_before,balance_after,previous_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7::uuid)")
            .bind(entry.as_slice()).bind(transaction_id.as_slice()).bind(guild.as_bytes().as_slice()).bind(ga as i64-gb as i64).bind(gb as i64).bind(ga as i64).bind(last(&g,"last_entry_id")?.as_ref().map(canonical)).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_guild_bank_balances SET balance=$2,last_entry_id=encode($3,'hex')::uuid WHERE guild_id=encode($1,'hex')::uuid").bind(guild.as_bytes().as_slice()).bind(ga as i64).bind(entry.as_slice()).execute(&mut **tx).await?;
    }
    if let Some(entry) = ae {
        let cause = if reserve {
            "HOUSE_BID_RESERVE"
        } else {
            "HOUSE_BID_RELEASE"
        };
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,guildhall_transaction_id,account_id,world_id,kind,guildhall_kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,NULL,$5,$6,$7,$8,$9::uuid,encode($10,'hex')::uuid)")
            .bind(entry.as_slice()).bind(transaction_id.as_slice()).bind(account.as_slice()).bind(world.as_bytes().as_slice()).bind(cause).bind(ab.abs_diff(aa) as i64).bind(ab as i64).bind(aa as i64).bind(last(&a,"last_entry_id")?.as_ref().map(canonical)).bind(actor.as_slice()).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_account_bank_balances SET balance=$3,last_entry_id=encode($4,'hex')::uuid WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid").bind(account.as_slice()).bind(world.as_bytes().as_slice()).bind(aa as i64).bind(entry.as_slice()).execute(&mut **tx).await?;
    }
    Ok(())
}

// Guild lifecycle tables/guards belong exclusively to GUILD1. This module consumes that
// owner's schema; it must never install surrogate guild/member tables or denial triggers.
// Exact unmerged owner inputs for isolated qualification are bound by the owned tests.

/// Private UNINSTALLED proposal. Migration owner executes only in isolated qualification;
/// never run as runtime startup/compatibility repair. No migration number or wire/event ID.
pub const GUILD_BANK_SCHEMA_PROPOSAL: &str = r#"-- game_guildhall_bids belongs to GUILDHALL1. Its exact candidate DDL must be installed
-- before this proposal; an absent source is an error, never interpreted as zero escrow.
CREATE TABLE game_guild_bank_operations(
 occurrence_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(occurrence_id)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 guild_id UUID NOT NULL REFERENCES game_guilds,world_id UUID NOT NULL,
 account_id UUID NOT NULL REFERENCES game_character_account_guards,
 acting_character_id UUID NULL REFERENCES game_character_roots,
 cause TEXT NOT NULL CHECK(cause IN('GUILD_DEPOSIT','GUILD_WITHDRAW','GUILD_DISBAND_PAYOUT','GUILD_DISBAND_CLAIM')),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 credited BIGINT NOT NULL CHECK(credited BETWEEN 0 AND 9000000000000000),
 claim_remaining BIGINT NOT NULL CHECK(claim_remaining BETWEEN 0 AND 9000000000000000),
 guild_before BIGINT NOT NULL CHECK(guild_before BETWEEN 0 AND 999999999999),
 guild_after BIGINT NOT NULL CHECK(guild_after BETWEEN 0 AND 999999999999),
 account_before BIGINT NOT NULL CHECK(account_before BETWEEN 0 AND 9000000000000000),
 account_after BIGINT NOT NULL CHECK(account_after BETWEEN 0 AND 9000000000000000),
 claim_before BIGINT NOT NULL CHECK(claim_before BETWEEN 0 AND 9000000000000000),
 claim_after BIGINT NOT NULL CHECK(claim_after BETWEEN 0 AND 9000000000000000),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(guild_before+account_before+claim_before=guild_after+account_after+claim_after),
 CHECK(claim_remaining=claim_after),
 CHECK((cause='GUILD_DISBAND_PAYOUT')=(acting_character_id IS NULL)),
 CHECK(CASE cause
 WHEN 'GUILD_DEPOSIT' THEN guild_after>guild_before AND account_after<account_before AND claim_before=claim_after AND credited=guild_after-guild_before
 WHEN 'GUILD_WITHDRAW' THEN guild_after<guild_before AND account_after>account_before AND account_after<=999999999999 AND claim_before=claim_after AND credited=account_after-account_before
 WHEN 'GUILD_DISBAND_PAYOUT' THEN guild_after=0 AND claim_before=0 AND account_after>=account_before AND credited=account_after-account_before
 ELSE guild_before=0 AND guild_after=0 AND claim_after<claim_before AND account_after>account_before AND credited=account_after-account_before END));
CREATE UNIQUE INDEX game_guild_disband_payout_once ON game_guild_bank_operations(guild_id) WHERE cause='GUILD_DISBAND_PAYOUT';
CREATE TABLE game_guild_bank_entries(
 entry_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(entry_id)),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_guild_bank_operations(transaction_id),
 guild_id UUID NOT NULL REFERENCES game_guilds,amount BIGINT NOT NULL CHECK(amount<>0),
 balance_before BIGINT NOT NULL CHECK(balance_before BETWEEN 0 AND 999999999999),
 balance_after BIGINT NOT NULL CHECK(balance_after BETWEEN 0 AND 999999999999),
 previous_entry_id UUID NULL UNIQUE REFERENCES game_guild_bank_entries,
 counterpart_entry_id UUID NULL UNIQUE REFERENCES game_account_bank_entries DEFERRABLE INITIALLY DEFERRED,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),CHECK(balance_before+amount=balance_after),
 CHECK(previous_entry_id IS DISTINCT FROM entry_id),UNIQUE(entry_id,guild_id));
CREATE UNIQUE INDEX game_guild_bank_first_entry ON game_guild_bank_entries(guild_id) WHERE previous_entry_id IS NULL;
CREATE TABLE game_guild_bank_balances(guild_id UUID PRIMARY KEY REFERENCES game_guilds,
 balance BIGINT NOT NULL CHECK(balance BETWEEN 0 AND 999999999999),last_entry_id UUID NULL UNIQUE,
 CHECK(last_entry_id IS NOT NULL OR balance=0),FOREIGN KEY(last_entry_id,guild_id) REFERENCES game_guild_bank_entries(entry_id,guild_id));
CREATE TABLE game_guild_disband_claims(guild_id UUID PRIMARY KEY REFERENCES game_guilds,
 exleader_account_id UUID NOT NULL REFERENCES game_character_account_guards,world_id UUID NOT NULL,
 amount BIGINT NOT NULL CHECK(amount BETWEEN 0 AND 9000000000000000),
 last_entry_id UUID NOT NULL REFERENCES game_guild_bank_operations(transaction_id),created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp());
CREATE INDEX game_guild_claim_account_world ON game_guild_disband_claims(exleader_account_id,world_id,guild_id) WHERE amount>0;
CREATE TABLE game_guild_bank_audit_proposals(event_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)),
 transaction_id UUID NOT NULL REFERENCES game_guild_bank_operations(transaction_id),
 purpose TEXT NOT NULL CHECK(purpose IN('BANK_OPERATION','GUILD_OPERATION')),
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 1024),
 payload_sha256 BYTEA NOT NULL CHECK(payload_sha256=sha256(payload)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),UNIQUE(transaction_id,purpose));
-- No registered event ID, envelope schema, retention or publication API is fabricated here.
-- Required native bank extension, symbolic candidate kinds (NULL native kind is NOT a new ID).
ALTER TABLE game_account_bank_entries ADD COLUMN guild_transaction_id UUID NULL UNIQUE REFERENCES game_guild_bank_operations(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN guild_kind TEXT NULL,ALTER COLUMN kind DROP NOT NULL,ALTER COLUMN acting_character_id DROP NOT NULL,
 DROP CONSTRAINT game_account_bank_entries_one_reference,
 DROP CONSTRAINT game_account_bank_entries_fee_reference,
 DROP CONSTRAINT game_account_bank_entries_check,
 ADD CONSTRAINT game_account_bank_entries_check CHECK(kind IS NULL OR balance_after=CASE WHEN kind IN(1,4) THEN balance_before+amount ELSE balance_before-amount END),
 DROP CONSTRAINT game_account_bank_entries_balance_before_check,
 DROP CONSTRAINT game_account_bank_entries_balance_after_check,
 ADD CONSTRAINT game_account_bank_entries_balance_before_check CHECK(balance_before BETWEEN 0 AND 9000000000000000),
 ADD CONSTRAINT game_account_bank_entries_balance_after_check CHECK(balance_after BETWEEN 0 AND 9000000000000000),
 ADD CONSTRAINT game_account_bank_entries_one_reference CHECK(num_nonnulls(transaction_id,fee_transaction_id,guild_transaction_id)=1),
 ADD CONSTRAINT game_account_bank_entries_fee_reference CHECK((kind IS NOT DISTINCT FROM 5)=(fee_transaction_id IS NOT NULL)),
 ADD CONSTRAINT game_account_bank_entries_guild_reference CHECK(
   (guild_transaction_id IS NULL AND guild_kind IS NULL AND kind IS NOT NULL AND acting_character_id IS NOT NULL)
   OR (guild_transaction_id IS NOT NULL AND kind IS NULL AND transaction_id IS NULL AND fee_transaction_id IS NULL
      AND guild_kind IS NOT NULL AND guild_kind IN('GUILD_DEPOSIT','GUILD_WITHDRAW','GUILD_DISBAND_PAYOUT','GUILD_DISBAND_CLAIM')
      AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL
      AND ((guild_kind='GUILD_DISBAND_PAYOUT')=(acting_character_id IS NULL))
      AND balance_after=CASE WHEN guild_kind='GUILD_DEPOSIT' THEN balance_before-amount ELSE balance_before+amount END));
ALTER TABLE game_account_bank_balances DROP CONSTRAINT game_account_bank_balances_balance_check,
 ADD CONSTRAINT game_account_bank_balances_balance_check CHECK(balance BETWEEN 0 AND 9000000000000000);
CREATE FUNCTION game_guild_bank_operation_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_bank_operations%ROWTYPE; g INTEGER; a INTEGER; events INTEGER; leader_account UUID;
BEGIN
 SELECT * INTO o FROM game_guild_bank_operations WHERE transaction_id=NEW.transaction_id;
 IF NOT FOUND OR o.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'guild operation not current transaction' USING ERRCODE='23514';END IF;
 SELECT count(*) INTO g FROM game_guild_bank_entries WHERE transaction_id=o.transaction_id;
 SELECT count(*) INTO a FROM game_account_bank_entries WHERE guild_transaction_id=o.transaction_id;
 SELECT count(*) INTO events FROM game_guild_bank_audit_proposals WHERE transaction_id=o.transaction_id AND created_xact_id=pg_current_xact_id();
 IF g<>(o.guild_before<>o.guild_after)::integer OR a<>(o.account_before<>o.account_after)::integer
    OR events<>(CASE WHEN o.cause='GUILD_DISBAND_CLAIM' THEN 1 ELSE 2 END)
    OR NOT EXISTS(SELECT 1 FROM game_guild_bank_audit_proposals WHERE transaction_id=o.transaction_id AND purpose='BANK_OPERATION')
    OR (o.cause<>'GUILD_DISBAND_CLAIM' AND NOT EXISTS(SELECT 1 FROM game_guild_bank_audit_proposals WHERE transaction_id=o.transaction_id AND purpose='GUILD_OPERATION'))
    OR (SELECT count(DISTINCT payload_sha256) FROM game_guild_bank_audit_proposals WHERE transaction_id=o.transaction_id)<>1
 THEN RAISE EXCEPTION 'guild operation requires exact ledgers and dual audit' USING ERRCODE='23514';END IF;
 IF o.cause<>'GUILD_DISBAND_CLAIM' AND NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state=CASE WHEN o.cause='GUILD_DISBAND_PAYOUT' THEN 3 ELSE state END AND (o.cause='GUILD_DISBAND_PAYOUT' OR state IN(1,2))) THEN
 RAISE EXCEPTION 'guild state/world mismatch' USING ERRCODE='23514';END IF;
 IF o.cause IN('GUILD_DEPOSIT','GUILD_WITHDRAW') AND NOT EXISTS(
 SELECT 1 FROM game_guild_members m JOIN game_character_roots c USING(character_id)
 WHERE m.guild_id=o.guild_id AND m.character_id=o.acting_character_id AND c.account_id=o.account_id AND c.world_id=o.world_id AND c.lifecycle=1 AND (o.cause='GUILD_DEPOSIT' OR m.rank_level<=2)) THEN
 RAISE EXCEPTION 'guild membership mismatch' USING ERRCODE='23514';END IF;
 IF o.cause='GUILD_DISBAND_PAYOUT' THEN
 SELECT c.account_id INTO leader_account FROM game_guild_members m JOIN game_character_roots c USING(character_id) WHERE m.guild_id=o.guild_id AND m.rank_level=1 AND c.world_id=o.world_id;
 IF leader_account IS DISTINCT FROM o.account_id OR EXISTS(SELECT 1 FROM game_guildhall_bids WHERE guild_id=o.guild_id AND state='HELD') THEN RAISE EXCEPTION 'disband requires leader and completed bid release' USING ERRCODE='23514';END IF;
 END IF;
 IF o.cause IN('GUILD_DISBAND_PAYOUT','GUILD_DISBAND_CLAIM') AND NOT EXISTS(
 SELECT 1 FROM game_guild_disband_claims c WHERE c.guild_id=o.guild_id AND c.exleader_account_id=o.account_id AND c.world_id=o.world_id AND c.amount=o.claim_after AND c.last_entry_id=o.transaction_id) THEN RAISE EXCEPTION 'claim must commit with payout/claim' USING ERRCODE='23514';END IF;
 RETURN NULL;
END;$$;
CREATE FUNCTION game_guild_bank_balance_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
 IF NEW.balance<>0 OR NEW.last_entry_id IS NOT NULL THEN RAISE EXCEPTION 'guild balance starts at zero' USING ERRCODE='23514';END IF;
 ELSE
 IF NEW.guild_id<>OLD.guild_id OR NOT EXISTS(SELECT 1 FROM game_guild_bank_entries e WHERE e.entry_id=NEW.last_entry_id AND e.guild_id=NEW.guild_id AND e.balance_before=OLD.balance AND e.balance_after=NEW.balance AND e.previous_entry_id IS NOT DISTINCT FROM OLD.last_entry_id AND e.created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'guild balance must advance exact ledger' USING ERRCODE='23514';END IF;
 END IF;RETURN NULL;
END;$$;
CREATE FUNCTION game_guild_bank_entry_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guild_bank_operations o WHERE o.transaction_id=NEW.transaction_id AND o.guild_id=NEW.guild_id AND o.guild_before=NEW.balance_before AND o.guild_after=NEW.balance_after AND o.created_xact_id=pg_current_xact_id())
 OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
 OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guild_bank_entries p WHERE p.entry_id=NEW.previous_entry_id AND p.guild_id=NEW.guild_id AND p.balance_after=NEW.balance_before))
 OR NOT(EXISTS(SELECT 1 FROM game_guild_bank_balances b WHERE b.guild_id=NEW.guild_id AND b.last_entry_id=NEW.entry_id) OR EXISTS(SELECT 1 FROM game_guild_bank_entries s WHERE s.previous_entry_id=NEW.entry_id))
 OR (NEW.counterpart_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries a WHERE a.entry_id=NEW.counterpart_entry_id AND a.guild_transaction_id=NEW.transaction_id))
 THEN RAISE EXCEPTION 'guild ledger mismatch' USING ERRCODE='23514';END IF;RETURN NULL;
END;$$;
CREATE FUNCTION game_guild_bank_claim_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND(NEW.guild_id<>OLD.guild_id OR NEW.exleader_account_id<>OLD.exleader_account_id OR NEW.world_id<>OLD.world_id OR NEW.created_at<>OLD.created_at OR NEW.amount>=OLD.amount) THEN RAISE EXCEPTION 'claim immutable binding and strictly falling amount' USING ERRCODE='23514';END IF;
 IF NOT EXISTS(SELECT 1 FROM game_guild_bank_operations o WHERE o.transaction_id=NEW.last_entry_id AND o.guild_id=NEW.guild_id AND o.account_id=NEW.exleader_account_id AND o.world_id=NEW.world_id AND o.claim_after=NEW.amount AND o.claim_before=CASE WHEN TG_OP='INSERT' THEN 0 ELSE OLD.amount END AND o.cause=CASE WHEN TG_OP='INSERT' THEN 'GUILD_DISBAND_PAYOUT' ELSE 'GUILD_DISBAND_CLAIM' END AND o.created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'claim requires exact same-transaction cause' USING ERRCODE='23514';END IF;RETURN NULL;
END;$$;
CREATE FUNCTION game_guild_bank_headroom_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE aid UUID;wid UUID;gid UUID;
BEGIN
 IF TG_TABLE_NAME='game_guildhall_bids' THEN aid:=NEW.funding_account_id;wid:=NEW.world_id;gid:=NEW.guild_id;
 ELSIF TG_TABLE_NAME='game_account_bank_balances' THEN aid:=NEW.account_id;wid:=NEW.world_id;
 ELSE gid:=NEW.guild_id;END IF;
 IF aid IS NOT NULL AND coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=aid AND world_id=wid),0)+coalesce((SELECT sum(escrow_account_gold) FROM game_guildhall_bids WHERE funding_account_id=aid AND world_id=wid AND state='HELD'),0)>9000000000000000 THEN RAISE EXCEPTION 'account held headroom exceeded' USING ERRCODE='23514';END IF;
 IF gid IS NOT NULL AND coalesce((SELECT balance FROM game_guild_bank_balances WHERE guild_id=gid),0)+coalesce((SELECT sum(escrow_guild_gold) FROM game_guildhall_bids WHERE guild_id=gid AND state='HELD'),0)>999999999999 THEN RAISE EXCEPTION 'guild held headroom exceeded' USING ERRCODE='23514';END IF;RETURN NULL;
END;$$;
CREATE CONSTRAINT TRIGGER game_guild_bank_operation_consistent AFTER INSERT ON game_guild_bank_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_operation_guard();
CREATE CONSTRAINT TRIGGER game_guild_bank_entry_consistent AFTER INSERT ON game_guild_bank_entries DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_entry_guard();
CREATE CONSTRAINT TRIGGER game_guild_bank_audit_consistent AFTER INSERT ON game_guild_bank_audit_proposals DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_operation_guard();
CREATE CONSTRAINT TRIGGER game_guild_bank_balance_consistent AFTER INSERT OR UPDATE ON game_guild_bank_balances DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_balance_guard();
CREATE CONSTRAINT TRIGGER game_guild_bank_claim_consistent AFTER INSERT OR UPDATE ON game_guild_disband_claims DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_claim_guard();
CREATE CONSTRAINT TRIGGER game_guild_bank_headroom AFTER INSERT OR UPDATE ON game_guild_bank_balances DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_headroom_guard();
CREATE CONSTRAINT TRIGGER game_guild_bank_account_headroom AFTER INSERT OR UPDATE ON game_account_bank_balances DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_headroom_guard();
CREATE CONSTRAINT TRIGGER game_guild_bank_escrow_headroom AFTER INSERT OR UPDATE ON game_guildhall_bids DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_bank_headroom_guard();

-- Latest native 0072 guard body retained verbatim after the new closed guild arm.
CREATE OR REPLACE FUNCTION game_account_bank_entry_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN

    IF NEW.guild_transaction_id IS NOT NULL THEN
      IF NEW.created_xact_id<>pg_current_xact_id()
      OR NOT EXISTS(SELECT 1 FROM game_guild_bank_operations o WHERE o.transaction_id=NEW.guild_transaction_id AND o.account_id=NEW.account_id AND o.world_id=NEW.world_id AND o.cause=NEW.guild_kind AND o.account_before=NEW.balance_before AND o.account_after=NEW.balance_after AND o.acting_character_id IS NOT DISTINCT FROM NEW.acting_character_id AND o.created_xact_id=pg_current_xact_id())
      OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
      OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.entry_id=NEW.previous_entry_id AND p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.balance_after=NEW.balance_before))
      OR NOT(EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=NEW.account_id AND b.world_id=NEW.world_id AND b.last_entry_id=NEW.entry_id) OR EXISTS(SELECT 1 FROM game_account_bank_entries s WHERE s.previous_entry_id=NEW.entry_id))
      OR (NEW.acting_character_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=NEW.acting_character_id AND c.account_id=NEW.account_id AND c.world_id=NEW.world_id AND c.lifecycle=1))
      THEN RAISE EXCEPTION 'guild bank entry mismatch' USING ERRCODE='23514';END IF;
      RETURN NULL;
    END IF;
    IF (NEW.kind <> 5 AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_operations o
            WHERE o.transaction_id = NEW.transaction_id
              AND o.outcome = 0
              AND o.created_xact_id = pg_current_xact_id()))
       OR (NEW.kind = 5 AND NOT EXISTS (
           SELECT 1 FROM game_item_fee_burns f
            WHERE f.transaction_id = NEW.fee_transaction_id
              AND f.bank_debit_gold_units = NEW.amount
              AND f.world_id = NEW.world_id
              AND f.character_id = NEW.acting_character_id
              AND f.created_xact_id = pg_current_xact_id()))
       OR NEW.created_xact_id <> pg_current_xact_id()
       OR (NEW.previous_entry_id IS NULL AND NEW.balance_before <> 0)
       OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries p
            WHERE p.entry_id = NEW.previous_entry_id
              AND p.account_id = NEW.account_id AND p.world_id = NEW.world_id
              AND p.balance_after = NEW.balance_before))
       OR NOT (EXISTS (
           SELECT 1 FROM game_account_bank_balances b
            WHERE b.account_id = NEW.account_id AND b.world_id = NEW.world_id
              AND b.last_entry_id = NEW.entry_id)
           OR EXISTS (
           SELECT 1 FROM game_account_bank_entries s WHERE s.previous_entry_id = NEW.entry_id))
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = CASE WHEN NEW.kind = 4 THEN NEW.recipient_character_id
                                         ELSE NEW.acting_character_id END
              AND cr.account_id = NEW.account_id
              AND cr.world_id = NEW.world_id
              AND cr.lifecycle = 1)
       OR (NEW.recipient_character_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.recipient_character_id
              AND cr.world_id = NEW.world_id AND cr.lifecycle = 1))
       OR (NEW.counterpart_entry_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries c
            WHERE c.entry_id = NEW.counterpart_entry_id
              AND c.transaction_id = NEW.transaction_id
              AND c.counterpart_entry_id = NEW.entry_id
              AND c.amount = NEW.amount
              AND c.account_id <> NEW.account_id
              AND c.kind = 7 - NEW.kind)) THEN
        RAISE EXCEPTION 'a bank ledger entry must chain from the previous entry of its live account balance'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_guild_bank_operations','game_guild_bank_entries','game_guild_bank_audit_proposals'] LOOP
 EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable()',t||'_immutable',t);
 END LOOP;
 FOREACH t IN ARRAY ARRAY['game_guild_bank_balances','game_guild_disband_claims'] LOOP
 EXECUTE format('CREATE TRIGGER %I BEFORE DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_account_bank_immutable()',t||'_undeletable',t);
 END LOOP;
 FOREACH t IN ARRAY ARRAY['game_guild_bank_operations','game_guild_bank_entries','game_guild_bank_audit_proposals','game_guild_bank_balances','game_guild_disband_claims'] LOOP
 EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',t||'_no_truncate',t);
 END LOOP;
 FOREACH t IN ARRAY ARRAY['game_guild_bank_operation_guard','game_guild_bank_balance_guard','game_guild_bank_entry_guard','game_guild_bank_claim_guard','game_guild_bank_headroom_guard','game_account_bank_entry_consistency_guard'] LOOP
 EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',t,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',t);
 END LOOP;
END;$$;
GRANT SELECT ON game_guilds,game_guild_ranks,game_guild_members,game_guildhall_bids TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_guild_bank_operations,game_guild_bank_entries,game_guild_bank_audit_proposals TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_guild_bank_balances,game_guild_disband_claims TO oteryn_game_runtime;
GRANT SELECT ON game_guild_bank_operations,game_guild_bank_entries,game_guild_bank_audit_proposals,game_guild_bank_balances,game_guild_disband_claims TO oteryn_game_control;
"#;

/// Uninstalled composition extension for actual closed GUILDHALL escrow operations.
/// It installs no actor authority, runtime wiring, event ID or burn/fee arm.
pub const GUILDHALL_LEDGER_SCHEMA_PROPOSAL: &str = r#"
-- Apply after GUILD_BANK_SCHEMA_PROPOSAL and exact GUILDHALL OPERATION_SCHEMA.
ALTER TABLE game_account_bank_entries
 ADD COLUMN guildhall_transaction_id UUID NULL UNIQUE REFERENCES game_guildhall_operations(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN guildhall_kind TEXT NULL,
 DROP CONSTRAINT game_account_bank_entries_one_reference,
 DROP CONSTRAINT game_account_bank_entries_guild_reference,
 DROP CONSTRAINT game_account_bank_entries_amount_check,
 ADD CONSTRAINT game_account_bank_entries_one_reference CHECK(num_nonnulls(transaction_id,fee_transaction_id,guild_transaction_id,guildhall_transaction_id)=1),
 ADD CONSTRAINT game_account_bank_entries_amount_check CHECK(amount BETWEEN 1 AND CASE WHEN guildhall_transaction_id IS NOT NULL THEN 9000000000000000 ELSE 999999999999 END),
 ADD CONSTRAINT game_account_bank_entries_guild_reference CHECK(
 (guildhall_transaction_id IS NULL AND guildhall_kind IS NULL AND(
 (guild_transaction_id IS NULL AND guild_kind IS NULL AND kind IS NOT NULL AND acting_character_id IS NOT NULL)
 OR(guild_transaction_id IS NOT NULL AND kind IS NULL AND transaction_id IS NULL AND fee_transaction_id IS NULL
 AND guild_kind IS NOT NULL AND guild_kind IN('GUILD_DEPOSIT','GUILD_WITHDRAW','GUILD_DISBAND_PAYOUT','GUILD_DISBAND_CLAIM')
 AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL
 AND((guild_kind='GUILD_DISBAND_PAYOUT')=(acting_character_id IS NULL))
 AND balance_after=CASE WHEN guild_kind='GUILD_DEPOSIT' THEN balance_before-amount ELSE balance_before+amount END)))
 OR(guildhall_transaction_id IS NOT NULL AND guild_transaction_id IS NULL AND guild_kind IS NULL AND kind IS NULL AND acting_character_id IS NOT NULL
 AND guildhall_kind IS NOT NULL AND guildhall_kind IN('HOUSE_BID_RESERVE','HOUSE_BID_RELEASE')
 AND recipient_character_id IS NULL AND counterpart_entry_id IS NULL
 AND balance_after=CASE WHEN guildhall_kind='HOUSE_BID_RESERVE' THEN balance_before-amount ELSE balance_before+amount END));
ALTER TABLE game_guild_bank_entries ALTER COLUMN transaction_id DROP NOT NULL,
 ADD COLUMN guildhall_transaction_id UUID NULL UNIQUE REFERENCES game_guildhall_operations(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 ADD CONSTRAINT game_guild_bank_entry_one_reference CHECK(num_nonnulls(transaction_id,guildhall_transaction_id)=1);

-- Complete previously extended bank guard preserved after this closed Hall arm.
CREATE OR REPLACE FUNCTION game_account_bank_entry_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN

    IF NEW.guildhall_transaction_id IS NOT NULL THEN
      IF NEW.created_xact_id<>pg_current_xact_id()
      OR NOT EXISTS(SELECT 1 FROM game_guildhall_operations o WHERE o.transaction_id=NEW.guildhall_transaction_id AND o.account_entry_id=NEW.entry_id AND o.funding_account_id=NEW.account_id AND o.funding_character_id=NEW.acting_character_id AND o.world_id=NEW.world_id AND o.account_balance_before=NEW.balance_before AND o.account_balance_after=NEW.balance_after AND NEW.guildhall_kind=CASE WHEN o.kind='BID_RESERVE' THEN 'HOUSE_BID_RESERVE' ELSE 'HOUSE_BID_RELEASE' END AND o.created_xact_id=pg_current_xact_id())
      OR(NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
      OR(NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.entry_id=NEW.previous_entry_id AND p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.balance_after=NEW.balance_before))
      OR NOT(EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=NEW.account_id AND b.world_id=NEW.world_id AND b.last_entry_id=NEW.entry_id) OR EXISTS(SELECT 1 FROM game_account_bank_entries c WHERE c.previous_entry_id=NEW.entry_id))
      OR NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=NEW.acting_character_id AND c.account_id=NEW.account_id AND c.world_id=NEW.world_id)
      THEN RAISE EXCEPTION 'guildhall account ledger mismatch' USING ERRCODE='23514';END IF;RETURN NULL;
    END IF;

    IF NEW.guild_transaction_id IS NOT NULL THEN
      IF NEW.created_xact_id<>pg_current_xact_id()
      OR NOT EXISTS(SELECT 1 FROM game_guild_bank_operations o WHERE o.transaction_id=NEW.guild_transaction_id AND o.account_id=NEW.account_id AND o.world_id=NEW.world_id AND o.cause=NEW.guild_kind AND o.account_before=NEW.balance_before AND o.account_after=NEW.balance_after AND o.acting_character_id IS NOT DISTINCT FROM NEW.acting_character_id AND o.created_xact_id=pg_current_xact_id())
      OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
      OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries p WHERE p.entry_id=NEW.previous_entry_id AND p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.balance_after=NEW.balance_before))
      OR NOT(EXISTS(SELECT 1 FROM game_account_bank_balances b WHERE b.account_id=NEW.account_id AND b.world_id=NEW.world_id AND b.last_entry_id=NEW.entry_id) OR EXISTS(SELECT 1 FROM game_account_bank_entries s WHERE s.previous_entry_id=NEW.entry_id))
      OR (NEW.acting_character_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=NEW.acting_character_id AND c.account_id=NEW.account_id AND c.world_id=NEW.world_id AND c.lifecycle=1))
      THEN RAISE EXCEPTION 'guild bank entry mismatch' USING ERRCODE='23514';END IF;
      RETURN NULL;
    END IF;
    IF (NEW.kind <> 5 AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_operations o
            WHERE o.transaction_id = NEW.transaction_id
              AND o.outcome = 0
              AND o.created_xact_id = pg_current_xact_id()))
       OR (NEW.kind = 5 AND NOT EXISTS (
           SELECT 1 FROM game_item_fee_burns f
            WHERE f.transaction_id = NEW.fee_transaction_id
              AND f.bank_debit_gold_units = NEW.amount
              AND f.world_id = NEW.world_id
              AND f.character_id = NEW.acting_character_id
              AND f.created_xact_id = pg_current_xact_id()))
       OR NEW.created_xact_id <> pg_current_xact_id()
       OR (NEW.previous_entry_id IS NULL AND NEW.balance_before <> 0)
       OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries p
            WHERE p.entry_id = NEW.previous_entry_id
              AND p.account_id = NEW.account_id AND p.world_id = NEW.world_id
              AND p.balance_after = NEW.balance_before))
       OR NOT (EXISTS (
           SELECT 1 FROM game_account_bank_balances b
            WHERE b.account_id = NEW.account_id AND b.world_id = NEW.world_id
              AND b.last_entry_id = NEW.entry_id)
           OR EXISTS (
           SELECT 1 FROM game_account_bank_entries s WHERE s.previous_entry_id = NEW.entry_id))
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = CASE WHEN NEW.kind = 4 THEN NEW.recipient_character_id
                                         ELSE NEW.acting_character_id END
              AND cr.account_id = NEW.account_id
              AND cr.world_id = NEW.world_id
              AND cr.lifecycle = 1)
       OR (NEW.recipient_character_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.recipient_character_id
              AND cr.world_id = NEW.world_id AND cr.lifecycle = 1))
       OR (NEW.counterpart_entry_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_account_bank_entries c
            WHERE c.entry_id = NEW.counterpart_entry_id
              AND c.transaction_id = NEW.transaction_id
              AND c.counterpart_entry_id = NEW.entry_id
              AND c.amount = NEW.amount
              AND c.account_id <> NEW.account_id
              AND c.kind = 7 - NEW.kind)) THEN
        RAISE EXCEPTION 'a bank ledger entry must chain from the previous entry of its live account balance'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_guild_bank_entry_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN

 IF NEW.guildhall_transaction_id IS NOT NULL THEN
 IF NEW.created_xact_id<>pg_current_xact_id()
 OR NOT EXISTS(SELECT 1 FROM game_guildhall_operations o WHERE o.transaction_id=NEW.guildhall_transaction_id AND o.guild_entry_id=NEW.entry_id AND o.guild_id=NEW.guild_id AND o.guild_balance_before=NEW.balance_before AND o.guild_balance_after=NEW.balance_after AND o.created_xact_id=pg_current_xact_id())
 OR NEW.counterpart_entry_id IS NOT NULL
 OR(NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
 OR(NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guild_bank_entries p WHERE p.entry_id=NEW.previous_entry_id AND p.guild_id=NEW.guild_id AND p.balance_after=NEW.balance_before))
 OR NOT(EXISTS(SELECT 1 FROM game_guild_bank_balances b WHERE b.guild_id=NEW.guild_id AND b.last_entry_id=NEW.entry_id) OR EXISTS(SELECT 1 FROM game_guild_bank_entries c WHERE c.previous_entry_id=NEW.entry_id))
 THEN RAISE EXCEPTION 'guildhall guild ledger mismatch' USING ERRCODE='23514';END IF;RETURN NULL;
 END IF;
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guild_bank_operations o WHERE o.transaction_id=NEW.transaction_id AND o.guild_id=NEW.guild_id AND o.guild_before=NEW.balance_before AND o.guild_after=NEW.balance_after AND o.created_xact_id=pg_current_xact_id())
 OR (NEW.previous_entry_id IS NULL AND NEW.balance_before<>0)
 OR (NEW.previous_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_guild_bank_entries p WHERE p.entry_id=NEW.previous_entry_id AND p.guild_id=NEW.guild_id AND p.balance_after=NEW.balance_before))
 OR NOT(EXISTS(SELECT 1 FROM game_guild_bank_balances b WHERE b.guild_id=NEW.guild_id AND b.last_entry_id=NEW.entry_id) OR EXISTS(SELECT 1 FROM game_guild_bank_entries s WHERE s.previous_entry_id=NEW.entry_id))
 OR (NEW.counterpart_entry_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries a WHERE a.entry_id=NEW.counterpart_entry_id AND a.guild_transaction_id=NEW.transaction_id))
 THEN RAISE EXCEPTION 'guild ledger mismatch' USING ERRCODE='23514';END IF;RETURN NULL;
END;$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_account_bank_entry_consistency_guard() SET search_path=%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_guild_bank_entry_guard() SET search_path=%I,pg_temp',current_schema());
END;$$;
"#;

#[cfg(test)]
#[path = "guild_bank_tests.rs"]
mod tests;
