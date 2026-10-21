//! Durable Wheel of Destiny allocation (WHEEL-W1, migration 0070).
//!
//! `docs/architecture/reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md`
//! §4 on the Wheel state candidate §3.2-§3.4: each Character's 36 slot points, pinned to the
//! Wheel ruleset revision they were validated under.
//!
//! [`DurabilityRoot::commit_character_wheel`] commits one full replacement of the slot vector as
//! one CharacterRevision with one `ALLOCATION` receipt, or writes nothing and returns a
//! [`WheelRefusal`]. It is fenced exactly like `commit_character_experience` (composition rule 2:
//! the session fence, then `character_root` FOR UPDATE, then the Wheel rows). The request carries
//! the expected `wheel_revision`, not the CharacterRevision: a stale one is `STALE_REVISION`, and
//! the binding excludes the CharacterRevision, so a `CharacterRevisionMismatch` reloads the cursor
//! and retries once in the revision slot. The receipt is keyed by (Character, occurrence) and
//! binds the request only: the same occurrence and binding replay the first outcome without
//! reacquiring session authority; another binding conflicts.
//!
//! [`DurabilityRoot::reset_character_wheel`] is the admission Wheel reset (WHEEL0-RST-1): when a
//! `RESET` revision lies after the stored revision up to the active one, one `RULESET_RESET`
//! receipt clears the allocation under the active revision. Its occurrence is derived from the
//! Character and both revisions, so a replay finds the stored revision active and writes
//! nothing. Runtime callers reach both writers only through a
//! [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
//!
//! [`DurabilityRoot::read_character_wheel`] loads the allocation for the runtime actor, and
//! [`WheelStages::derive`] turns it into the stages a cast reads (WHEEL0-EL-1): from the cached
//! allocation and the eligibility read at that moment, never a cached stage.
//!
//! The Wheel ruleset is the embedded `rulesets/progression/wheel-of-destiny/wheel.json`
//! catalogue ([`WheelRuleset::from_catalogue`]); its revision and its 36 capacities must be the
//! ones migration 0070 (or a later W-R migration) registered, or the allocation fails closed.

use std::collections::{BTreeMap, BTreeSet};

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::character_revision_sequencer::CharacterRevisionSequencer;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
type Transaction<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

const BINDING_VERSION: u8 = 1;

pub use oteryn_gameplay_progression::wheel::*;

impl From<WheelInputError> for CharacterProgressionError {
    fn from(_: WheelInputError) -> Self {
        Self::InvalidInput
    }
}

/// A Wheel read made under the cast's already checked physical Character/Item transaction.
/// The private constructor prevents the admission cache from masquerading as a current read.
pub struct CurrentWheelCastRead {
    raw: super::character_equipment::RawCastDurableFacts,
    allocation: WheelAllocation,
    ruleset_revision: String,
}
impl CurrentWheelCastRead {
    pub fn raw(&self) -> &super::character_equipment::RawCastDurableFacts {
        &self.raw
    }
    pub fn allocation(&self) -> &WheelAllocation {
        &self.allocation
    }
    pub fn ruleset_revision(&self) -> &str {
        &self.ruleset_revision
    }
}

/// Reuses the actual equipment proof: its reader checks the physical transaction identity and
/// holds the Character root lock that every allocation/reset writer also takes. No independent
/// transaction, admission cache, write or guessed Wheel revision supplies cast authority.
pub async fn read_current_allocation_in_transaction(
    tx: &mut Transaction<'_>,
    authority: &super::character_equipment::EquipmentAuthority,
    ruleset: &WheelRuleset,
) -> std::result::Result<CurrentWheelCastRead, DurabilityError> {
    let raw = super::character_equipment::read_raw_cast_facts_in_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let stored = read_wheel_state(tx, raw.fence.character_id, false).await?;
    let allocation = allocation_from_stored(tx, stored, ruleset).await?;
    Ok(CurrentWheelCastRead {
        raw,
        allocation,
        ruleset_revision: ruleset.revision().to_owned(),
    })
}

/// The idempotency identity of one allocation change (UUIDv7), issued once by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WheelOccurrence([u8; 16]);

impl WheelOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// `WHEEL_INTENT set_allocation` (§7.1): a full replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelChangeRequest {
    pub occurrence: WheelOccurrence,
    pub expected_wheel_revision: u64,
    pub slots: WheelSlots,
}

/// Runtime facts read at the change: the Character's promotion (PREM-2; `false` until a durable
/// promotion owner exists) and whether it stands where points may be removed (§7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelChangeFacts {
    pub promoted: bool,
    pub at_temple: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelReceiptKind {
    Allocation,
    RulesetReset,
}

impl WheelReceiptKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Allocation => "ALLOCATION",
            Self::RulesetReset => "RULESET_RESET",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedWheelChange {
    pub occurrence: [u8; 16],
    pub kind: WheelReceiptKind,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub before_wheel_revision: u64,
    pub after_wheel_revision: u64,
    pub before: WheelSlots,
    pub after: WheelSlots,
    pub before_wheel_ruleset_revision: String,
    pub wheel_ruleset_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WheelCommitOutcome {
    Committed(CommittedWheelChange),
    AlreadyCommitted(CommittedWheelChange),
    Refused(WheelRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WheelResetOutcome {
    Reset(Box<CommittedWheelChange>),
    /// Nothing to reset: no row, the active revision, or only value-only revisions after it.
    Current,
    /// The stored or active revision is not registered in that order (a newer stored revision,
    /// an unregistered active one or other capacities): nothing is written and the allocation
    /// fails closed.
    Unresolved,
}

pub use oteryn_gameplay_progression::wheel_read::{
    Currency, derive_currency, read_allocation, revision_range,
};

impl DurabilityRoot {
    /// Commit one full replacement of the Character's slot vector. Exact occurrence replay
    /// returns the retained receipt without reacquiring session authority; another binding of
    /// the occurrence conflicts. A refusal writes nothing and consumes no occurrence.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: WheelChangeRequest,
        ruleset: std::sync::Arc<WheelRuleset>,
        facts: WheelChangeFacts,
    ) -> Result<WheelCommitOutcome> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let binding = request_binding(fence.character_id, &request);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_occurrence(&mut tx, &request.occurrence.0).await?;

                    if let Some(row) =
                        load_receipt(&mut tx, fence.character_id, &request.occurrence.0).await?
                    {
                        let stored: Vec<u8> = row.try_get("request_binding")?;
                        if stored != binding {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(WheelCommitOutcome::AlreadyCommitted(committed)));
                    }

                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let state = match lock_progression_state(&mut tx, fence.character_id).await? {
                        Some(state) => state,
                        None => {
                            return Ok(Err(CharacterProgressionError::MissingProgressionState));
                        }
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let level = u32::try_from(state.try_get::<i64, _>("level")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let stored = lock_wheel_state(&mut tx, fence.character_id).await?;
                    let refuse = |refusal| Ok(Ok(WheelCommitOutcome::Refused(refusal)));

                    let currency = match &stored {
                        Some(stored) => {
                            currency(&mut tx, &stored.wheel_ruleset_revision, &ruleset).await?
                        }
                        None => match currency(&mut tx, ruleset.revision(), &ruleset).await? {
                            Currency::Current => Currency::Current,
                            _ => Currency::Unresolved,
                        },
                    };
                    if currency != Currency::Current {
                        return refuse(WheelRefusal::RulesetNotCurrent);
                    }
                    let (before_revision, before) =
                        stored.as_ref().map_or((0, WheelSlots::ZERO), |stored| {
                            (stored.wheel_revision, stored.slots)
                        });
                    if request.expected_wheel_revision != before_revision {
                        return refuse(WheelRefusal::StaleRevision);
                    }
                    if request.slots == before {
                        return refuse(WheelRefusal::NoChange);
                    }
                    let vocation = read_vocation(&mut tx, fence.character_id).await?;
                    if let Err(reason) = eligibility(&ruleset, &vocation, level, facts.promoted) {
                        return refuse(WheelRefusal::NotEligible(reason));
                    }
                    if let Err(refusal) = ruleset.validate(
                        &before,
                        &request.slots,
                        available_points(level),
                        facts.at_temple,
                    ) {
                        return refuse(refusal);
                    }
                    // An allocation keeps its pinned revision; a first row takes the active one.
                    let pinned = stored.as_ref().map_or_else(
                        || ruleset.revision().to_owned(),
                        |stored| stored.wheel_ruleset_revision.clone(),
                    );
                    let change = PlannedChange {
                        occurrence: request.occurrence.0,
                        kind: WheelReceiptKind::Allocation,
                        binding: binding.clone(),
                        original: root.revision,
                        before_wheel_revision: before_revision,
                        before,
                        after: request.slots,
                        before_ruleset: pinned.clone(),
                        after_ruleset: pinned,
                        first: stored.is_none(),
                    };
                    let committed = write_change(&mut tx, fence.character_id, &change).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(WheelCommitOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// The admission Wheel reset (WHEEL0-RST-1): after the admission's own session fence and
    /// `character_root` FOR UPDATE, clear an allocation whose revision has a `RESET` revision
    /// after it up to the active one, with one `RULESET_RESET` receipt. Free; no eligibility,
    /// points or temple rule applies. Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot).
    pub async fn reset_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        ruleset: std::sync::Arc<WheelRuleset>,
    ) -> Result<WheelResetOutcome> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let state = match lock_progression_state(&mut tx, fence.character_id).await? {
                        Some(state) => state,
                        None => {
                            return Ok(Err(CharacterProgressionError::MissingProgressionState));
                        }
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let Some(stored) = lock_wheel_state(&mut tx, fence.character_id).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(WheelResetOutcome::Current));
                    };
                    let outcome =
                        match currency(&mut tx, &stored.wheel_ruleset_revision, &ruleset).await? {
                            Currency::Current => WheelResetOutcome::Current,
                            Currency::Unresolved => WheelResetOutcome::Unresolved,
                            Currency::NeedsReset => {
                                let occurrence = reset_occurrence(
                                    fence.character_id,
                                    &stored.wheel_ruleset_revision,
                                    ruleset.revision(),
                                );
                                let change = PlannedChange {
                                    occurrence,
                                    kind: WheelReceiptKind::RulesetReset,
                                    binding: reset_binding(
                                        fence.character_id,
                                        &stored.wheel_ruleset_revision,
                                        ruleset.revision(),
                                    ),
                                    original: root.revision,
                                    before_wheel_revision: stored.wheel_revision,
                                    before: stored.slots,
                                    after: WheelSlots::ZERO,
                                    before_ruleset: stored.wheel_ruleset_revision.clone(),
                                    after_ruleset: ruleset.revision().to_owned(),
                                    first: false,
                                };
                                WheelResetOutcome::Reset(Box::new(
                                    write_change(&mut tx, fence.character_id, &change).await?,
                                ))
                            }
                        };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(outcome))
                })
            })
            .await?
    }

    /// Read a retained change after a lost response. This proves only what committed for the
    /// occurrence and never reacquires gameplay authority.
    pub async fn reconcile_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        occurrence: WheelOccurrence,
    ) -> Result<Option<CommittedWheelChange>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let record = load_receipt(&mut tx, character_id, &occurrence.0)
                        .await?
                        .as_ref()
                        .map(decode_receipt)
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(record))
                })
            })
            .await?
    }

    /// The allocation a new runtime actor caches (§4 "Load"). No row reads every slot 0 under
    /// the active revision. A slot above its capacity under the stored revision, a total that is
    /// not the sum of the slots, or an unknown stored revision fails the load closed with
    /// `Unavailable(InvalidStoredState)`.
    pub async fn read_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        ruleset: std::sync::Arc<WheelRuleset>,
    ) -> Result<WheelAllocation> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let stored = read_wheel_state(&mut tx, character_id, false).await?;
                    let allocation = allocation_from_stored(&mut tx, stored, &ruleset).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(allocation))
                })
            })
            .await?
    }
}

/// The admission step of fresh admission and resume (§4 "Wheel reset" and "Load"): the Wheel
/// reset in the Character's revision slot, then the load. `None` when the load failed: Wheel
/// actions of the session then fail closed, and login does not. A reset that did not commit
/// leaves the allocation not current, so every stage derives as 0.
pub async fn admit_character_wheel(
    sequencer: &CharacterRevisionSequencer,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    ruleset: &std::sync::Arc<WheelRuleset>,
) -> Option<WheelAllocation> {
    let mut slot = sequencer.acquire(fence.character_id).await;
    let reset = slot
        .reset_wheel(root, authority, node, fence, std::sync::Arc::clone(ruleset))
        .await;
    drop(slot);
    if let Err(error) = &reset {
        eprintln!("oteryn-game-server: Wheel reset at admission did not commit: {error}");
    }
    root.read_character_wheel(
        authority,
        fence.character_id,
        std::sync::Arc::clone(ruleset),
    )
    .await
    .ok()
}

/// One stored Wheel state row with its slots.
struct StoredWheel {
    wheel_ruleset_revision: String,
    wheel_revision: u64,
    slots: WheelSlots,
}

/// One change to write: the receipt and the row successor.
struct PlannedChange {
    occurrence: [u8; 16],
    kind: WheelReceiptKind,
    binding: Vec<u8>,
    original: u64,
    before_wheel_revision: u64,
    before: WheelSlots,
    after: WheelSlots,
    before_ruleset: String,
    after_ruleset: String,
    first: bool,
}

async fn lock_occurrence(
    tx: &mut Transaction<'_>,
    occurrence: &[u8; 16],
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:character-wheel:' || encode($1, 'hex'), 0))",
    )
    .bind(occurrence.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn lock_progression_state(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT character_revision::text, level, profile_revision, ruleset_revision, \
                content_revision \
           FROM game_character_progression_state \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

async fn lock_wheel_state(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
) -> std::result::Result<Option<StoredWheel>, DurabilityError> {
    read_wheel_state(tx, character_id, true).await
}

async fn read_wheel_state(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
    for_update: bool,
) -> std::result::Result<Option<StoredWheel>, DurabilityError> {
    let query = if for_update {
        "SELECT wheel_ruleset_revision, wheel_revision::text, allocated_total \
           FROM game_character_wheel_state \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE"
    } else {
        "SELECT wheel_ruleset_revision, wheel_revision::text, allocated_total \
           FROM game_character_wheel_state \
          WHERE character_id = encode($1,'hex')::uuid"
    };
    let Some(row) = sqlx::query(query)
        .bind(character_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?
    else {
        return Ok(None);
    };
    let rows = sqlx::query(
        "SELECT slot, points FROM game_character_wheel_slots \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    let mut points = [0_u16; WHEEL_SLOTS];
    for slot in rows {
        let index = usize::try_from(slot.try_get::<i16, _>("slot")?)
            .ok()
            .and_then(|slot| slot.checked_sub(1))
            .filter(|index| *index < WHEEL_SLOTS)
            .ok_or(DurabilityError::InvalidStoredState)?;
        points[index] = u16::try_from(slot.try_get::<i16, _>("points")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    let slots = WheelSlots::new(points).map_err(|_| DurabilityError::InvalidStoredState)?;
    let total = u32::try_from(row.try_get::<i32, _>("allocated_total")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if total != slots.total() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(StoredWheel {
        wheel_ruleset_revision: row.try_get("wheel_ruleset_revision")?,
        wheel_revision: numeric_u64(&row, "wheel_revision")?,
        slots,
    }))
}

async fn allocation_from_stored(
    tx: &mut Transaction<'_>,
    stored: Option<StoredWheel>,
    ruleset: &WheelRuleset,
) -> std::result::Result<WheelAllocation, DurabilityError> {
    let stored = match stored {
        None => None,
        Some(stored) => {
            let capacities = stored_capacities(tx, &stored.wheel_ruleset_revision).await?;
            Some((
                stored.wheel_ruleset_revision,
                stored.wheel_revision,
                stored.slots,
                capacities,
            ))
        }
    };
    let mut allocation =
        read_allocation(stored, false).map_err(|_| DurabilityError::InvalidStoredState)?;
    allocation.current = currency(
        tx,
        allocation
            .wheel_ruleset_revision
            .as_deref()
            .unwrap_or(ruleset.revision()),
        ruleset,
    )
    .await?
        == Currency::Current;
    Ok(allocation)
}

/// The Character's vocation key; `none` without a build row.
async fn read_vocation(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
) -> std::result::Result<String, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT vocation FROM game_character_build_state \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or_else(|| "none".to_owned()))
}

async fn stored_capacities(
    tx: &mut Transaction<'_>,
    revision: &str,
) -> std::result::Result<[u16; WHEEL_SLOTS], DurabilityError> {
    let rows = sqlx::query(
        "SELECT slot, capacity FROM game_wheel_ruleset_slot_capacities \
          WHERE wheel_ruleset_revision = $1",
    )
    .bind(revision)
    .fetch_all(&mut **tx)
    .await?;
    let mut capacities = [0_u16; WHEEL_SLOTS];
    let mut seen = BTreeSet::new();
    for row in rows {
        let slot = usize::try_from(row.try_get::<i16, _>("slot")?)
            .ok()
            .filter(|slot| (1..=WHEEL_SLOTS).contains(slot))
            .ok_or(DurabilityError::InvalidStoredState)?;
        capacities[slot - 1] = u16::try_from(row.try_get::<i16, _>("capacity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        seen.insert(slot);
    }
    if seen.len() != WHEEL_SLOTS {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(capacities)
}

/// The stored revision `stored` against the active `ruleset` (§4 WHEEL0-RST-1, §5.1). The active
/// revision must be registered with the ruleset's own capacities, and no later than nothing.
async fn currency(
    tx: &mut Transaction<'_>,
    stored: &str,
    ruleset: &WheelRuleset,
) -> std::result::Result<Currency, DurabilityError> {
    let rows = sqlx::query(
        "SELECT wheel_ruleset_revision, ordinal, kind FROM game_wheel_ruleset_revisions",
    )
    .fetch_all(&mut **tx)
    .await?;
    let mut order = BTreeMap::new();
    for row in rows {
        order.insert(
            row.try_get::<String, _>("wheel_ruleset_revision")?,
            (
                row.try_get::<i32, _>("ordinal")?,
                row.try_get::<String, _>("kind")?,
            ),
        );
    }
    let Some((stored_ordinal, active_ordinal)) = revision_range(&order, stored, ruleset.revision())
    else {
        return Ok(Currency::Unresolved);
    };
    let capacities = stored_capacities(tx, ruleset.revision()).await?;
    Ok(derive_currency(
        &order,
        stored_ordinal,
        active_ordinal,
        capacities == ruleset.capacities(),
    ))
}

/// Write one planned change: the root and typed-state successor, the receipt, the state row and
/// the changed slot rows, in that order.
async fn write_change(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
    change: &PlannedChange,
) -> std::result::Result<CommittedWheelChange, DurabilityError> {
    let character = character_id.as_bytes().as_slice();
    let committed_revision = change
        .original
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let after_wheel_revision = change
        .before_wheel_revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let root_update = sqlx::query(
        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
          WHERE character_id = encode($1,'hex')::uuid \
            AND character_revision = $3::text::numeric(20,0)",
    )
    .bind(character)
    .bind(committed_revision.to_string())
    .bind(change.original.to_string())
    .execute(&mut **tx)
    .await?;
    let state_update = sqlx::query(
        "UPDATE game_character_progression_state \
            SET character_revision = $2::text::numeric(20,0) \
          WHERE character_id = encode($1,'hex')::uuid \
            AND character_revision = $3::text::numeric(20,0)",
    )
    .bind(character)
    .bind(committed_revision.to_string())
    .bind(change.original.to_string())
    .execute(&mut **tx)
    .await?;
    if root_update.rows_affected() != 1 || state_update.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let receipt = sqlx::query(
        "INSERT INTO game_character_wheel_receipts(\
           character_id, wheel_occurrence_id, kind, request_binding, before_wheel_revision, \
           after_wheel_revision, slots_before, slots_after, before_wheel_ruleset_revision, \
           wheel_ruleset_revision, original_character_revision, committed_character_revision, \
           level_before, level_after, experience_before, experience_after, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         SELECT s.character_id, encode($2,'hex')::uuid, $3, $4, $5::text::numeric(20,0), \
           $6::text::numeric(20,0), $7, $8, $9, $10, $11::text::numeric(20,0), \
           $12::text::numeric(20,0), s.level, s.level, s.total_experience, \
           s.total_experience, s.profile_revision, s.ruleset_revision, s.content_revision, \
           s.simulation_revision, s.evidence_revision, s.declaration_revision, \
           s.policy_revision, s.reward_revision, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
           FROM game_character_progression_state s \
          WHERE s.character_id = encode($1,'hex')::uuid",
    )
    .bind(character)
    .bind(change.occurrence.as_slice())
    .bind(change.kind.as_str())
    .bind(&change.binding)
    .bind(change.before_wheel_revision.to_string())
    .bind(after_wheel_revision.to_string())
    .bind(slot_columns(&change.before))
    .bind(slot_columns(&change.after))
    .bind(&change.before_ruleset)
    .bind(&change.after_ruleset)
    .bind(change.original.to_string())
    .bind(committed_revision.to_string())
    .execute(&mut **tx)
    .await?;
    if receipt.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let total =
        i32::try_from(change.after.total()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let row = if change.first {
        sqlx::query(
            "INSERT INTO game_character_wheel_state(character_id, wheel_ruleset_revision, \
               wheel_revision, allocated_total, committed_character_revision, \
               last_wheel_occurrence_id) \
             VALUES (encode($1,'hex')::uuid, $2, $3::text::numeric(20,0), $4, \
               $5::text::numeric(20,0), encode($6,'hex')::uuid)",
        )
    } else {
        sqlx::query(
            "UPDATE game_character_wheel_state SET wheel_ruleset_revision = $2, \
               wheel_revision = $3::text::numeric(20,0), allocated_total = $4, \
               committed_character_revision = $5::text::numeric(20,0), \
               last_wheel_occurrence_id = encode($6,'hex')::uuid \
             WHERE character_id = encode($1,'hex')::uuid",
        )
    }
    .bind(character)
    .bind(&change.after_ruleset)
    .bind(after_wheel_revision.to_string())
    .bind(total)
    .bind(committed_revision.to_string())
    .bind(change.occurrence.as_slice())
    .execute(&mut **tx)
    .await?;
    if row.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Only the slots that change: a slot at 0 has no row.
    let (mut kept_slots, mut kept_points) = (Vec::new(), Vec::new());
    for (index, points) in change.after.points().iter().enumerate() {
        if *points > 0 {
            kept_slots.push(i16::try_from(index + 1).unwrap_or(i16::MAX));
            kept_points.push(i16::try_from(*points).unwrap_or(i16::MAX));
        }
    }
    sqlx::query(
        "DELETE FROM game_character_wheel_slots \
          WHERE character_id = encode($1,'hex')::uuid AND NOT (slot = ANY($2))",
    )
    .bind(character)
    .bind(&kept_slots)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_wheel_slots(character_id, slot, points) \
         SELECT encode($1,'hex')::uuid, s.slot, s.points \
           FROM unnest($2::smallint[], $3::smallint[]) AS s(slot, points) \
         ON CONFLICT (character_id, slot) DO UPDATE SET points = EXCLUDED.points \
          WHERE game_character_wheel_slots.points <> EXCLUDED.points",
    )
    .bind(character)
    .bind(&kept_slots)
    .bind(&kept_points)
    .execute(&mut **tx)
    .await?;

    let revision =
        |value| CharacterRevision::new(value).map_err(|_| DurabilityError::InvalidStoredState);
    Ok(CommittedWheelChange {
        occurrence: change.occurrence,
        kind: change.kind,
        character_id,
        original_character_revision: revision(change.original)?,
        committed_character_revision: revision(committed_revision)?,
        before_wheel_revision: change.before_wheel_revision,
        after_wheel_revision,
        before: change.before,
        after: change.after,
        before_wheel_ruleset_revision: change.before_ruleset.clone(),
        wheel_ruleset_revision: change.after_ruleset.clone(),
    })
}

fn bound(semantic: &[u8]) -> Vec<u8> {
    let digest: [u8; 32] = Sha256::digest(semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(BINDING_VERSION);
    binding.extend_from_slice(&digest);
    binding
}

/// The request-only binding of a change (§4): occurrence, Character, expected Wheel revision and
/// slot vector. The CharacterRevision and every retry-local authority are excluded.
fn request_binding(character: CharacterId, request: &WheelChangeRequest) -> Vec<u8> {
    let mut semantic = vec![BINDING_VERSION, b'A'];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(character.as_bytes());
    semantic.extend_from_slice(&request.expected_wheel_revision.to_be_bytes());
    for points in *request.slots.points() {
        semantic.extend_from_slice(&points.to_be_bytes());
    }
    bound(&semantic)
}

fn reset_semantic(character: CharacterId, source: &str, destination: &str) -> Vec<u8> {
    let mut semantic = vec![BINDING_VERSION, b'R'];
    semantic.extend_from_slice(character.as_bytes());
    semantic.extend_from_slice(source.as_bytes());
    semantic.push(0);
    semantic.extend_from_slice(destination.as_bytes());
    semantic
}

fn reset_binding(character: CharacterId, source: &str, destination: &str) -> Vec<u8> {
    bound(&reset_semantic(character, source, destination))
}

/// The occurrence of a reset, derived from (Character, source, destination) (§13 item 3): a
/// version 8 UUID of the binding's digest.
fn reset_occurrence(character: CharacterId, source: &str, destination: &str) -> [u8; 16] {
    let mut semantic = b"oteryn:wheel-reset:".to_vec();
    semantic.extend_from_slice(&reset_semantic(character, source, destination));
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut occurrence = [0_u8; 16];
    occurrence.copy_from_slice(&digest[..16]);
    occurrence[6] = (occurrence[6] & 0x0f) | 0x80;
    occurrence[8] = (occurrence[8] & 0x3f) | 0x80;
    occurrence
}

async fn load_receipt(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
    occurrence: &[u8; 16],
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT wheel_occurrence_id::text, kind, request_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                before_wheel_revision::text, after_wheel_revision::text, slots_before, \
                slots_after, before_wheel_ruleset_revision, wheel_ruleset_revision \
           FROM game_character_wheel_receipts \
          WHERE character_id = encode($1,'hex')::uuid \
            AND wheel_occurrence_id = encode($2,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .bind(occurrence.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedWheelChange, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let kind = match row.try_get::<String, _>("kind")?.as_str() {
        "ALLOCATION" => WheelReceiptKind::Allocation,
        "RULESET_RESET" => WheelReceiptKind::RulesetReset,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(CommittedWheelChange {
        occurrence: uuid_text(row.try_get("wheel_occurrence_id")?)?,
        kind,
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(invalid)?,
        original_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "original_character_revision",
        )?)
        .map_err(invalid)?,
        committed_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "committed_character_revision",
        )?)
        .map_err(invalid)?,
        before_wheel_revision: numeric_u64(row, "before_wheel_revision")?,
        after_wheel_revision: numeric_u64(row, "after_wheel_revision")?,
        before: stored_slots(&row.try_get::<Vec<i16>, _>("slots_before")?)?,
        after: stored_slots(&row.try_get::<Vec<i16>, _>("slots_after")?)?,
        before_wheel_ruleset_revision: row.try_get("before_wheel_ruleset_revision")?,
        wheel_ruleset_revision: row.try_get("wheel_ruleset_revision")?,
    })
}


fn slot_columns(slots: &WheelSlots) -> Vec<i16> {
        // Each value is at most 200.
        slots.points()
            .iter()
            .map(|value| i16::try_from(*value).unwrap_or(i16::MAX))
            .collect()
    }

fn stored_slots(values: &[i16]) -> std::result::Result<WheelSlots, DurabilityError> {
        let mut points = [0_u16; WHEEL_SLOTS];
        if values.len() != WHEEL_SLOTS {
            return Err(DurabilityError::InvalidStoredState);
        }
        for (slot, value) in points.iter_mut().zip(values) {
            *slot = u16::try_from(*value).map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        WheelSlots::new(points).map_err(|_| DurabilityError::InvalidStoredState)
    }

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn slots_and_occurrences_are_bounded() {
        let mut points = [0; WHEEL_SLOTS];
        points[0] = MAX_SLOT_POINTS;
        assert!(WheelSlots::new(points).is_ok());
        points[0] = MAX_SLOT_POINTS + 1;
        assert!(WheelSlots::new(points).is_err());
        assert!(stored_slots(&[0; 35]).is_err());
        assert!(stored_slots(&[-1; 36]).is_err());
        let mut v7 = [0x11; 16];
        v7[6] = 0x70;
        v7[8] = 0x80;
        assert!(WheelOccurrence::from_bytes(v7).is_ok());
        v7[6] = 0x40;
        assert!(WheelOccurrence::from_bytes(v7).is_err());
    }

    #[test]
    fn a_reset_occurrence_is_derived_from_the_character_and_both_revisions() {
        let id = |seed: u8| {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            CharacterId::from_bytes(bytes).expect("character")
        };
        let (character, other) = (id(3), id(4));
        let occurrence = reset_occurrence(character, "r1", "r2");
        assert_eq!(occurrence, reset_occurrence(character, "r1", "r2"));
        assert_ne!(occurrence, reset_occurrence(other, "r1", "r2"));
        assert_ne!(occurrence, reset_occurrence(character, "r1", "r3"));
        assert_ne!(occurrence, reset_occurrence(character, "r1r", "2"));
        assert_eq!(occurrence[6] >> 4, 8);
        assert_eq!(occurrence[8] & 0xc0, 0x80);
        assert_eq!(reset_binding(character, "r1", "r2").len(), 33);
    }
}
