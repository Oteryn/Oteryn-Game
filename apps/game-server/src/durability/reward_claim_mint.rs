//! DUR-03 reward-claim MINT into the equipped main backpack (reward chest
//! decisions D40-D42 and §5.1 D92; child `CHEST-1`).
//!
//! A player USE of a reward chest mints exactly one fresh live item directly
//! into a new entry of the character's equipped main backpack and records the
//! character's `once` RewardClaim, in one DUR-03 transaction with its receipt
//! and audit event. This component owns durable admission, application,
//! idempotency and reconciliation. It does not prove that the USE was
//! legitimately ingested or which placement was used (GAME-INTERACTION, D39)
//! nor resolve the claim and item facts from Content: the caller supplies
//! them from the current compatible Content.
//!
//! First-slice scope (§5.1): `once` claims only (`next_allowed_at` stays
//! NULL), one top-level reward item per claim, never a container (DUR03-RL-05
//! = 0), never into an existing stack (DUR-03 §39.1) and never onto Ground.
//! Room (D92): the main backpack needs a free direct entry,
//! `current_entry_count < definition_capacity`; weight is not checked here
//! (a declared delivery gap closed with B3-3). A claim without room is
//! refused with nothing written, and the player can make room and retry.
//!
//! Lifecycle of one logical reward-claim MINT, keyed by its full CommandRef:
//! 1. [`DurabilityRoot::freeze_reward_claim_mint`] validates the intent,
//!    checks the complete current fence and that the claim is admissible now,
//!    then reserves TransactionId, EventId, ItemInstanceId, the trusted
//!    timestamp and a zero DUR03-RL-08 budget in its own transaction. A
//!    refusal writes nothing.
//! 2. [`DurabilityRoot::commit_reward_claim_mint`] re-checks the fence,
//!    rereads the authoritative before-state under the `character_root` row
//!    lock, materializes the placement and the exact event bytes, and commits
//!    the item, its backpack entry, the RewardClaim, the receipt and the audit
//!    event together.
//! 3. [`DurabilityRoot::reconcile_reward_claim_mint`] reads the receipt after
//!    an unknown outcome under the current recovery fence.
//!
//! D40 idempotency: the RewardClaim row is unique per (character, claim), so
//! a claim never mints a second item. The same CommandRef returns its first
//! outcome; another command on an already claimed `once` claim is refused
//! with nothing written. The fence is the B3-1 TRANSFER fence
//! ([`super::item_transfer::character_item_fence_is_current`]);
//! `CharacterRevision` is never read as a fence nor written.
//!
//! Achievement (Achievement owner contract §3 and §5 step 4; chest decisions
//! §6): a chest's `placement.achievement` rides on the request with the
//! caller's resolution against the world's current catalogue, and is part of
//! the intent binding. The commit transaction records the grant request and
//! consumes it into the account's fact after the fence and the
//! `character_root` row lock, together with the claim. The source event is
//! the claim itself, (Character, claim family, claim production key), so a
//! replay can never request it twice. A key the catalogue lacks is refused
//! before anything is reserved; a retired key grants nothing and the claim
//! commits.

use super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantError, AchievementGrantRequest,
    AchievementSourceEvent, FencedGrantingCharacter, record_achievement_grant, valid_key,
};
use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::character_progression::valid_revision;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint::{TypedDefinitionRef, uuid_text};
use super::item_mint_audit::{
    self as mint_audit, AuditError, ITEM_LIFECYCLE_LIVE, OneItemStateV1, RL08_RETRY_WORK_UNITS_MAX,
};
use super::item_transfer::{
    BackpackEntry, ContainerEntryPosition, CurrentCharacterItemFence,
    GAMEITEM01_CONTAINER_ENTRIES_MAX, InventoryItem, ItemDefinitionFacts, ItemTransferError,
    ItemTransferRefusal, character_item_fence_is_current, definition_message, entry_message,
    load_entries, load_slot, push_facts, push_text, scope_of, stack_maximum, validate_facts,
};
use super::item_transfer_audit::{OneItemCommandRefV1, OneItemInventoryV1};
use super::reward_claim_mint_audit::{
    self as audit, OneItemRewardClaimCauseV1, OneItemRewardClaimMintV1,
    REWARD_CLAIM_MINT_TYPED_CAUSE, RewardClaimMintEventIdentity,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::CharacterId;
use crate::foundation::CommandRef;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, RewardClaimMintError>;
type Pass<T> = std::result::Result<std::result::Result<T, RewardClaimMintError>, DurabilityError>;
const INTENT_BINDING_VERSION: u8 = 1;
const EVENT_TYPE_ID: i64 = mint_audit::EVENT_TYPE_ID as i64;
const EVENT_SCHEMA_REVISION: i64 = mint_audit::EVENT_SCHEMA_REVISION as i64;
/// `source_kind` of the achievement grant requests a reward claim records.
pub const ACHIEVEMENT_SOURCE_KIND: &str = "oteryn:reward-claim";

/// The chest's achievement and the caller's lookup of its key in the world's
/// current catalogue (no runtime catalogue loader exists).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardClaimAchievement {
    pub key: String,
    pub catalogue: AchievementCatalogueLookup,
}

/// Complete semantic intent of one reward-claim MINT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardClaimMintRequest {
    /// The actual FND-02 CommandRef of the player's USE.
    pub command: CommandRef,
    /// The claimed RewardClaim definition. Uniqueness is per (character,
    /// family, production key): a later revision never re-opens a claim.
    pub claim: TypedDefinitionRef,
    /// Facts of the one top-level reward item's definition.
    pub item: ItemDefinitionFacts,
    pub quantity: u32,
    /// Facts of the equipped main backpack's definition.
    pub backpack: ItemDefinitionFacts,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
    /// The chest's achievement, granted with the claim; `None` grants none.
    pub achievement: Option<RewardClaimAchievement>,
}

/// Refusal reasons. A refusal writes nothing and the player can retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardClaimRefusal {
    /// The character already holds this `once` claim.
    AlreadyClaimed,
    /// A container reward waits for the nested-bags decision (RL-05 = 0).
    RewardIsContainer,
    /// Unknown stack class (D82: fail closed).
    UnknownStackClass,
    /// A proven stack maximum of 0 or above GAMEITEM01-STACK-QUANTITY-MAX.
    UnsupportedStackMaximum,
    /// The reward quantity exceeds the definition's stack maximum.
    QuantityAboveStackMaximum,
    /// No main backpack is equipped.
    NoMainBackpack,
    /// The supplied backpack facts do not describe the equipped backpack.
    DefinitionMismatch,
    /// A main backpack capacity of 0 or above GAMEITEM01-CONTAINER-ENTRIES-MAX.
    UnsupportedContainerCapacity,
    /// No free direct entry in the main backpack (D92).
    MainBackpackFull,
}

/// Terminal committed result of one CommandRef.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedRewardClaimMint {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub item_instance_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
    pub quantity: u32,
    pub destination: ContainerEntryPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewardClaimMintOutcome {
    Committed(CommittedRewardClaimMint),
    /// The CommandRef already claimed; this is its original result.
    AlreadyCommitted(CommittedRewardClaimMint),
}

#[derive(Debug)]
pub enum RewardClaimMintError {
    /// Malformed or oversize input; never truncated.
    InvalidInput,
    /// A registered DUR-03 ceiling (including RL-08) was exceeded.
    CapacityExceeded,
    /// A current recovery, session, lease, scope, node or binding fence
    /// rejected the claim.
    AuthorityRejected,
    /// The claim is not admissible in the current state.
    Refused(RewardClaimRefusal),
    /// The CommandRef was already used with a different intent.
    ConflictingCause,
    /// A frozen identity is bound to a different cause.
    ConflictingCandidate,
    /// The world's catalogue lacks the chest's achievement: nothing is
    /// reserved, minted, claimed or granted.
    UnknownAchievement,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for RewardClaimMintError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<AuditError> for RewardClaimMintError {
    fn from(error: AuditError) -> Self {
        match error {
            AuditError::InvalidInput => Self::InvalidInput,
            AuditError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

impl From<AchievementGrantError> for RewardClaimMintError {
    fn from(error: AchievementGrantError) -> Self {
        match error {
            AchievementGrantError::InvalidInput => Self::InvalidInput,
            AchievementGrantError::AuthorityRejected => Self::AuthorityRejected,
            AchievementGrantError::UnknownAchievement => Self::UnknownAchievement,
            AchievementGrantError::ConflictingSourceEvent => Self::ConflictingCandidate,
            AchievementGrantError::Unavailable(error) => Self::Unavailable(error),
        }
    }
}

/// Shared B3-1 helpers report input errors as TRANSFER errors.
fn from_transfer_input(error: ItemTransferError) -> RewardClaimMintError {
    match error {
        ItemTransferError::CapacityExceeded => RewardClaimMintError::CapacityExceeded,
        _ => RewardClaimMintError::InvalidInput,
    }
}

impl std::fmt::Display for RewardClaimMintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid reward-claim MINT input"),
            Self::CapacityExceeded => formatter.write_str("reward-claim MINT capacity exceeded"),
            Self::AuthorityRejected => formatter.write_str("reward-claim MINT authority rejected"),
            Self::Refused(reason) => write!(formatter, "reward-claim MINT refused: {reason:?}"),
            Self::ConflictingCause => {
                formatter.write_str("reward-claim command was reused with different semantics")
            }
            Self::ConflictingCandidate => {
                formatter.write_str("reward-claim identity is bound to a different command")
            }
            Self::UnknownAchievement => {
                formatter.write_str("reward-claim achievement is not in the world catalogue")
            }
            Self::Unavailable(error) => {
                write!(
                    formatter,
                    "reward-claim MINT storage is unavailable: {error:?}"
                )
            }
        }
    }
}

impl std::error::Error for RewardClaimMintError {}

/// Frozen candidate of one logical reward-claim MINT: a process-local view of
/// its durable reservation. Deliberately not `Clone`.
#[derive(Debug)]
pub struct RewardClaimMintCandidate {
    request: RewardClaimMintRequest,
    character_id: CharacterId,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    intent_binding: [u8; 33],
    work_units_used: u8,
}

impl RewardClaimMintCandidate {
    #[must_use]
    pub const fn transaction_id(&self) -> &[u8; 16] {
        &self.transaction_id
    }
    #[must_use]
    pub const fn event_id(&self) -> &[u8; 16] {
        &self.event_id
    }
    #[must_use]
    pub const fn item_instance_id(&self) -> &[u8; 16] {
        &self.item_instance_id
    }
    #[must_use]
    pub const fn occurred_at_unix_ms(&self) -> i64 {
        self.occurred_at_unix_ms
    }
    #[must_use]
    pub const fn work_units_used(&self) -> u8 {
        self.work_units_used
    }
}

/// Authoritative before-state read under the `character_root` row lock.
pub(crate) struct RewardPlanInput<'a> {
    pub item: &'a ItemDefinitionFacts,
    pub quantity: u32,
    pub backpack: &'a ItemDefinitionFacts,
    pub already_claimed: bool,
    pub slot: Option<&'a InventoryItem>,
    pub entries: &'a [BackpackEntry],
}

fn from_stack_refusal(refusal: ItemTransferRefusal) -> RewardClaimRefusal {
    match refusal {
        ItemTransferRefusal::UnknownStackClass => RewardClaimRefusal::UnknownStackClass,
        _ => RewardClaimRefusal::UnsupportedStackMaximum,
    }
}

/// Pure D92 admission and placement: the new entry takes the highest
/// existing ordinal plus one and nothing is renumbered.
pub(crate) fn plan_reward_claim_mint(
    input: &RewardPlanInput<'_>,
) -> std::result::Result<ContainerEntryPosition, RewardClaimRefusal> {
    use RewardClaimRefusal as Refusal;
    if input.already_claimed {
        return Err(Refusal::AlreadyClaimed);
    }
    if input.item.container_capacity.is_some() {
        return Err(Refusal::RewardIsContainer);
    }
    let (maximum, _) = stack_maximum(input.item.stack).map_err(from_stack_refusal)?;
    if input.quantity == 0 || input.quantity > maximum {
        return Err(Refusal::QuantityAboveStackMaximum);
    }
    let slot = input.slot.ok_or(Refusal::NoMainBackpack)?;
    if input.backpack.definition != slot.definition {
        return Err(Refusal::DefinitionMismatch);
    }
    let capacity = input
        .backpack
        .container_capacity
        .ok_or(Refusal::DefinitionMismatch)?;
    if capacity == 0 || capacity > GAMEITEM01_CONTAINER_ENTRIES_MAX {
        return Err(Refusal::UnsupportedContainerCapacity);
    }
    let count = u32::try_from(input.entries.len()).unwrap_or(u32::MAX);
    if count >= capacity {
        return Err(Refusal::MainBackpackFull);
    }
    let next = input
        .entries
        .iter()
        .map(|entry| entry.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(Refusal::MainBackpackFull)?;
    Ok(ContainerEntryPosition {
        parent_item_instance_id: slot.item_instance_id,
        placement_ordinal: next,
    })
}

/// Owned copy of the frozen candidate moved into one database pass.
struct FrozenClaim {
    request: RewardClaimMintRequest,
    character_id: CharacterId,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    intent_binding: [u8; 33],
}

impl FrozenClaim {
    fn of(candidate: &RewardClaimMintCandidate) -> Self {
        Self {
            request: candidate.request.clone(),
            character_id: candidate.character_id,
            transaction_id: candidate.transaction_id,
            event_id: candidate.event_id,
            item_instance_id: candidate.item_instance_id,
            occurred_at_unix_ms: candidate.occurred_at_unix_ms,
            intent_binding: candidate.intent_binding,
        }
    }
}

/// The durable reservation row of one CommandRef.
struct Reservation {
    character_id: CharacterId,
    world_id: [u8; 16],
    channel_id: [u8; 16],
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    work_units_used: u8,
}

impl DurabilityRoot {
    /// Validate the intent and reserve the logical reward-claim MINT of this
    /// CommandRef, or resume its reservation with the same identities and
    /// budget. A new reservation, and the resumption of one without a
    /// receipt, require the complete current fence and a claim admissible
    /// now; a refusal writes nothing. This grants nothing and mints nothing.
    pub async fn freeze_reward_claim_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: RewardClaimMintRequest,
    ) -> Result<RewardClaimMintCandidate> {
        validate_request(&request)?;
        let intent_binding = intent_binding(&request, fence.character_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        let (request, reservation) = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, request.command).await?;
                    if let Some(row) = load_reservation(&mut tx, request.command).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != intent_binding {
                            return Ok(Err(RewardClaimMintError::ConflictingCause));
                        }
                        let reservation = decode_reservation(&row)?;
                        if load_receipt(&mut tx, request.command).await?.is_none()
                            && let Err(error) =
                                admit(&mut tx, &node, &fence, &request, &reservation).await?
                        {
                            return Ok(Err(error));
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok((request, reservation)));
                    }
                    let (world_id, channel_id) = match scope_of(&fence) {
                        Ok(scope) => scope,
                        Err(_) => return Ok(Err(RewardClaimMintError::AuthorityRejected)),
                    };
                    let row = sqlx::query(
                        "SELECT game_character_uuid_v7()::text AS transaction_id, \
                                game_character_uuid_v7()::text AS event_id, \
                                game_character_uuid_v7()::text AS item_instance_id, \
                                floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                                  AS occurred_at",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let reservation = Reservation {
                        character_id: fence.character_id,
                        world_id: *world_id.as_bytes(),
                        channel_id: *channel_id.as_bytes(),
                        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
                        event_id: uuid_text(row.try_get("event_id")?)?,
                        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                        occurred_at_unix_ms: row.try_get("occurred_at")?,
                        work_units_used: 0,
                    };
                    if let Err(error) =
                        admit(&mut tx, &node, &fence, &request, &reservation).await?
                    {
                        return Ok(Err(error));
                    }
                    insert_reservation(&mut tx, &request, &intent_binding, &reservation).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok((request, reservation)))
                })
            })
            .await??;
        Ok(RewardClaimMintCandidate {
            request,
            character_id: reservation.character_id,
            transaction_id: reservation.transaction_id,
            event_id: reservation.event_id,
            item_instance_id: reservation.item_instance_id,
            occurred_at_unix_ms: reservation.occurred_at_unix_ms,
            intent_binding,
            work_units_used: reservation.work_units_used,
        })
    }

    /// Durably charge one DUR03-RL-08 work unit to the reservation in its own
    /// committed transaction before the pass does any work.
    async fn charge_reward_claim_mint_work_unit(
        &self,
        recovery: CharacterRecoveryFenceV1,
        candidate: &mut RewardClaimMintCandidate,
    ) -> Result<()> {
        if candidate.work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
            return Err(RewardClaimMintError::CapacityExceeded);
        }
        let command = candidate.request.command;
        let transaction_id = candidate.transaction_id;
        let charged = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let used = sqlx::query(
                        "UPDATE game_reward_claim_mint_reservations \
                            SET work_units_used = work_units_used + 1 \
                          WHERE game_session_id = encode($1,'hex')::uuid \
                            AND command_id = $2::text::numeric(20,0) \
                            AND transaction_id = encode($3,'hex')::uuid \
                            AND work_units_used < $4 RETURNING work_units_used",
                    )
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .bind(transaction_id.as_slice())
                    .bind(i16::from(RL08_RETRY_WORK_UNITS_MAX))
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(used) = used else {
                        let reserved = sqlx::query(
                            "SELECT 1 FROM game_reward_claim_mint_reservations \
                              WHERE game_session_id = encode($1,'hex')::uuid \
                                AND command_id = $2::text::numeric(20,0) \
                                AND transaction_id = encode($3,'hex')::uuid",
                        )
                        .bind(command.game_session_id().as_bytes().as_slice())
                        .bind(command.command_id().get().to_string())
                        .bind(transaction_id.as_slice())
                        .fetch_optional(&mut *tx)
                        .await?;
                        return Ok(Err(if reserved.is_some() {
                            RewardClaimMintError::CapacityExceeded
                        } else {
                            RewardClaimMintError::ConflictingCandidate
                        }));
                    };
                    let used = u8::try_from(used.try_get::<i16, _>("work_units_used")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(used))
                })
            })
            .await?;
        match charged {
            Ok(used) => {
                candidate.work_units_used = used;
                Ok(())
            }
            Err(RewardClaimMintError::CapacityExceeded) => {
                candidate.work_units_used = RL08_RETRY_WORK_UNITS_MAX;
                Err(RewardClaimMintError::CapacityExceeded)
            }
            Err(error) => Err(error),
        }
    }

    /// Commit one frozen reward-claim MINT. A CommandRef that already claimed
    /// returns its original result under the current recovery fence without
    /// reacquiring session authority; a changed intent conflicts. A new claim
    /// commits only under the complete current fence and with the
    /// authoritative before-state admitting it.
    pub async fn commit_reward_claim_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        candidate: &mut RewardClaimMintCandidate,
    ) -> Result<RewardClaimMintOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        self.charge_reward_claim_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let frozen = FrozenClaim::of(candidate);
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let command = frozen.request.command;
                    lock_cause(&mut tx, command).await?;

                    if let Some(row) = load_receipt(&mut tx, command).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != frozen.intent_binding {
                            return Ok(Err(RewardClaimMintError::ConflictingCause));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(RewardClaimMintOutcome::AlreadyCommitted(committed)));
                    }

                    let Some(row) = load_reservation(&mut tx, command).await? else {
                        return Ok(Err(RewardClaimMintError::ConflictingCandidate));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != frozen.intent_binding {
                        return Ok(Err(RewardClaimMintError::ConflictingCause));
                    }
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != frozen.transaction_id
                        || reservation.event_id != frozen.event_id
                        || reservation.item_instance_id != frozen.item_instance_id
                        || reservation.occurred_at_unix_ms != frozen.occurred_at_unix_ms
                        || reservation.character_id != frozen.character_id
                    {
                        return Ok(Err(RewardClaimMintError::ConflictingCandidate));
                    }
                    let identity_reused: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_reward_claim_mint_receipts \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid \
                                            OR item_instance_id = encode($3,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_audit_outbox \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_instances \
                                         WHERE item_instance_id = encode($3,'hex')::uuid \
                                            OR minted_transaction_id = encode($1,'hex')::uuid)",
                    )
                    .bind(frozen.transaction_id.as_slice())
                    .bind(frozen.event_id.as_slice())
                    .bind(frozen.item_instance_id.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if identity_reused {
                        return Ok(Err(RewardClaimMintError::ConflictingCandidate));
                    }

                    let (destination, granter) =
                        match admit(&mut tx, &node, &fence, &frozen.request, &reservation).await? {
                            Ok(admitted) => admitted,
                            Err(error) => return Ok(Err(error)),
                        };
                    let message = claim_message(&frozen, &reservation, &fence, destination);
                    let envelope = match audit::encode_reward_claim_mint_event(
                        RewardClaimMintEventIdentity {
                            event_id: frozen.event_id,
                            transaction_id: frozen.transaction_id,
                            occurred_at_unix_ms: frozen.occurred_at_unix_ms,
                            channel_id: reservation.channel_id,
                            server_build_id: SERVER_BUILD_ID,
                        },
                        message,
                    ) {
                        Ok(envelope) => envelope,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    // With the token `admit` minted after the fence and the
                    // `character_root` row lock in this transaction; an error
                    // drops the whole transaction.
                    if let Some(achievement) = &frozen.request.achievement {
                        let grant = match achievement_grant(
                            achievement,
                            fence.character_id,
                            &frozen.request.claim,
                        ) {
                            Ok(grant) => grant,
                            Err(error) => return Ok(Err(error)),
                        };
                        if let Err(error) =
                            record_achievement_grant(&mut tx, granter, &grant).await?
                        {
                            return Ok(Err(error.into()));
                        }
                    }
                    let committed =
                        apply_claim(&mut tx, &frozen, &reservation, destination, &envelope).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(RewardClaimMintOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an unknown outcome. The cause lock waits for any in-flight
    /// attempt, so `None` proves nothing committed for this CommandRef and the
    /// same candidate may be retried. Never reacquires session authority.
    pub async fn reconcile_reward_claim_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &mut RewardClaimMintCandidate,
    ) -> Result<Option<CommittedRewardClaimMint>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        self.charge_reward_claim_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let command = candidate.request.command;
        let binding = candidate.intent_binding;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, command).await?;
                    let Some(row) = load_receipt(&mut tx, command).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != binding {
                        return Ok(Err(RewardClaimMintError::ConflictingCause));
                    }
                    let committed = decode_receipt(&row)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }
}

/// Witness that [`admit`] found the complete current item fence in its
/// transaction. Its private field keeps construction in this module, so
/// [`FencedGrantingCharacter::after_fence`] is reachable only from `admit`.
pub(super) struct RewardClaimFenceChecked(());

/// The complete current fence (shared with B3-1 TRANSFER, ending with the
/// `character_root` row lock), the claim state, the authoritative backpack
/// before-state and the D92 placement. Writes no row. Returns the
/// destination and the achievement grant token of this transaction (reading
/// its id assigns the transaction one).
async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &RewardClaimMintRequest,
    reservation: &Reservation,
) -> Pass<(ContainerEntryPosition, FencedGrantingCharacter)> {
    if !character_item_fence_is_current(
        tx,
        node,
        fence,
        request.command,
        reservation.character_id,
        reservation.world_id,
        reservation.channel_id,
    )
    .await?
    {
        return Ok(Err(RewardClaimMintError::AuthorityRejected));
    }
    let granter =
        FencedGrantingCharacter::after_fence(tx, fence.character_id, RewardClaimFenceChecked(()))
            .await?;
    let already_claimed: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_reward_claims \
                         WHERE character_id = encode($1,'hex')::uuid \
                           AND claim_family = $2 AND claim_production_key = $3)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&request.claim.family)
    .bind(&request.claim.production_key)
    .fetch_one(&mut **tx)
    .await?;
    let slot = load_slot(tx, fence.character_id).await?;
    let entries = match slot.as_ref() {
        Some(slot) => load_entries(tx, slot.item_instance_id).await?,
        None => Vec::new(),
    };
    let destination = match plan_reward_claim_mint(&RewardPlanInput {
        item: &request.item,
        quantity: request.quantity,
        backpack: &request.backpack,
        already_claimed,
        slot: slot.as_ref(),
        entries: &entries,
    }) {
        Ok(destination) => destination,
        Err(refusal) => return Ok(Err(RewardClaimMintError::Refused(refusal))),
    };
    Ok(Ok((destination, granter)))
}

/// The complete typed after-state evidence of the admitted claim.
fn claim_message(
    frozen: &FrozenClaim,
    reservation: &Reservation,
    fence: &CurrentCharacterItemFence,
    destination: ContainerEntryPosition,
) -> OneItemRewardClaimMintV1 {
    let request = &frozen.request;
    OneItemRewardClaimMintV1 {
        after: Some(OneItemStateV1 {
            item_instance_id: frozen.item_instance_id.to_vec(),
            world_id: reservation.world_id.to_vec(),
            definition: Some(definition_message(&request.item.definition)),
            quantity: request.quantity,
            lifecycle: ITEM_LIFECYCLE_LIVE,
        }),
        destination: Some(OneItemInventoryV1 {
            character_id: fence.character_id.as_bytes().to_vec(),
            expected_session_generation: fence.connection_generation.get(),
            expected_game_session_id: fence.game_session_id.as_bytes().to_vec(),
            expected_character_lease_generation: fence.character_lease_generation,
            container_entry: Some(entry_message(destination)),
            equipment_container_slot: false,
        }),
        source: Some(OneItemRewardClaimCauseV1 {
            typed_cause: REWARD_CLAIM_MINT_TYPED_CAUSE.into(),
            content_revision: request.content_revision.clone(),
            ruleset_revision: request.ruleset_revision.clone(),
            sim_revision: request.sim_revision.clone(),
            command_ref: Some(OneItemCommandRefV1 {
                game_session_id: request.command.game_session_id().as_bytes().to_vec(),
                command_id: request.command.command_id().get(),
            }),
            reward_claim: Some(definition_message(&request.claim)),
        }),
        before_semantically_absent: true,
    }
}

/// Apply the item, its backpack entry, the audit event, the RewardClaim and
/// the receipt; the deferred guards prove they commit together.
async fn apply_claim(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenClaim,
    reservation: &Reservation,
    destination: ContainerEntryPosition,
    envelope: &[u8],
) -> std::result::Result<CommittedRewardClaimMint, DurabilityError> {
    let request = &frozen.request;
    let tx_id = frozen.transaction_id.as_slice();
    let item_id = frozen.item_instance_id.as_slice();
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
           encode($7,'hex')::uuid)",
    )
    .bind(item_id)
    .bind(reservation.world_id.as_slice())
    .bind(&request.item.definition.family)
    .bind(&request.item.definition.production_key)
    .bind(&request.item.definition.revision_ref)
    .bind(i64::from(request.quantity))
    .bind(tx_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
           parent_item_instance_id, placement_ordinal, placed_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, $5::text::numeric(20,0), encode($6,'hex')::uuid)",
    )
    .bind(item_id)
    .bind(reservation.world_id.as_slice())
    .bind(frozen.character_id.as_bytes().as_slice())
    .bind(destination.parent_item_instance_id.as_slice())
    .bind(destination.placement_ordinal.to_string())
    .bind(tx_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(frozen.event_id.as_slice())
    .bind(tx_id)
    .bind(EVENT_TYPE_ID)
    .bind(EVENT_SCHEMA_REVISION)
    .bind(mint_audit::RETENTION_PROFILE_ID)
    .bind(item_id)
    .bind(frozen.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope)
    .execute(&mut **tx)
    .await?;
    let committed_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query(
        "INSERT INTO game_reward_claims(character_id, claim_family, claim_production_key, \
           claim_revision_ref, next_allowed_at, claimed_transaction_id, claimed_at) \
         VALUES (encode($1,'hex')::uuid, $2, $3, $4, NULL, encode($5,'hex')::uuid, $6)",
    )
    .bind(frozen.character_id.as_bytes().as_slice())
    .bind(&request.claim.family)
    .bind(&request.claim.production_key)
    .bind(&request.claim.revision_ref)
    .bind(tx_id)
    .bind(committed_at)
    .execute(&mut **tx)
    .await?;
    let envelope_sha256: [u8; 32] = Sha256::digest(envelope).into();
    let command = request.command;
    sqlx::query(
        "INSERT INTO game_reward_claim_mint_receipts(game_session_id, command_id, \
           character_id, claim_family, claim_production_key, claim_revision_ref, \
           intent_binding, transaction_id, event_id, item_instance_id, quantity, \
           destination_parent_item_instance_id, destination_ordinal, occurred_at, \
           envelope_sha256, committed_at) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), encode($3,'hex')::uuid, \
           $4, $5, $6, $7, encode($8,'hex')::uuid, encode($9,'hex')::uuid, \
           encode($10,'hex')::uuid, $11, encode($12,'hex')::uuid, $13::text::numeric(20,0), \
           $14, $15, $16)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(frozen.character_id.as_bytes().as_slice())
    .bind(&request.claim.family)
    .bind(&request.claim.production_key)
    .bind(&request.claim.revision_ref)
    .bind(frozen.intent_binding.as_slice())
    .bind(tx_id)
    .bind(frozen.event_id.as_slice())
    .bind(item_id)
    .bind(i64::from(request.quantity))
    .bind(destination.parent_item_instance_id.as_slice())
    .bind(destination.placement_ordinal.to_string())
    .bind(frozen.occurred_at_unix_ms)
    .bind(envelope_sha256.as_slice())
    .bind(committed_at)
    .execute(&mut **tx)
    .await?;
    Ok(CommittedRewardClaimMint {
        transaction_id: frozen.transaction_id,
        event_id: frozen.event_id,
        item_instance_id: frozen.item_instance_id,
        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
        envelope_sha256,
        quantity: request.quantity,
        destination,
    })
}

async fn lock_cause(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: CommandRef,
) -> std::result::Result<(), DurabilityError> {
    let mut key = Vec::with_capacity(24);
    key.extend_from_slice(command.game_session_id().as_bytes());
    key.extend_from_slice(&command.command_id().get().to_be_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:reward-claim-mint:' || encode($1, 'hex'), 0))",
    )
    .bind(key.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: CommandRef,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT intent_binding, character_id::text, world_id::text, channel_id::text, \
                transaction_id::text, event_id::text, item_instance_id::text, occurred_at, \
                work_units_used \
           FROM game_reward_claim_mint_reservations \
          WHERE game_session_id = encode($1,'hex')::uuid \
            AND command_id = $2::text::numeric(20,0)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_reservation(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Reservation, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    Ok(Reservation {
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        world_id: uuid_text(row.try_get("world_id")?)?,
        channel_id: uuid_text(row.try_get("channel_id")?)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        work_units_used: u8::try_from(row.try_get::<i16, _>("work_units_used")?)
            .map_err(invalid)?,
    })
}

async fn insert_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &RewardClaimMintRequest,
    intent_binding: &[u8; 33],
    reservation: &Reservation,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_reward_claim_mint_reservations(game_session_id, command_id, \
           character_id, world_id, channel_id, claim_family, claim_production_key, \
           claim_revision_ref, intent_binding, transaction_id, event_id, item_instance_id, \
           occurred_at, work_units_used, reserved_at) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6, $7, $8, $9, \
           encode($10,'hex')::uuid, encode($11,'hex')::uuid, encode($12,'hex')::uuid, $13, 0, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(request.command.game_session_id().as_bytes().as_slice())
    .bind(request.command.command_id().get().to_string())
    .bind(reservation.character_id.as_bytes().as_slice())
    .bind(reservation.world_id.as_slice())
    .bind(reservation.channel_id.as_slice())
    .bind(&request.claim.family)
    .bind(&request.claim.production_key)
    .bind(&request.claim.revision_ref)
    .bind(intent_binding.as_slice())
    .bind(reservation.transaction_id.as_slice())
    .bind(reservation.event_id.as_slice())
    .bind(reservation.item_instance_id.as_slice())
    .bind(reservation.occurred_at_unix_ms)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: CommandRef,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT intent_binding, transaction_id::text, event_id::text, \
                item_instance_id::text, quantity, destination_parent_item_instance_id::text, \
                destination_ordinal::text, occurred_at, envelope_sha256 \
           FROM game_reward_claim_mint_receipts \
          WHERE game_session_id = encode($1,'hex')::uuid \
            AND command_id = $2::text::numeric(20,0)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedRewardClaimMint, DurabilityError> {
    let digest: Vec<u8> = row.try_get("envelope_sha256")?;
    Ok(CommittedRewardClaimMint {
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        envelope_sha256: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        destination: ContainerEntryPosition {
            parent_item_instance_id: uuid_text(
                row.try_get("destination_parent_item_instance_id")?,
            )?,
            placement_ordinal: row
                .try_get::<String, _>("destination_ordinal")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
    })
}

fn validate_request(request: &RewardClaimMintRequest) -> Result<()> {
    mint_audit::check_technical_text(&request.claim.family)?;
    mint_audit::check_content_key(&request.claim.production_key)?;
    mint_audit::check_content_key(&request.claim.revision_ref)?;
    validate_facts(&request.item).map_err(from_transfer_input)?;
    validate_facts(&request.backpack).map_err(from_transfer_input)?;
    if request.quantity == 0 {
        return Err(RewardClaimMintError::InvalidInput);
    }
    for revision in [
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ] {
        mint_audit::check_content_key(revision)?;
    }
    if let Some(achievement) = &request.achievement {
        let revision_ok = match &achievement.catalogue {
            AchievementCatalogueLookup::Earnable { revision } => valid_revision(revision),
            AchievementCatalogueLookup::Retired | AchievementCatalogueLookup::Absent => true,
        };
        if !valid_key(&achievement.key) || !revision_ok {
            return Err(RewardClaimMintError::InvalidInput);
        }
        if achievement.catalogue == AchievementCatalogueLookup::Absent {
            return Err(RewardClaimMintError::UnknownAchievement);
        }
    }
    Ok(())
}

/// The grant request of the chest's achievement. Its source event is the
/// claim: the Character's id and a SHA-256 over the claim family and
/// production key (the RewardClaim's unique key), 48 bytes.
fn achievement_grant(
    achievement: &RewardClaimAchievement,
    character_id: CharacterId,
    claim: &TypedDefinitionRef,
) -> Result<AchievementGrantRequest> {
    let mut claim_key = Vec::new();
    push_text(&mut claim_key, claim.family.as_bytes()).map_err(from_transfer_input)?;
    push_text(&mut claim_key, claim.production_key.as_bytes()).map_err(from_transfer_input)?;
    let mut event_id = character_id.as_bytes().to_vec();
    event_id.extend_from_slice(&Sha256::digest(&claim_key));
    Ok(AchievementGrantRequest {
        achievement_key: achievement.key.clone(),
        catalogue: achievement.catalogue.clone(),
        source: AchievementSourceEvent {
            kind: ACHIEVEMENT_SOURCE_KIND.into(),
            event_id,
        },
    })
}

/// Version byte plus SHA-256 over the complete intent: the CommandRef, the
/// fenced Character, the claim, the reward item facts and quantity, the
/// backpack facts, the interpretation revisions and, only when the chest has
/// one, its achievement and catalogue lookup (so a claim without one keeps
/// its binding). The connection generation is not part of the intent
/// (DUR-03 §31).
fn intent_binding(request: &RewardClaimMintRequest, character_id: CharacterId) -> Result<[u8; 33]> {
    let mut canonical = Vec::new();
    canonical.extend_from_slice(request.command.game_session_id().as_bytes());
    canonical.extend_from_slice(&request.command.command_id().get().to_be_bytes());
    canonical.extend_from_slice(character_id.as_bytes());
    for part in [
        &request.claim.family,
        &request.claim.production_key,
        &request.claim.revision_ref,
    ] {
        push_text(&mut canonical, part.as_bytes()).map_err(from_transfer_input)?;
    }
    push_facts(&mut canonical, &request.item).map_err(from_transfer_input)?;
    canonical.extend_from_slice(&request.quantity.to_be_bytes());
    push_facts(&mut canonical, &request.backpack).map_err(from_transfer_input)?;
    for revision in [
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ] {
        push_text(&mut canonical, revision.as_bytes()).map_err(from_transfer_input)?;
    }
    if let Some(achievement) = &request.achievement {
        canonical.push(1);
        push_text(&mut canonical, achievement.key.as_bytes()).map_err(from_transfer_input)?;
        match &achievement.catalogue {
            AchievementCatalogueLookup::Earnable { revision } => {
                canonical.push(1);
                push_text(&mut canonical, revision.as_bytes()).map_err(from_transfer_input)?;
            }
            AchievementCatalogueLookup::Retired => canonical.push(2),
            AchievementCatalogueLookup::Absent => canonical.push(3),
        }
    }
    let mut out = [0_u8; 33];
    out[0] = INTENT_BINDING_VERSION;
    out[1..].copy_from_slice(&Sha256::digest(&canonical));
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::durability::item_transfer::ItemStackClass;

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn definition(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }

    fn facts(key: &str, stack: ItemStackClass) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            definition: definition(key),
            stack,
            container_capacity: None,
            container_slot_equip_pattern: false,
        }
    }

    fn backpack_facts(capacity: u32) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            definition: definition("fixture:b3.backpack"),
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(capacity),
            container_slot_equip_pattern: true,
        }
    }

    const COIN: ItemStackClass = ItemStackClass::Stackable {
        proven_maximum: None,
    };

    fn entry(seed: u8, key: &str, quantity: u32, ordinal: u64) -> BackpackEntry {
        BackpackEntry {
            item: InventoryItem {
                item_instance_id: id(seed),
                definition: definition(key),
                quantity,
            },
            placement_ordinal: ordinal,
        }
    }

    fn plan(
        item: &ItemDefinitionFacts,
        quantity: u32,
        claimed: bool,
        entries: &[BackpackEntry],
        capacity: u32,
    ) -> std::result::Result<ContainerEntryPosition, RewardClaimRefusal> {
        let slot = InventoryItem {
            item_instance_id: id(1),
            definition: definition("fixture:b3.backpack"),
            quantity: 1,
        };
        let backpack = backpack_facts(capacity);
        plan_reward_claim_mint(&RewardPlanInput {
            item,
            quantity,
            backpack: &backpack,
            already_claimed: claimed,
            slot: Some(&slot),
            entries,
        })
    }

    #[test]
    fn a_new_entry_takes_the_highest_ordinal_plus_one_and_never_merges() {
        let coin = facts("fixture:b3.coin", COIN);
        // A compatible coin stack with room exists; the reward still takes a
        // new entry (no mint into an existing stack, DUR-03 §39.1).
        let entries = [entry(5, "fixture:b3.coin", 10, 4), entry(6, "x", 1, 2)];
        let position = plan(&coin, 30, false, &entries, 20).expect("admitted");
        assert_eq!(position.parent_item_instance_id, id(1));
        assert_eq!(position.placement_ordinal, 5);
        assert_eq!(
            plan(&coin, 1, false, &[], 20)
                .expect("admitted")
                .placement_ordinal,
            1
        );
    }

    #[test]
    fn every_refusal_is_typed() {
        use RewardClaimRefusal as R;
        let coin = facts("fixture:b3.coin", COIN);
        let stone = facts("fixture:b3.stone", ItemStackClass::NonStackable);
        let full: Vec<BackpackEntry> = (1..=20_u8)
            .map(|n| entry(100 + n, "x", 1, u64::from(n)))
            .collect();
        assert_eq!(plan(&coin, 1, true, &[], 20), Err(R::AlreadyClaimed));
        // A claimed chest is refused before any room check.
        assert_eq!(plan(&coin, 1, true, &full, 20), Err(R::AlreadyClaimed));
        let mut bag = facts("fixture:b3.bag", ItemStackClass::NonStackable);
        bag.container_capacity = Some(8);
        assert_eq!(plan(&bag, 1, false, &[], 20), Err(R::RewardIsContainer));
        assert_eq!(
            plan(&facts("u", ItemStackClass::Unknown), 1, false, &[], 20),
            Err(R::UnknownStackClass)
        );
        assert_eq!(
            plan(
                &facts(
                    "u",
                    ItemStackClass::Stackable {
                        proven_maximum: Some(101)
                    }
                ),
                1,
                false,
                &[],
                20
            ),
            Err(R::UnsupportedStackMaximum)
        );
        assert_eq!(
            plan(&stone, 2, false, &[], 20),
            Err(R::QuantityAboveStackMaximum)
        );
        assert_eq!(
            plan(&coin, 101, false, &[], 20),
            Err(R::QuantityAboveStackMaximum)
        );
        assert_eq!(
            plan(&coin, 0, false, &[], 20),
            Err(R::QuantityAboveStackMaximum)
        );
        assert_eq!(plan(&coin, 1, false, &full, 20), Err(R::MainBackpackFull));
        assert_eq!(
            plan(&coin, 1, false, &full[..3], 3),
            Err(R::MainBackpackFull)
        );
        assert_eq!(
            plan(&coin, 1, false, &[], 21),
            Err(R::UnsupportedContainerCapacity)
        );
        assert_eq!(
            plan(&coin, 1, false, &[], 0),
            Err(R::UnsupportedContainerCapacity)
        );
        let backpack = backpack_facts(20);
        let no_slot = plan_reward_claim_mint(&RewardPlanInput {
            item: &coin,
            quantity: 1,
            backpack: &backpack,
            already_claimed: false,
            slot: None,
            entries: &[],
        });
        assert_eq!(no_slot, Err(R::NoMainBackpack));
        let other_slot = InventoryItem {
            item_instance_id: id(1),
            definition: definition("fixture:other.backpack"),
            quantity: 1,
        };
        let mismatch = plan_reward_claim_mint(&RewardPlanInput {
            item: &coin,
            quantity: 1,
            backpack: &backpack,
            already_claimed: false,
            slot: Some(&other_slot),
            entries: &[],
        });
        assert_eq!(mismatch, Err(R::DefinitionMismatch));
        // The last free entry is admitted (strict count < capacity).
        assert!(plan(&coin, 100, false, &full[..19], 20).is_ok());
    }

    fn request(achievement: Option<(&str, AchievementCatalogueLookup)>) -> RewardClaimMintRequest {
        use crate::foundation::{CommandId, GameSessionId};
        RewardClaimMintRequest {
            command: CommandRef::new(
                GameSessionId::decode(&id(50)).expect("session"),
                CommandId::new(7).expect("command"),
            ),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: "fixture:chest.claim".into(),
                revision_ref: "definition-r1".into(),
            },
            item: facts("fixture:b3.coin", COIN),
            quantity: 3,
            backpack: backpack_facts(20),
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
            achievement: achievement.map(|(key, catalogue)| RewardClaimAchievement {
                key: key.into(),
                catalogue,
            }),
        }
    }

    #[test]
    fn the_achievement_is_validated_and_bound_into_the_intent() {
        use AchievementCatalogueLookup as Lookup;
        const KEY: &str = "oteryn:achievement/allow_cookies";
        let earnable = |revision: &str| Lookup::Earnable {
            revision: revision.into(),
        };
        assert!(validate_request(&request(None)).is_ok());
        assert!(validate_request(&request(Some((KEY, earnable("r1"))))).is_ok());
        assert!(validate_request(&request(Some((KEY, Lookup::Retired)))).is_ok());
        assert!(matches!(
            validate_request(&request(Some((KEY, Lookup::Absent)))),
            Err(RewardClaimMintError::UnknownAchievement)
        ));
        for (key, catalogue) in [
            ("oteryn:achievement/Allow", earnable("r1")),
            ("oteryn:achievement/allow", earnable("-r1")),
            ("oteryn:achievement/", Lookup::Absent),
        ] {
            assert!(matches!(
                validate_request(&request(Some((key, catalogue)))),
                Err(RewardClaimMintError::InvalidInput)
            ));
        }

        // Each achievement intent has its own binding. `None` appends
        // nothing, so a claim without one keeps its pre-achievement binding
        // (pinned: an in-flight reservation stays resumable).
        let character = CharacterId::from_bytes(id(41)).expect("character");
        let bind =
            |request: &RewardClaimMintRequest| intent_binding(request, character).expect("binding");
        let hex: String = bind(&request(None))
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex,
            "016d5194b02ecc07d47aef727e1a1077ba93027a8a51c607f5096bf95193f63429"
        );
        let bindings = [
            bind(&request(None)),
            bind(&request(Some((KEY, earnable("r1"))))),
            bind(&request(Some((KEY, earnable("r2"))))),
            bind(&request(Some((KEY, Lookup::Retired)))),
            bind(&request(Some(("oteryn:achievement/other", earnable("r1"))))),
        ];
        for (index, binding) in bindings.iter().enumerate() {
            assert_eq!(binding[0], INTENT_BINDING_VERSION);
            assert!(bindings[index + 1..].iter().all(|other| other != binding));
        }
    }

    #[test]
    fn the_grant_source_event_is_the_claim_of_the_character() {
        let achievement = RewardClaimAchievement {
            key: "oteryn:achievement/allow_cookies".into(),
            catalogue: AchievementCatalogueLookup::Retired,
        };
        let character = CharacterId::from_bytes(id(41)).expect("character");
        let claim = request(None).claim;
        let grant = achievement_grant(&achievement, character, &claim).expect("grant");
        assert_eq!(grant.source.kind, ACHIEVEMENT_SOURCE_KIND);
        assert_eq!(grant.source.event_id.len(), 48);
        assert_eq!(&grant.source.event_id[..16], character.as_bytes());
        // The claim revision is not part of the claim's identity (D40).
        let mut revised = claim.clone();
        revised.revision_ref = "definition-r2".into();
        assert_eq!(
            achievement_grant(&achievement, character, &revised)
                .expect("grant")
                .source,
            grant.source
        );
        let mut other = claim.clone();
        other.production_key = "fixture:chest.other".into();
        assert_ne!(
            achievement_grant(&achievement, character, &other)
                .expect("grant")
                .source,
            grant.source
        );
        let second = CharacterId::from_bytes(id(44)).expect("character");
        assert_ne!(
            achievement_grant(&achievement, second, &claim)
                .expect("grant")
                .source,
            grant.source
        );
    }
}
