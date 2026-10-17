//! PREY-0 §4/§10 and DAILY_REWARD0 §4.5: the pending DAILY wildcard sink.
//!
//! A retained DAILY claim/line supplies the quantity; it never supplies current
//! authority. A previously unseen delivery uses the native gameplay fence in
//! the writing transaction, advances CharacterRevision once, and commits its
//! PREY receipt, Account+World ledger, event and terminal DAILY line together.
//! This is not a general grant API, the claim writer, or a Store/test source.
//! Shared migration, receipt-chain, sequencer, event and runtime registration
//! belong to HUNT's integration slot; merely mounting this module enables none.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::prey_audit::{self, DailyWildcardEvent, WildcardEventIdentity};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{AccountId, CharacterId, CharacterRevision};
use crate::foundation::{RuntimeScopeRefV1, WorldId};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, PreyWildcardError>;
const WILDCARDS_MAX: u8 = 52;
const BINDING_VERSION: u8 = 1;

/// The source identity and planned delivery identities, fixed before retry.
/// Quantity, Account, World and credit/excess are deliberately absent: the
/// writer resolves them from the real source row and currently fenced root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDailyWildcardRequest {
    pub claim_transaction_id: [u8; 16],
    pub line: u32,
    pub delivery_transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub server_build_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedDailyWildcardDelivery {
    pub claim_transaction_id: [u8; 16],
    pub line: u32,
    pub delivery_transaction_id: [u8; 16],
    pub character_id: CharacterId,
    pub account_id: AccountId,
    pub world_id: WorldId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub quantity: u64,
    pub credited: u8,
    pub uncredited_excess: u64,
    pub balance_before: u8,
    pub balance_after: u8,
    pub entry_id: Option<[u8; 16]>,
    pub event_id: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DailyWildcardOutcome {
    Committed(CommittedDailyWildcardDelivery),
    AlreadyCommitted(CommittedDailyWildcardDelivery),
}

#[derive(Debug)]
pub enum PreyWildcardError {
    InvalidInput,
    AuthorityRejected,
    CharacterRevisionMismatch,
    ProgressionContextMismatch,
    MissingProgressionState,
    MissingDailyLine,
    SourceBindingMismatch,
    NotWildcardLine,
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for PreyWildcardError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl std::fmt::Display for PreyWildcardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid pending DAILY wildcard request"),
            Self::AuthorityRejected => f.write_str("current PREY authority rejected"),
            Self::CharacterRevisionMismatch => f.write_str("PREY CharacterRevision mismatch"),
            Self::ProgressionContextMismatch => f.write_str("PREY progression context mismatch"),
            Self::MissingProgressionState => f.write_str("PREY progression state is missing"),
            Self::MissingDailyLine => f.write_str("pending DAILY line is missing"),
            Self::SourceBindingMismatch => {
                f.write_str("DAILY source does not belong to this actor")
            }
            Self::NotWildcardLine => f.write_str("DAILY line is not a PREY wildcard reward"),
            Self::ConflictingOccurrence => f.write_str("PREY delivery occurrence conflicts"),
            Self::Unavailable(error) => write!(f, "PREY wildcard store unavailable: {error}"),
        }
    }
}

impl std::error::Error for PreyWildcardError {}

impl DurabilityRoot {
    /// Deliver one already-created pending DAILY line. The runtime caller must
    /// hold this Character's native RevisionSlot; revision-independent binding
    /// permits the sequencer's one current-cursor retry. An exact retained
    /// replay returns its original result and does not reacquire gameplay
    /// authority. Any refusal or error before commit writes nothing.
    pub async fn deliver_pending_daily_wildcard(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PendingDailyWildcardRequest,
    ) -> Result<DailyWildcardOutcome> {
        validate_request(&fence, &request)?;
        let binding = request_binding(&fence, &request);
        let recovery = authority
            .record_for(self)
            .map_err(|_| PreyWildcardError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // The accepted once key is (claim, line), not a caller's
                    // newly planned delivery UUID. Losing callers read the
                    // winner's receipt before attempting any second mutation.
                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:prey-daily:' || encode($1,'hex') || ':' || $2::text, 0))",
                    )
                    .bind(request.claim_transaction_id.as_slice())
                    .bind(i64::from(request.line))
                    .execute(&mut *tx)
                    .await?;
                    if let Some(row) = load_receipt(&mut tx, &request).await? {
                        if row.try_get::<Vec<u8>, _>("command_binding")? != binding {
                            return Ok(Err(PreyWildcardError::ConflictingOccurrence));
                        }
                        let receipt = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(DailyWildcardOutcome::AlreadyCommitted(receipt)));
                    }
                    let reused: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM game_character_prey_receipts \
                         WHERE prey_occurrence_id=encode($1,'hex')::uuid \
                            OR event_id=encode($2,'hex')::uuid)",
                    )
                    .bind(request.delivery_transaction_id.as_slice())
                    .bind(request.event_id.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if reused {
                        return Ok(Err(PreyWildcardError::ConflictingOccurrence));
                    }

                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(fence_error(error))),
                    };
                    let state = sqlx::query(
                        "SELECT character_revision::text, level, total_experience, \
                         profile_revision,ruleset_revision,content_revision,simulation_revision,\
                         evidence_revision,declaration_revision,policy_revision,reward_revision \
                         FROM game_character_progression_state \
                         WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(PreyWildcardError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision
                        || !state_matches_root(&state, &root)
                    {
                        return Ok(Err(PreyWildcardError::ProgressionContextMismatch));
                    }
                    let RuntimeScopeRefV1::Channel {
                        world_id,
                        channel_id,
                    } = fence.runtime_scope
                    else {
                        return Ok(Err(PreyWildcardError::AuthorityRejected));
                    };
                    // assert_gameplay_fence already locked and proved this root
                    // against the independently current session Account/World.
                    let account_text: String = sqlx::query_scalar(
                        "SELECT account_id::text FROM game_character_roots \
                         WHERE character_id=encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    let account_id = AccountId::from_bytes(uuid_text(&account_text)?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;

                    let source = sqlx::query(
                        "SELECT c.character_id::text,c.world_id::text,c.request_binding,\
                         l.kind,l.quantity::text,l.status,l.delivering_transaction_id::text,\
                         l.uncredited_excess::text \
                         FROM game_character_daily_reward_claims c \
                         JOIN game_character_daily_reward_lines l \
                           ON (l.character_id,l.world_id,l.reset_epoch)=\
                              (c.character_id,c.world_id,c.reset_epoch) \
                         WHERE c.transaction_id=encode($1,'hex')::uuid AND l.line=$2 \
                         FOR UPDATE OF l",
                    )
                    .bind(request.claim_transaction_id.as_slice())
                    .bind(i64::from(request.line))
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(source) = source else {
                        return Ok(Err(PreyWildcardError::MissingDailyLine));
                    };
                    if uuid_text(&source.try_get::<String, _>("character_id")?)?
                        != *fence.character_id.as_bytes()
                        || uuid_text(&source.try_get::<String, _>("world_id")?)?
                            != *world_id.as_bytes()
                    {
                        return Ok(Err(PreyWildcardError::SourceBindingMismatch));
                    }
                    if source.try_get::<String, _>("kind")? != "PREY_WILDCARD" {
                        return Ok(Err(PreyWildcardError::NotWildcardLine));
                    }
                    if source.try_get::<String, _>("status")? != "PENDING"
                        || source
                            .try_get::<Option<String>, _>("delivering_transaction_id")?
                            .is_some()
                        || numeric_u64(&source, "uncredited_excess")? != 0
                    {
                        // A terminal line without its real PREY receipt is
                        // corruption, never permission to credit it again.
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let quantity = numeric_u64(&source, "quantity")?;
                    let source_binding: [u8; 32] = source
                        .try_get::<Vec<u8>, _>("request_binding")?
                        .try_into()
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if quantity == 0 || i64::try_from(quantity).is_err() {
                        return Err(DurabilityError::InvalidStoredState);
                    }

                    let balance = lock_wildcard_balance(&mut tx, account_id, world_id).await?;
                    let credited =
                        u8::try_from(quantity.min(u64::from(WILDCARDS_MAX - balance.value)))
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let after = balance.value + credited;
                    let excess = quantity - u64::from(credited);
                    let committed_revision = root
                        .revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let occurred_at: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let entry_id = if credited == 0 {
                        None
                    } else {
                        let text: String =
                            sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                                .fetch_one(&mut *tx)
                                .await?;
                        Some(uuid_text(&text)?)
                    };
                    let event = entry_id
                        .map(|entry_id| {
                            prey_audit::encode_daily_wildcard_event(
                                WildcardEventIdentity {
                                    event_id: request.event_id,
                                    transaction_id: request.delivery_transaction_id,
                                    world_id,
                                    channel_id,
                                    scope_ownership_generation: fence
                                        .scope_ownership_generation
                                        .get(),
                                    occurred_at,
                                    server_build_id: &request.server_build_id,
                                },
                                DailyWildcardEvent {
                                    claim_transaction_id: request.claim_transaction_id,
                                    line: request.line,
                                    character_id: fence.character_id,
                                    account_id,
                                    entry_id,
                                    quantity,
                                    credited,
                                    uncredited_excess: excess,
                                    balance_before: balance.value,
                                    balance_after: after,
                                    original_character_revision: root.revision,
                                    committed_character_revision: committed_revision,
                                },
                            )
                        })
                        .transpose()
                        .map_err(|_| DurabilityError::InvalidStoredState)?;

                    advance_revision(
                        &mut tx,
                        fence.character_id,
                        root.revision,
                        committed_revision,
                    )
                    .await?;
                    insert_receipt(
                        &mut tx,
                        &request,
                        &binding,
                        &source_binding,
                        &fence,
                        account_id,
                        world_id,
                        &state,
                        quantity,
                        credited,
                        excess,
                        balance,
                        after,
                        entry_id,
                        event.as_ref(),
                        occurred_at,
                        committed_revision,
                    )
                    .await?;
                    if let Some(entry_id) = entry_id {
                        insert_wildcard_entry(
                            &mut tx, &request, &fence, account_id, world_id, balance, entry_id,
                            credited, after,
                        )
                        .await?;
                    }
                    if let Some(event) = event {
                        prey_audit::insert_event(&mut tx, &event).await?;
                    }
                    let terminal = sqlx::query(
                        "UPDATE game_character_daily_reward_lines l \
                         SET status='DELIVERED',delivering_transaction_id=encode($3,'hex')::uuid,\
                         uncredited_excess=$4 \
                         FROM game_character_daily_reward_claims c \
                         WHERE c.transaction_id=encode($1,'hex')::uuid AND l.line=$2 \
                           AND (l.character_id,l.world_id,l.reset_epoch)=\
                               (c.character_id,c.world_id,c.reset_epoch) \
                           AND l.status='PENDING'",
                    )
                    .bind(request.claim_transaction_id.as_slice())
                    .bind(i64::from(request.line))
                    .bind(request.delivery_transaction_id.as_slice())
                    .bind(i64::try_from(excess).map_err(|_| DurabilityError::InvalidStoredState)?)
                    .execute(&mut *tx)
                    .await?;
                    if terminal.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let receipt = CommittedDailyWildcardDelivery {
                        claim_transaction_id: request.claim_transaction_id,
                        line: request.line,
                        delivery_transaction_id: request.delivery_transaction_id,
                        character_id: fence.character_id,
                        account_id,
                        world_id,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        quantity,
                        credited,
                        uncredited_excess: excess,
                        balance_before: balance.value,
                        balance_after: after,
                        entry_id,
                        event_id: event_id_for_credit(&request, credited),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(DailyWildcardOutcome::Committed(receipt)))
                })
            })
            .await?
    }
}

fn valid_uuid7(id: &[u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] & 0xc0 == 0x80
}

fn validate_request(
    fence: &CurrentCharacterGameplayFence,
    request: &PendingDailyWildcardRequest,
) -> Result<()> {
    if !valid_uuid7(&request.claim_transaction_id)
        || !valid_uuid7(&request.delivery_transaction_id)
        || !valid_uuid7(&request.event_id)
        || request.delivery_transaction_id == request.event_id
        || request.delivery_transaction_id == request.claim_transaction_id
        || i32::try_from(request.line).is_err()
        || super::item_mint_audit::check_technical_text(&request.server_build_id).is_err()
        || fence.character_lease_generation == 0
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
    {
        return Err(PreyWildcardError::InvalidInput);
    }
    Ok(())
}

fn request_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &PendingDailyWildcardRequest,
) -> Vec<u8> {
    let mut digest = Sha256::new();
    digest.update(b"oteryn.prey.daily-wildcard");
    digest.update([BINDING_VERSION]);
    digest.update(request.claim_transaction_id);
    digest.update(request.line.to_be_bytes());
    digest.update(fence.character_id.as_bytes());
    digest.update(fence.runtime_scope.world_id().as_bytes());
    // Revision/session generations are independently checked for new writes.
    // Planned delivery/event UUIDs do not change the accepted (claim,line) effect.
    digest.finalize().to_vec()
}

fn fence_error(error: CharacterProgressionError) -> PreyWildcardError {
    match error {
        CharacterProgressionError::CharacterRevisionMismatch => {
            PreyWildcardError::CharacterRevisionMismatch
        }
        CharacterProgressionError::ProgressionContextMismatch => {
            PreyWildcardError::ProgressionContextMismatch
        }
        CharacterProgressionError::Unavailable(error) => PreyWildcardError::Unavailable(error),
        _ => PreyWildcardError::AuthorityRejected,
    }
}

fn event_id_for_credit(request: &PendingDailyWildcardRequest, credited: u8) -> Option<[u8; 16]> {
    (credited > 0).then_some(request.event_id)
}

#[derive(Clone, Copy)]
struct LockedWildcardBalance {
    value: u8,
    last_entry_id: Option<[u8; 16]>,
}

async fn lock_wildcard_balance(
    tx: &mut Transaction<'_, Postgres>,
    account: AccountId,
    world: WorldId,
) -> std::result::Result<LockedWildcardBalance, DurabilityError> {
    sqlx::query("INSERT INTO game_account_prey_wildcards(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0,NULL) ON CONFLICT(account_id,world_id) DO NOTHING")
        .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .execute(&mut **tx).await?;
    let row = sqlx::query("SELECT balance,last_entry_id::text FROM game_account_prey_wildcards WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .fetch_one(&mut **tx).await?;
    let value = u8::try_from(row.try_get::<i16, _>("balance")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if value > WILDCARDS_MAX {
        return Err(DurabilityError::InvalidStoredState);
    }
    let previous: Option<String> = row.try_get("last_entry_id")?;
    Ok(LockedWildcardBalance {
        value,
        last_entry_id: previous.as_deref().map(uuid_text).transpose()?,
    })
}

async fn advance_revision(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    before: u64,
    after: u64,
) -> std::result::Result<(), DurabilityError> {
    for statement in [
        "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
    ] {
        let changed = sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(character.as_bytes().as_slice())
            .bind(after.to_string())
            .bind(before.to_string())
            .execute(&mut **tx)
            .await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_receipt(
    connection: &mut PgConnection,
    request: &PendingDailyWildcardRequest,
    binding: &[u8],
    source_binding: &[u8; 32],
    fence: &CurrentCharacterGameplayFence,
    account: AccountId,
    world: WorldId,
    state: &sqlx::postgres::PgRow,
    quantity: u64,
    credited: u8,
    excess: u64,
    balance: LockedWildcardBalance,
    after: u8,
    entry_id: Option<[u8; 16]>,
    event: Option<&prey_audit::EncodedWildcardEvent>,
    committed_at: i64,
    committed_revision: u64,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_character_prey_receipts(\
         prey_occurrence_id,command_binding,character_id,account_id,world_id,\
         original_character_revision,committed_character_revision,level_before,level_after,\
         experience_before,experience_after,kind,daily_claim_transaction_id,daily_line,\
         source_request_binding,quantity,credited,uncredited_excess,balance_before,balance_after,\
         balance_entry_before,wildcard_entry_id,event_id,envelope_sha256,profile_revision,ruleset_revision,content_revision,\
         simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) \
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,\
         encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,$8,$8,$9,$9,'DAILY_REWARD',\
         encode($10,'hex')::uuid,$11,$12,$13,$14,$15,$16,$17,encode($18,'hex')::uuid,\
         encode($19,'hex')::uuid,encode($20,'hex')::uuid,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30)",
    )
    .bind(request.delivery_transaction_id.as_slice()).bind(binding)
    .bind(fence.character_id.as_bytes().as_slice()).bind(account.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice()).bind(fence.expected_character_revision.get().to_string())
    .bind(committed_revision.to_string()).bind(state.try_get::<i64,_>("level")?)
    .bind(state.try_get::<i64,_>("total_experience")?)
    .bind(request.claim_transaction_id.as_slice()).bind(i64::from(request.line)).bind(source_binding.as_slice())
    .bind(i64::try_from(quantity).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(i16::from(credited)).bind(i64::try_from(excess).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(i16::from(balance.value)).bind(i16::from(after))
    .bind(balance.last_entry_id.map(|id|id.to_vec())).bind(entry_id.map(|id|id.to_vec()))
    .bind(event_id_for_credit(request,credited).map(|id|id.to_vec()))
    .bind(event.map(|e|e.envelope_sha256.to_vec()))
    .bind(state.try_get::<String,_>("profile_revision")?)
    .bind(state.try_get::<String,_>("ruleset_revision")?)
    .bind(state.try_get::<String,_>("content_revision")?)
    .bind(state.try_get::<String,_>("simulation_revision")?)
    .bind(state.try_get::<String,_>("evidence_revision")?)
    .bind(state.try_get::<String,_>("declaration_revision")?)
    .bind(state.try_get::<String,_>("policy_revision")?)
    .bind(state.try_get::<String,_>("reward_revision")?)
    .bind(committed_at).execute(connection).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_wildcard_entry(
    connection: &mut PgConnection,
    request: &PendingDailyWildcardRequest,
    fence: &CurrentCharacterGameplayFence,
    account: AccountId,
    world: WorldId,
    balance: LockedWildcardBalance,
    entry_id: [u8; 16],
    amount: u8,
    after: u8,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_account_prey_wildcard_entries(entry_id,prey_occurrence_id,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,'DAILY_REWARD',$5,$6,$7,encode($8,'hex')::uuid,encode($9,'hex')::uuid)")
        .bind(entry_id.as_slice()).bind(request.delivery_transaction_id.as_slice())
        .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .bind(i16::from(amount)).bind(i16::from(balance.value)).bind(i16::from(after))
        .bind(balance.last_entry_id.map(|id|id.to_vec())).bind(fence.character_id.as_bytes().as_slice())
        .execute(&mut *connection).await?;
    let changed=sqlx::query("UPDATE game_account_prey_wildcards SET balance=$3,last_entry_id=encode($4,'hex')::uuid WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND balance=$5 AND last_entry_id IS NOT DISTINCT FROM encode($6,'hex')::uuid")
        .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .bind(i16::from(after)).bind(entry_id.as_slice()).bind(i16::from(balance.value))
        .bind(balance.last_entry_id.map(|id|id.to_vec())).execute(connection).await?;
    if changed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn load_receipt(
    tx: &mut Transaction<'_, Postgres>,
    request: &PendingDailyWildcardRequest,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query("SELECT prey_occurrence_id::text,command_binding,character_id::text,account_id::text,world_id::text,original_character_revision::text,committed_character_revision::text,daily_claim_transaction_id::text,daily_line,quantity::text,credited,uncredited_excess::text,balance_before,balance_after,wildcard_entry_id::text,event_id::text FROM game_character_prey_receipts WHERE daily_claim_transaction_id=encode($1,'hex')::uuid AND daily_line=$2")
        .bind(request.claim_transaction_id.as_slice()).bind(i64::from(request.line))
        .fetch_optional(&mut **tx).await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedDailyWildcardDelivery, DurabilityError> {
    let small = |name| {
        u8::try_from(row.try_get::<i16, _>(name)?).map_err(|_| DurabilityError::InvalidStoredState)
    };
    let credited = small("credited")?;
    let before = small("balance_before")?;
    let after = small("balance_after")?;
    let quantity = numeric_u64(row, "quantity")?;
    let excess = numeric_u64(row, "uncredited_excess")?;
    let entry: Option<String> = row.try_get("wildcard_entry_id")?;
    let event: Option<String> = row.try_get("event_id")?;
    let original = numeric_u64(row, "original_character_revision")?;
    let committed = numeric_u64(row, "committed_character_revision")?;
    if before > WILDCARDS_MAX
        || after > WILDCARDS_MAX
        || after.checked_sub(before) != Some(credited)
        || quantity == 0
        || quantity.checked_sub(u64::from(credited)) != Some(excess)
        || quantity.min(u64::from(WILDCARDS_MAX - before)) != u64::from(credited)
        || original == 0
        || original.checked_add(1) != Some(committed)
        || (credited > 0) != (entry.is_some() && event.is_some())
        || (credited == 0) && (entry.is_some() || event.is_some())
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(CommittedDailyWildcardDelivery {
        claim_transaction_id: uuid_text(&row.try_get::<String, _>("daily_claim_transaction_id")?)?,
        line: u32::try_from(row.try_get::<i32, _>("daily_line")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        delivery_transaction_id: uuid_text(&row.try_get::<String, _>("prey_occurrence_id")?)?,
        character_id: CharacterId::from_bytes(uuid_text(
            &row.try_get::<String, _>("character_id")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        account_id: AccountId::from_bytes(uuid_text(&row.try_get::<String, _>("account_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        world_id: WorldId::decode(&uuid_text(&row.try_get::<String, _>("world_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_character_revision: CharacterRevision::new(original)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        committed_character_revision: CharacterRevision::new(committed)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        quantity,
        credited,
        uncredited_excess: excess,
        balance_before: before,
        balance_after: after,
        entry_id: entry.as_deref().map(uuid_text).transpose()?,
        event_id: event.as_deref().map(uuid_text).transpose()?,
    })
}

#[cfg(test)]
#[path = "prey_tests.rs"]
mod tests;
