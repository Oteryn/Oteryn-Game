//! DUR-03 native one-item Ground MINT (VSL-COMBAT-01 stage C).
//!
//! This component owns durable application, idempotency and reconciliation
//! only. It does not prove that Combat legitimately produced the loot output
//! cause: [`ItemMintCause`]'s only runtime constructor takes the typed death
//! key of the actual scope owner's committed death, and no caller bytes.
//!
//! Lifecycle of one logical MINT transaction:
//! 1. [`DurabilityRoot::freeze_item_mint`] validates every registered bound and
//!    commits, in its own transaction and keyed by the full cause tuple, a
//!    durable reservation of the logical transaction: TransactionId, EventId,
//!    ItemInstanceId, trusted timestamp, exact event bytes, the fence (scope
//!    ownership generation plus holder node incarnation) and the DUR03-RL-08
//!    work units used. Freezing an already reserved cause resumes that row:
//!    the same identities and the same budget, never new ones.
//! 2. [`DurabilityRoot::commit_item_mint`] commits that frozen candidate under
//!    the current recovery, admission-relation, runtime-scope assignment and
//!    node-incarnation fence. The reservation's fence must be the live one:
//!    the death's ownership generation is the current assignment held by the
//!    reserving incarnation (D52); otherwise the MINT is refused, never
//!    deferred and never re-reserved.
//! 3. After an unknown outcome, [`DurabilityRoot::reconcile_item_mint`] reads
//!    the receipt of the same cause. Committed returns the original result; not
//!    committed permits a retry of the same frozen candidate.
//!
//! Each commit attempt and reconciliation pass first charges one DUR03-RL-08
//! work unit to the reservation row with a conditional UPDATE committed before
//! the pass's work, so the budget is shared by every holder of the cause and
//! survives restarts; the fourth unit is rejected before the pass's work.

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use super::item_mint_audit::{
    self as audit, AuditError, CreatureDeathOccurrenceRefV1, ITEM_LIFECYCLE_LIVE,
    LOOT_MINT_TYPED_CAUSE, MintEventIdentity, OneItemGroundV1, OneItemMintV1, OneItemProvenanceV1,
    OneItemStateV1, OneItemTypedDefinitionRevisionV1, RL08_RETRY_WORK_UNITS_MAX,
    SelectedType2Tuple,
};
use super::item_transfer_audit::OneItemContainerEntryV1;
use super::runtime_scope_assignment::{
    NodeIncarnationProof, assignment_scope_key, prove_current_incarnation,
};
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::foundation::{
    ChannelId, CreatureDeathOccurrenceKey, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, ItemMintError>;
const INTENT_BINDING_VERSION: u8 = 1;
const INSTANCE_INTENT_BINDING_VERSION: u8 = 2;
#[derive(Clone, Copy)]
enum NewMintFormat {
    Canonical,
    // Faithfully create a pre-merge reservation through the same SQL authority,
    // fences, audit and work-budget owner. Never callable in production builds.
    #[cfg(test)]
    LegacyInstanceV1,
}

const EVENT_TYPE_ID: i64 = audit::EVENT_TYPE_ID as i64;

/// DUR-03 §39.4 / D3 §4.1: the reserved sentinel `loot_purpose_key` naming a
/// creature's own corpse MINT (always `draw_ordinal = 0`), never reused by an
/// ordinary loot entry's own cause.
pub const CORPSE_MATERIALIZATION_PURPOSE_KEY: &str = "CORPSE_MATERIALIZATION";

/// VSL resource-rows decision §4.1 row 6 / D3 §4.2: the one bound on live
/// corpse projections per scope (`COMBAT01-CORPSES-PER-SCOPE`), enforced only
/// at the corpse's own MINT commit under the per-scope `oteryn:corpse-cap:`
/// advisory lock -- never re-decided here.
pub(super) const COMBAT01_CORPSES_PER_SCOPE_MAX: i64 = 64;

/// Equality on the full tagged cause tuple, parameters `$1..$10`; never a hash.
macro_rules! cause_key {
    () => {
        "death_world_id = encode($1,'hex')::uuid \
         AND death_scope_id = $2 \
         AND death_scope_ownership_generation = $3::text::numeric(20,0) \
         AND death_actor_local_id = $4 \
         AND death_actor_local_generation = $5::text::numeric(20,0) \
         AND loot_table_family = $6 AND loot_table_production_key = $7 \
         AND loot_table_revision_ref = $8 AND loot_purpose_key = $9 \
         AND draw_ordinal = $10"
    };
}

/// Binds the full tagged cause tuple as `$1..$10` of a query using `cause_key!()`.
macro_rules! bind_cause {
    ($query:expr, $cause:expr) => {{
        let cause: &ItemMintCause = $cause;
        $query
            .bind(cause.death.world_id.as_bytes().as_slice())
            .bind(cause.death.scope_id().to_vec())
            .bind(cause.death.scope_ownership_generation.get().to_string())
            .bind(i64::from(cause.death.actor_local_id))
            .bind(cause.death.actor_local_generation.to_string())
            .bind(&cause.loot_table.family)
            .bind(&cause.loot_table.production_key)
            .bind(&cause.loot_table.revision_ref)
            .bind(&cause.purpose_key)
            .bind(i64::from(cause.draw_ordinal))
    }};
}

mod source_death;

pub use crate::domain::item_definition::TypedDefinitionRef;

/// `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` durable, restart-stable death key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDeathKey {
    world_id: WorldId,
    runtime_scope: RuntimeScopeRefV1,
    scope_ownership_generation: ScopeOwnershipGeneration,
    actor_local_id: u32,
    actor_local_generation: u64,
}

impl CreatureDeathKey {
    fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Full typed namespace for SQL cause equality; no Instance can alias a Channel.
    fn scope_id(self) -> [u8; 17] {
        let mut key = [0; 17];
        match self.runtime_scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                key[0] = 1;
                key[1..].copy_from_slice(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                key[0] = 2;
                key[1..].copy_from_slice(&instance_id);
            }
        }
        key
    }

    fn channel_bytes(self) -> Option<Vec<u8>> {
        self.runtime_scope
            .channel_id()
            .map(|id| id.as_bytes().to_vec())
    }

    fn instance_bytes(self) -> Option<Vec<u8>> {
        self.runtime_scope.instance_id().map(|id| id.to_vec())
    }

    fn scope_kind(self) -> i16 {
        // 1152_instance_creature_death_mints.sql:9-13: the persisted typed
        // discriminator and assignment-scope key must both be explicit. This
        // also keeps exact writers valid when fixture triggers are disabled.
        match self.runtime_scope {
            RuntimeScopeRefV1::Channel { .. } => 1,
            RuntimeScopeRefV1::Instance { .. } => 2,
        }
    }

    /// Historical Channel intent bindings retain their exact original bytes.
    fn append_scope_id(self, bytes: &mut Vec<u8>) {
        match self.runtime_scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                bytes.extend_from_slice(channel_id.as_bytes())
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                bytes.extend_from_slice(b"instance:");
                bytes.extend_from_slice(&instance_id);
            }
        }
    }
}

/// The typed loot output cause `(death key, LootTableDefinitionRef,
/// LootEntryOrPurposeKey, DeterministicDrawOrdinal)`, stored in full as the
/// MINT's unique source cause. The runtime constructor takes the death key
/// only from the physical scope owner's committed lethal occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMintCause {
    death: CreatureDeathKey,
    loot_table: TypedDefinitionRef,
    purpose_key: String,
    draw_ordinal: u32,
    corpse_race: Option<crate::domain::bestiary::BestiaryRace>,
}

impl ItemMintCause {
    /// Scoped test fixture only. Production causes require a committed carrier death.
    #[cfg(any(test, feature = "test-support"))]
    pub fn for_test_scope(
        runtime_scope: RuntimeScopeRefV1,
        scope_ownership_generation: ScopeOwnershipGeneration,
        actor_local_id: u32,
        actor_local_generation: u64,
        loot_table: TypedDefinitionRef,
        purpose_key: String,
        draw_ordinal: u32,
    ) -> Self {
        Self {
            death: CreatureDeathKey {
                world_id: runtime_scope.world_id(),
                runtime_scope,
                scope_ownership_generation,
                actor_local_id,
                actor_local_generation,
            },
            loot_table,
            purpose_key,
            draw_ordinal,
            corpse_race: None,
        }
    }

    /// Loot output cause of one committed creature death (DUR-03 decision
    /// §4.2): the full typed tuple, with no caller-supplied death bytes. The
    /// death key has no constructor outside the carrier's committed lethal
    /// occurrence, so every cause built here descends from a real death.
    /// Crystal src/creatures/creature.cpp:711-721,777-782: physical corpse custody uses the
    /// actual death scope, independently of private boss reward entitlements.
    pub fn from_creature_death(
        death: CreatureDeathOccurrenceKey,
        loot_table: TypedDefinitionRef,
        purpose_key: String,
        draw_ordinal: u32,
    ) -> Result<Self> {
        Ok(Self {
            death: CreatureDeathKey {
                world_id: death.world_id(),
                runtime_scope: death.runtime_scope(),
                scope_ownership_generation: death.scope_ownership_generation(),
                actor_local_id: death.actor_local_id(),
                actor_local_generation: death.actor_local_generation(),
            },
            loot_table,
            purpose_key,
            draw_ordinal,
            corpse_race: None,
        })
    }

    /// Bind source Creature identity only to the committed death's own corpse.
    /// Skinning never derives race from a shared corpse Item definition.
    pub fn with_corpse_race(mut self, race: crate::domain::bestiary::BestiaryRace) -> Result<Self> {
        if self.purpose_key != CORPSE_MATERIALIZATION_PURPOSE_KEY || self.draw_ordinal != 0 {
            return Err(ItemMintError::InvalidInput);
        }
        self.corpse_race = Some(race);
        Ok(self)
    }

    /// Test-only: production causes descend from a committed Combat death.
    #[cfg(any(test, feature = "test-support"))]
    #[allow(clippy::too_many_arguments)]
    pub fn for_test(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_ownership_generation: ScopeOwnershipGeneration,
        actor_local_id: u32,
        actor_local_generation: u64,
        loot_table: TypedDefinitionRef,
        purpose_key: String,
        draw_ordinal: u32,
    ) -> Self {
        Self {
            death: CreatureDeathKey {
                world_id,
                runtime_scope: RuntimeScopeRefV1::channel(world_id, channel_id),
                scope_ownership_generation,
                actor_local_id,
                actor_local_generation,
            },
            loot_table,
            purpose_key,
            draw_ordinal,
            corpse_race: None,
        }
    }
}

/// Actual typed Ground in the death's exact runtime scope and generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundPlacement {
    pub spatial_position: Vec<u8>,
    pub corpse_ref: Vec<u8>,
    pub map_revision: String,
    pub content_revision: String,
    pub native_room_placement_context: Vec<u8>,
}

pub use crate::foundation::loot_data::CORPSE_CONTAINER_ENTRIES_MAX;

/// `Container { parent_item_instance_id = <the death's corpse>, entry }`
/// (DUR-03 §39.4, D3 §4.1/§4.2): where a loot entry's own MINT establishes
/// its fresh item instead of Ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseContainerPlacement {
    /// The committed corpse `ItemInstance` of the same death.
    pub corpse_item_instance_id: [u8; 16],
    /// `1..=CORPSE_CONTAINER_ENTRIES_MAX`; the caller assigns it so the
    /// frozen audit event and the replayed cause always agree on it.
    pub placement_ordinal: u32,
    /// W22a: the fluid the loot entry is minted with (`1..=255`, Crystal `item.type`); `None`
    /// is no fluid row, the empty container. Part of the intent binding when present.
    pub fluid_subtype: Option<u8>,
}

/// Complete semantic input of one loot MINT into a corpse container (D3-2).
/// The corpse's own MINT stays an ordinary Ground [`ItemMintRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpseLootMintRequest {
    pub cause: ItemMintCause,
    pub item: TypedDefinitionRef,
    pub quantity: u32,
    pub placement: CorpseContainerPlacement,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

/// Complete semantic input of one Ground MINT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMintRequest {
    pub cause: ItemMintCause,
    pub item: TypedDefinitionRef,
    pub quantity: u32,
    pub ground: GroundPlacement,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

/// Frozen candidate of one logical MINT transaction, a process-local view of
/// its durable reservation. It is deliberately not `Clone`; the authoritative
/// DUR03-RL-08 budget is the reservation row, and `work_units_used` is the
/// last value this holder observed there.
#[derive(Debug)]
pub struct ItemMintCandidate {
    request: ItemMintRequest,
    /// `Some` exactly for a corpse-loot MINT; `request.ground` is then an
    /// unused empty placeholder that no validation, binding or insert reads.
    corpse_entry: Option<CorpseContainerPlacement>,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    envelope: Vec<u8>,
    envelope_sha256: [u8; 32],
    intent_binding: [u8; 33],
    work_units_used: u8,
}

#[cfg(any(test, feature = "test-support"))]
impl ItemMintCandidate {
    /// A fresh lane permit of this candidate's exact death scope, for the PostgreSQL test support.
    #[allow(
        dead_code,
        reason = "used only by the PostgreSQL item-writer test support"
    )]
    #[allow(
        clippy::expect_used,
        reason = "a fresh native death lane has no retained failure state"
    )]
    pub async fn fresh_death_lane_permit(&self) -> SpellLanePermit {
        SpellLanePermit::of_fresh_scope_lane(self.request.cause.death.runtime_scope).await
    }
}

impl ItemMintCandidate {
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
    /// Exact immutable EventEnvelope bytes (event type 2).
    #[must_use]
    pub fn envelope(&self) -> &[u8] {
        &self.envelope
    }
    #[must_use]
    pub const fn work_units_used(&self) -> u8 {
        self.work_units_used
    }
}

/// The durable reservation row of one cause.
struct Reservation {
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    envelope: Vec<u8>,
    fence_node_id: [u8; 16],
    fence_registration_revision: u64,
    work_units_used: u8,
}

/// Terminal committed result of one cause. It never changes after commit,
/// including after the audit event's own retention expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedItemMint {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub item_instance_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemMintOutcome {
    Committed(CommittedItemMint),
    /// The cause already minted; this is its original terminal result.
    AlreadyCommitted(CommittedItemMint),
}

/// Durable Ground ItemInstance read back from PostgreSQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundItemInstance {
    pub item_instance_id: [u8; 16],
    pub world_id: WorldId,
    pub runtime_scope: crate::foundation::RuntimeScopeRefV1,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    pub runtime_scope_ownership_generation: u64,
    pub ground: GroundPlacement,
    pub minted_transaction_id: [u8; 16],
}

#[derive(Debug)]
pub enum ItemMintError {
    /// Malformed or oversize input; registered `INVALID_INPUT`, never truncated.
    InvalidInput,
    /// A registered DUR-03 ceiling (including RL-08) was exceeded.
    CapacityExceeded,
    /// The current recovery, runtime-scope or node-incarnation fence rejected
    /// the MINT. Under D52 a death whose generation ended is never minted.
    AuthorityRejected,
    /// The cause already minted with a different intent (integrity CONFLICT).
    ConflictingCause,
    /// A frozen identity is already bound to a different cause.
    ConflictingCandidate,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for ItemMintError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<AuditError> for ItemMintError {
    fn from(error: AuditError) -> Self {
        match error {
            AuditError::InvalidInput => Self::InvalidInput,
            AuditError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

impl std::fmt::Display for ItemMintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid item MINT input"),
            Self::CapacityExceeded => formatter.write_str("item MINT capacity exceeded"),
            Self::AuthorityRejected => formatter.write_str("item MINT authority rejected"),
            Self::ConflictingCause => {
                formatter.write_str("MINT cause was reused with different semantics")
            }
            Self::ConflictingCandidate => {
                formatter.write_str("MINT identity is bound to a different cause")
            }
            Self::Unavailable(error) => {
                write!(formatter, "item MINT storage is unavailable: {error:?}")
            }
        }
    }
}

impl std::error::Error for ItemMintError {}

/// Owned copy of the frozen candidate moved into one database pass.
struct FrozenMint {
    request: ItemMintRequest,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    envelope: Vec<u8>,
    envelope_sha256: [u8; 32],
    intent_binding: [u8; 33],
}

impl FrozenMint {
    fn of(candidate: &ItemMintCandidate) -> Self {
        Self {
            request: candidate.request.clone(),
            transaction_id: candidate.transaction_id,
            event_id: candidate.event_id,
            item_instance_id: candidate.item_instance_id,
            occurred_at_unix_ms: candidate.occurred_at_unix_ms,
            envelope: candidate.envelope.clone(),
            envelope_sha256: candidate.envelope_sha256,
            intent_binding: candidate.intent_binding,
        }
    }
}

impl DurabilityRoot {
    /// Validate every registered bound and reserve the logical MINT
    /// transaction of this cause, or resume its existing reservation. A new
    /// reservation is created only while the death's ownership generation is
    /// the current assignment held by `node`'s current incarnation; it binds
    /// producer-owned UUIDv7 identities, the trusted database timestamp, the
    /// exact event bytes, that fence and a zero DUR03-RL-08 budget, and commits
    /// before any commit pass. An existing reservation is resumed with the same
    /// identities and budget: while no receipt exists its fence must still be
    /// live for `node` (D52), and a terminal receipt resumes regardless so the
    /// original result stays reachable. This grants nothing and mints nothing.
    pub async fn freeze_item_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ItemMintRequest,
    ) -> Result<ItemMintCandidate> {
        self.freeze_mint(authority, node, request, None, false)
            .await
    }

    /// Freeze one loot MINT into a corpse container (DUR-03 §39.4, D3-2),
    /// exactly like [`Self::freeze_item_mint`] except for its destination: a
    /// new reservation is created only while the parent is a live corpse
    /// `ItemInstance` of the same death. Commit it with
    /// [`Self::commit_corpse_loot_mint`].
    pub async fn freeze_corpse_loot_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: CorpseLootMintRequest,
    ) -> Result<ItemMintCandidate> {
        let CorpseLootMintRequest {
            cause,
            item,
            quantity,
            placement,
            content_revision,
            ruleset_revision,
            sim_revision,
        } = request;
        let request = ItemMintRequest {
            cause,
            item,
            quantity,
            ground: GroundPlacement {
                spatial_position: Vec::new(),
                corpse_ref: Vec::new(),
                map_revision: String::new(),
                content_revision: String::new(),
                native_room_placement_context: Vec::new(),
            },
            content_revision,
            ruleset_revision,
            sim_revision,
        };
        self.freeze_mint(authority, node, request, Some(placement), false)
            .await
    }

    /// Reserve one entry for an atomic kill batch. Its parent is a candidate in that batch;
    /// this grants no custody and the batch commit re-proves the live writer and ancestry.
    pub async fn freeze_kill_loot_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: CorpseLootMintRequest,
    ) -> Result<ItemMintCandidate> {
        let placement = request.placement;
        let request = ItemMintRequest {
            cause: request.cause,
            item: request.item,
            quantity: request.quantity,
            ground: GroundPlacement {
                spatial_position: Vec::new(),
                corpse_ref: Vec::new(),
                map_revision: String::new(),
                content_revision: String::new(),
                native_room_placement_context: Vec::new(),
            },
            content_revision: request.content_revision,
            ruleset_revision: request.ruleset_revision,
            sim_revision: request.sim_revision,
        };
        self.freeze_mint(authority, node, request, Some(placement), true)
            .await
    }

    #[cfg(test)]
    pub(super) async fn freeze_item_mint_legacy_instance_for_test(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ItemMintRequest,
    ) -> Result<ItemMintCandidate> {
        if native_instance_corpse_ref(request.cause.death, false).as_deref()
            != Some(request.ground.corpse_ref.as_slice())
        {
            return Err(ItemMintError::InvalidInput);
        }
        self.freeze_mint_format(
            authority,
            node,
            request,
            None,
            false,
            NewMintFormat::LegacyInstanceV1,
        )
        .await
    }

    #[cfg(test)]
    pub(super) async fn freeze_corpse_loot_mint_legacy_instance_for_test(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: CorpseLootMintRequest,
    ) -> Result<ItemMintCandidate> {
        if request.cause.death.runtime_scope.instance_id().is_none() {
            return Err(ItemMintError::InvalidInput);
        }
        let CorpseLootMintRequest {
            cause,
            item,
            quantity,
            placement,
            content_revision,
            ruleset_revision,
            sim_revision,
        } = request;
        let request = ItemMintRequest {
            cause,
            item,
            quantity,
            ground: GroundPlacement {
                spatial_position: Vec::new(),
                corpse_ref: Vec::new(),
                map_revision: String::new(),
                content_revision: String::new(),
                native_room_placement_context: Vec::new(),
            },
            content_revision,
            ruleset_revision,
            sim_revision,
        };
        self.freeze_mint_format(
            authority,
            node,
            request,
            Some(placement),
            false,
            NewMintFormat::LegacyInstanceV1,
        )
        .await
    }

    async fn freeze_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ItemMintRequest,
        corpse_entry: Option<CorpseContainerPlacement>,
        pending_parent: bool,
    ) -> Result<ItemMintCandidate> {
        self.freeze_mint_format(
            authority,
            node,
            request,
            corpse_entry,
            pending_parent,
            NewMintFormat::Canonical,
        )
        .await
    }

    async fn freeze_mint_format(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ItemMintRequest,
        corpse_entry: Option<CorpseContainerPlacement>,
        pending_parent: bool,
        format: NewMintFormat,
    ) -> Result<ItemMintCandidate> {
        validate_request(&request, corpse_entry.as_ref())?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        let node = node.clone();
        let (request, intent_binding, reservation) = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, &request.cause).await?;
                    if let Some(row) = load_reservation(&mut tx, &request.cause).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        let (request, intent_binding) =
                            match replay_native_request(request, corpse_entry.as_ref(), &stored) {
                                Ok(selected) => selected,
                                Err(error) => return Ok(Err(error)),
                            };
                        let reservation = decode_reservation(&row)?;
                        let terminal = load_receipt(&mut tx, &request.cause).await?.is_some();
                        let resumable = terminal
                            || (fence_matches(&reservation, &node)
                                && fence_is_live(&mut tx, &request.cause.death, &node).await?);
                        if !resumable {
                            return Ok(Err(ItemMintError::AuthorityRejected));
                        }
                        tx.commit(deadline).await?;
                        return Ok(Ok((request, intent_binding, reservation)));
                    }

                    let (request, encx_v2) = match format {
                        NewMintFormat::Canonical => {
                            let request = canonical_native_request(request, corpse_entry.as_ref());
                            let encx_v2 = matches!(
                                request.cause.death.runtime_scope,
                                RuntimeScopeRefV1::Instance { .. }
                            );
                            (request, encx_v2)
                        }
                        #[cfg(test)]
                        NewMintFormat::LegacyInstanceV1 => (request, false),
                    };
                    let intent_binding =
                        match intent_binding_format(&request, corpse_entry.as_ref(), encx_v2) {
                            Ok(binding) => binding,
                            Err(error) => return Ok(Err(error)),
                        };
                    if !fence_is_live(&mut tx, &request.cause.death, &node).await? {
                        return Ok(Err(ItemMintError::AuthorityRejected));
                    }
                    if let Some(entry) = &corpse_entry
                        && !pending_parent
                        && !corpse_parent_is_live(&mut tx, &request.cause.death, entry).await?
                    {
                        return Ok(Err(ItemMintError::InvalidInput));
                    }
                    let row = sqlx::query(
                        "SELECT game_character_uuid_v7()::text AS transaction_id, \
                                game_character_uuid_v7()::text AS event_id, \
                                game_character_uuid_v7()::text AS item_instance_id, \
                                floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                                  AS occurred_at",
                    )
                    .fetch_one(&mut **tx)
                    .await?;
                    let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
                    let event_id = uuid_text(row.try_get("event_id")?)?;
                    let item_instance_id = uuid_text(row.try_get("item_instance_id")?)?;
                    let occurred_at_unix_ms = row.try_get::<i64, _>("occurred_at")?;
                    let tuple = tx.tuple();
                    let envelope = match audit::encode_mint_event(
                        MintEventIdentity {
                            event_id,
                            transaction_id,
                            occurred_at_unix_ms,
                            server_build_id: SERVER_BUILD_ID,
                        },
                        mint_message(&request, corpse_entry.as_ref(), item_instance_id),
                        tuple.tuple(),
                    ) {
                        Ok(envelope) => envelope,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    let fact = node.fact();
                    let reservation = Reservation {
                        transaction_id,
                        event_id,
                        item_instance_id,
                        occurred_at_unix_ms,
                        envelope,
                        fence_node_id: *fact.node_id().as_bytes(),
                        fence_registration_revision: fact.registration_revision(),
                        work_units_used: 0,
                    };
                    insert_reservation(&mut tx, &request, &intent_binding, &reservation, tuple)
                        .await?;
                    tx.commit(deadline).await?;
                    Ok(Ok((request, intent_binding, reservation)))
                })
            })
            .await??;
        let envelope_sha256 = Sha256::digest(&reservation.envelope).into();
        Ok(ItemMintCandidate {
            request,
            corpse_entry,
            transaction_id: reservation.transaction_id,
            event_id: reservation.event_id,
            item_instance_id: reservation.item_instance_id,
            occurred_at_unix_ms: reservation.occurred_at_unix_ms,
            envelope: reservation.envelope,
            envelope_sha256,
            intent_binding,
            work_units_used: reservation.work_units_used,
        })
    }

    /// Durably charge one DUR03-RL-08 work unit to the candidate's reservation
    /// in its own committed transaction, before the pass does any work. The
    /// conditional UPDATE takes the row lock and re-checks the budget, so
    /// concurrent holders of the same cause never exceed the maximum.
    async fn charge_item_mint_work_unit(
        &self,
        recovery: CharacterRecoveryFenceV1,
        candidate: &mut ItemMintCandidate,
    ) -> Result<()> {
        if candidate.work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
            return Err(ItemMintError::CapacityExceeded);
        }
        let cause = candidate.request.cause.clone();
        let transaction_id = candidate.transaction_id;
        let charged = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let used = bind_cause!(
                        sqlx::query(concat!(
                            "UPDATE game_item_mint_reservations \
                                SET work_units_used = work_units_used + 1 WHERE ",
                            cause_key!(),
                            " AND transaction_id = encode($11,'hex')::uuid \
                              AND work_units_used < $12 RETURNING work_units_used"
                        )),
                        &cause
                    )
                    .bind(transaction_id.as_slice())
                    .bind(i16::from(RL08_RETRY_WORK_UNITS_MAX))
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(used) = used else {
                        let reserved = bind_cause!(
                            sqlx::query(concat!(
                                "SELECT 1 FROM game_item_mint_reservations WHERE ",
                                cause_key!(),
                                " AND transaction_id = encode($11,'hex')::uuid"
                            )),
                            &cause
                        )
                        .bind(transaction_id.as_slice())
                        .fetch_optional(&mut *tx)
                        .await?;
                        return Ok(Err(if reserved.is_some() {
                            ItemMintError::CapacityExceeded
                        } else {
                            ItemMintError::ConflictingCandidate
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
            Err(ItemMintError::CapacityExceeded) => {
                candidate.work_units_used = RL08_RETRY_WORK_UNITS_MAX;
                Err(ItemMintError::CapacityExceeded)
            }
            Err(error) => Err(error),
        }
    }

    /// Commit one frozen MINT candidate. A cause that already minted returns
    /// its original terminal result without reacquiring runtime authority; a
    /// changed intent for the same cause conflicts. A new cause commits only
    /// while its reservation's fence is live: the death's ownership generation
    /// is the current assignment held by the reserving node's current
    /// incarnation, which `node` must prove.
    pub async fn commit_item_mint(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
    ) -> Result<ItemMintOutcome> {
        // A corpse-loot candidate commits only through its own path: this one
        // writes a Ground location.
        if candidate.corpse_entry.is_some() {
            return Err(ItemMintError::InvalidInput);
        }
        // SPELL-LOCK-2 §1.2: the Ground (or corpse entry) this writes takes advisory key 33,
        // so the lane must be the death's exact scope, held to the end of the transaction.
        permit.check_scope(candidate.request.cause.death.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        self.charge_item_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let frozen = FrozenMint::of(candidate);
        let node = node.clone();
        let mut context = permit;

        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let permit: &SpellLanePermit = permit;
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, &frozen.request.cause).await?;

                    if let Some(row) = load_receipt(&mut tx, &frozen.request.cause).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != frozen.intent_binding {
                            return Ok(Err(ItemMintError::ConflictingCause));
                        }
                        let committed = decode_receipt(&row)?;
                        tx.commit(deadline).await?;
                        return Ok(Ok(ItemMintOutcome::AlreadyCommitted(committed)));
                    }

                    let Some(row) = load_reservation(&mut tx, &frozen.request.cause).await? else {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != frozen.intent_binding {
                        return Ok(Err(ItemMintError::ConflictingCause));
                    }
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != frozen.transaction_id
                        || reservation.event_id != frozen.event_id
                        || reservation.item_instance_id != frozen.item_instance_id
                    {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    }

                    if mint_identity_reused(&mut tx, &frozen).await? {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    }

                    // DUR-03 §32 / D52: the reservation's fence must be live:
                    // the death's generation is the current assignment held by
                    // the reserving node's current incarnation.
                    if !fence_matches(&reservation, &node)
                        || !fence_is_live(&mut tx, &frozen.request.cause.death, &node).await?
                    {
                        return Ok(Err(ItemMintError::AuthorityRejected));
                    }

                    // Capacity is checked only for a fresh Ground root, after the
                    // historical receipt and the real current writer fence above.
                    let death = &frozen.request.cause.death;
                    if let Err(error) =
                        super::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
                            &mut tx,
                            permit,
                            &recovery,
                            &node,
                            (death.runtime_scope, death.scope_ownership_generation.get()),
                            &frozen.request.ground,
                            &frozen.item_instance_id,
                        )
                        .await
                    {
                        return Ok(Err(error.into_item_mint_error()));
                    }

                    let tuple = match tx.frozen_tuple(&frozen.envelope) {
                        Ok(tuple) => tuple,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    insert_mint(&mut tx, permit, &frozen, tuple).await?;
                    let committed = CommittedItemMint {
                        transaction_id: frozen.transaction_id,
                        event_id: frozen.event_id,
                        item_instance_id: frozen.item_instance_id,
                        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
                        envelope_sha256: frozen.envelope_sha256,
                    };
                    tx.commit(deadline).await?;
                    Ok(Ok(ItemMintOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Commit one frozen corpse-MINT candidate (DUR-03 §39.4, child `D3-1`).
    /// The candidate is frozen exactly like any other MINT
    /// ([`Self::freeze_item_mint`]); this commit path differs from
    /// [`Self::commit_item_mint`] only in what it is admitted for and what it
    /// writes:
    /// - admitted only for a candidate whose cause carries the reserved
    ///   [`CORPSE_MATERIALIZATION_PURPOSE_KEY`] (D3 §4.1); any other cause is
    ///   `InvalidInput`, never silently handled as an ordinary Ground MINT;
    /// - the receipt additionally carries the corpse's captured top-damage
    ///   `CharacterId` (D132/§4.3), a value no other MINT shape ever writes;
    /// - `materialized_at` is never written here: only the deferred
    ///   `game_item_mint_receipt_materialize_corpse` trigger (migration 0013)
    ///   sets it, from `clock_timestamp()`, immediately before this
    ///   transaction's own commit finalizes (D133/D135);
    /// - before the existing `fence_is_live` check and before this MINT's own
    ///   insert, it takes the per-scope `oteryn:corpse-cap:` advisory
    ///   transaction lock and recounts live corpses for the death's scope,
    ///   refusing `CapacityExceeded` at or above the already-accepted
    ///   `COMBAT01-CORPSES-PER-SCOPE` = 64 ceiling (VSL resource-rows decision
    ///   §4.1 row 6; D3 §4.2) -- never a freeze-time-only count, so two
    ///   concurrent corpse commits for one scope can never both observe room
    ///   and both succeed.
    pub async fn commit_corpse_mint(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
        top_damage_character_id: [u8; 16],
    ) -> Result<ItemMintOutcome> {
        self.commit_corpse_mint_inner(
            permit,
            authority,
            node,
            candidate,
            top_damage_character_id,
            None,
        )
        .await
    }

    /// Boss clock transition and corpse receipt share the same fenced transaction.
    pub async fn commit_boss_corpse_mint(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
        top_damage_character_id: [u8; 16],
        boss: (String, u64, oteryn_gameplay_content::content::BossPolicy),
    ) -> Result<ItemMintOutcome> {
        self.commit_corpse_mint_inner(
            permit,
            authority,
            node,
            candidate,
            top_damage_character_id,
            Some(boss),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn commit_corpse_mint_inner(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
        top_damage_character_id: [u8; 16],
        boss: Option<(String, u64, oteryn_gameplay_content::content::BossPolicy)>,
    ) -> Result<ItemMintOutcome> {
        let mut outcomes = self
            .commit_kill_loot_mints(
                permit,
                authority,
                node,
                candidate,
                &mut [],
                top_damage_character_id,
                boss,
            )
            .await?;
        outcomes.pop().ok_or(ItemMintError::InvalidInput)
    }

    /// Atomically materialize a corpse and its flattened, parent-before-child loot tree.
    /// Every candidate keeps its own cause, reservation, audit event and retry budget.
    #[allow(
        clippy::too_many_arguments,
        reason = "the atomic kill retains the existing corpse writer's fence and boss facts"
    )]
    pub async fn commit_kill_loot_mints(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
        children: &mut [ItemMintCandidate],
        top_damage_character_id: [u8; 16],
        boss: Option<(String, u64, oteryn_gameplay_content::content::BossPolicy)>,
    ) -> Result<Vec<ItemMintOutcome>> {
        self.commit_kill_loot_mints_inner(
            permit,
            authority,
            node,
            candidate,
            children,
            Some(top_damage_character_id),
            boss,
        )
        .await
    }

    /// Internal physical writer port. Its sole source-event facade validates
    /// the opaque uncredited death completion before reaching this method.
    pub(super) async fn commit_uncredited_corpse_mint_inner(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
    ) -> Result<ItemMintOutcome> {
        let mut result = self
            .commit_kill_loot_mints_inner(permit, authority, node, candidate, &mut [], None, None)
            .await?;
        result.pop().ok_or(ItemMintError::InvalidInput)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one existing physical writer preserves credited and sealed uncredited source deaths"
    )]
    async fn commit_kill_loot_mints_inner(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
        children: &mut [ItemMintCandidate],
        top_damage_character_id: Option<[u8; 16]>,
        boss: Option<(String, u64, oteryn_gameplay_content::content::BossPolicy)>,
    ) -> Result<Vec<ItemMintOutcome>> {
        if candidate.request.cause.purpose_key != CORPSE_MATERIALIZATION_PURPOSE_KEY {
            return Err(ItemMintError::InvalidInput);
        }
        // D3 §4.1: the corpse's own MINT is always draw_ordinal = 0, the
        // reserved sentinel ordinal no ordinary loot entry's own cause ever
        // uses; the DB's own CHECK (migration 0013) enforces this too.
        if candidate.request.cause.draw_ordinal != 0 || candidate.corpse_entry.is_some() {
            return Err(ItemMintError::InvalidInput);
        }
        if boss.is_some()
            && candidate
                .request
                .cause
                .death
                .runtime_scope
                .channel_id()
                .is_none()
        {
            return Err(ItemMintError::InvalidInput);
        }
        if let Some(owner) = top_damage_character_id {
            audit::check_uuid_v7(&owner)?;
        } else if !children.is_empty() || boss.is_some() {
            return Err(ItemMintError::InvalidInput);
        }
        // SPELL-LOCK-2 §1.2: the Ground (or corpse entry) this writes takes advisory key 33,
        // so the lane must be the death's exact scope, held to the end of the transaction.
        permit.check_scope(candidate.request.cause.death.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        self.charge_item_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let mut parents = vec![candidate.item_instance_id];
        let mut frozen_children = Vec::with_capacity(children.len());
        if children.len() > 4096 {
            return Err(ItemMintError::CapacityExceeded);
        }
        for child in children {
            let placement = child.corpse_entry.ok_or(ItemMintError::InvalidInput)?;
            if child.request.cause.death != candidate.request.cause.death
                || !parents.contains(&placement.corpse_item_instance_id)
            {
                return Err(ItemMintError::InvalidInput);
            }
            self.charge_item_mint_work_unit(recovery.clone(), child)
                .await?;
            frozen_children.push((FrozenMint::of(child), placement));
            parents.push(child.item_instance_id);
        }
        let frozen = FrozenMint::of(candidate);
        let node = node.clone();
        let mut context = permit;

        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let permit: &SpellLanePermit = permit;
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, &frozen.request.cause).await?;

                    if let Some(row) = load_receipt(&mut tx, &frozen.request.cause).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != frozen.intent_binding {
                            return Ok(Err(ItemMintError::ConflictingCause));
                        }
                        // D132/§4.3: the committed top-damage CharacterId is
                        // part of this corpse's frozen intent even though it
                        // is bound only at commit; a replay carrying another
                        // winner must never silently return the first
                        // winner's outcome as its own.
                        let stored_top_damage: Option<String> =
                            row.try_get("corpse_top_damage_character_id")?;
                        let stored_top_damage = stored_top_damage
                            .map(|value| uuid_text(&value))
                            .transpose()?;
                        if stored_top_damage != top_damage_character_id {
                            return Ok(Err(ItemMintError::ConflictingCause));
                        }
                        let committed = decode_receipt(&row)?;
                        let mut outcomes = vec![ItemMintOutcome::AlreadyCommitted(committed)];
                        for (child, _) in &frozen_children {
                            let Some(row) = load_receipt(&mut tx, &child.request.cause).await?
                            else {
                                return Ok(Err(ItemMintError::ConflictingCandidate));
                            };
                            if row.try_get::<Vec<u8>, _>("intent_binding")? != child.intent_binding
                            {
                                return Ok(Err(ItemMintError::ConflictingCause));
                            }
                            outcomes.push(ItemMintOutcome::AlreadyCommitted(decode_receipt(&row)?));
                        }
                        tx.commit(deadline).await?;
                        return Ok(Ok(outcomes));
                    }

                    let Some(row) = load_reservation(&mut tx, &frozen.request.cause).await? else {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != frozen.intent_binding {
                        return Ok(Err(ItemMintError::ConflictingCause));
                    }
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != frozen.transaction_id
                        || reservation.event_id != frozen.event_id
                        || reservation.item_instance_id != frozen.item_instance_id
                    {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    }

                    if mint_identity_reused(&mut tx, &frozen).await? {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    }

                    // D3 §4.2: the per-scope corpse-cap advisory lock, taken
                    // before fence_is_live and before this MINT's own insert
                    // -- never a freeze-time-only count.
                    let live_corpses =
                        corpse_cap_recount(&mut tx, &frozen.request.cause.death).await?;
                    if live_corpses >= COMBAT01_CORPSES_PER_SCOPE_MAX {
                        return Ok(Err(ItemMintError::CapacityExceeded));
                    }

                    // DUR-03 §32 / D52: the reservation's fence must be live:
                    // the death's generation is the current assignment held by
                    // the reserving node's current incarnation.
                    if !fence_matches(&reservation, &node)
                        || !fence_is_live(&mut tx, &frozen.request.cause.death, &node).await?
                    {
                        return Ok(Err(ItemMintError::AuthorityRejected));
                    }

                    // Capacity is checked only for a fresh Ground root, after the
                    // historical receipt and the real current writer fence above.
                    let death = &frozen.request.cause.death;
                    if let Err(error) =
                        super::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
                            &mut tx,
                            permit,
                            &recovery,
                            &node,
                            (death.runtime_scope, death.scope_ownership_generation.get()),
                            &frozen.request.ground,
                            &frozen.item_instance_id,
                        )
                        .await
                    {
                        return Ok(Err(error.into_item_mint_error()));
                    }

                    let tuple = match tx.frozen_tuple(&frozen.envelope) {
                        Ok(tuple) => tuple,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    match top_damage_character_id {
                        Some(owner) => {
                            insert_corpse_mint(&mut tx, permit, &frozen, tuple, owner).await?
                        }
                        None => {
                            insert_mint_with_corpse_attribution(
                                &mut tx, permit, &frozen, tuple, None,
                            )
                            .await?
                        }
                    }
                    if let Some((spawn, cycle, policy)) = &boss {
                        let death = &frozen.request.cause.death;
                        let key = super::boss_encounter::BossClockKey {
                            world: death.world_id,
                            channel: death
                                .runtime_scope
                                .channel_id()
                                .ok_or(DurabilityError::InvalidStoredState)?,
                            spawn: spawn.clone(),
                        };
                        super::boss_encounter::advance_death_in_transaction(
                            &mut tx,
                            &key,
                            death.scope_ownership_generation,
                            *cycle,
                            *policy,
                        )
                        .await
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    }
                    let mut child_outcomes = Vec::with_capacity(frozen_children.len());
                    for (child, placement) in &frozen_children {
                        lock_cause(&mut tx, &child.request.cause).await?;
                        let Some(row) = load_reservation(&mut tx, &child.request.cause).await?
                        else {
                            return Ok(Err(ItemMintError::ConflictingCandidate));
                        };
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != child.intent_binding {
                            return Ok(Err(ItemMintError::ConflictingCause));
                        }
                        let reservation = decode_reservation(&row)?;
                        if reservation.transaction_id != child.transaction_id
                            || reservation.event_id != child.event_id
                            || reservation.item_instance_id != child.item_instance_id
                        {
                            return Ok(Err(ItemMintError::ConflictingCandidate));
                        }
                        if mint_identity_reused(&mut tx, child).await? {
                            return Ok(Err(ItemMintError::ConflictingCandidate));
                        }

                        // DUR-03 §32 / D52: the reservation's fence must be live.
                        if !fence_matches(&reservation, &node)
                            || !fence_is_live(&mut tx, &child.request.cause.death, &node).await?
                        {
                            return Ok(Err(ItemMintError::AuthorityRejected));
                        }

                        let tuple = match tx.frozen_tuple(&child.envelope) {
                            Ok(tuple) => tuple,
                            Err(error) => return Ok(Err(error.into())),
                        };
                        if placement.corpse_item_instance_id == frozen.item_instance_id {
                            insert_corpse_loot_mint(&mut tx, permit, child, tuple, placement)
                                .await?;
                        } else {
                            insert_nested_loot_mint(&mut tx, child, tuple, placement).await?;
                        }
                        child_outcomes.push(ItemMintOutcome::Committed(CommittedItemMint {
                            transaction_id: child.transaction_id,
                            event_id: child.event_id,
                            item_instance_id: child.item_instance_id,
                            occurred_at_unix_ms: child.occurred_at_unix_ms,
                            envelope_sha256: child.envelope_sha256,
                        }));
                    }
                    let committed = CommittedItemMint {
                        transaction_id: frozen.transaction_id,
                        event_id: frozen.event_id,
                        item_instance_id: frozen.item_instance_id,
                        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
                        envelope_sha256: frozen.envelope_sha256,
                    };
                    tx.commit(deadline).await?;
                    let mut outcomes = vec![ItemMintOutcome::Committed(committed)];
                    outcomes.extend(child_outcomes);
                    Ok(Ok(outcomes))
                })
            })
            .await?
    }

    /// Commit one frozen corpse-loot MINT (DUR-03 §39.4, child `D3-2`): the
    /// loot item is established as a fresh `Container(parent = the death's
    /// corpse)` entry, never on Ground and never through a TRANSFER. It
    /// differs from [`Self::commit_item_mint`] only in what it is admitted for
    /// and what it writes:
    /// - admitted only for a candidate frozen by
    ///   [`Self::freeze_corpse_loot_mint`];
    /// - the parent must be a live corpse `ItemInstance` of the same death
    ///   (also enforced by the migration 0013 consistency guard), else
    ///   `InvalidInput`;
    /// - the `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` ceiling and the
    ///   corpse-row lock serializing concurrent entries are the migration's
    ///   deferred `game_item_corpse_container_entry_proven` trigger;
    /// - the D52 fence is checked exactly as for every other MINT: a death
    ///   whose ownership generation ended is refused, never re-reserved.
    pub async fn commit_corpse_loot_mint(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
    ) -> Result<ItemMintOutcome> {
        let Some(placement) = candidate.corpse_entry else {
            return Err(ItemMintError::InvalidInput);
        };
        // SPELL-LOCK-2 §1.2: the Ground (or corpse entry) this writes takes advisory key 33,
        // so the lane must be the death's exact scope, held to the end of the transaction.
        permit.check_scope(candidate.request.cause.death.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        self.charge_item_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let frozen = FrozenMint::of(candidate);
        let node = node.clone();
        let mut context = permit;

        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let permit: &SpellLanePermit = permit;
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, &frozen.request.cause).await?;

                    if let Some(row) = load_receipt(&mut tx, &frozen.request.cause).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != frozen.intent_binding {
                            return Ok(Err(ItemMintError::ConflictingCause));
                        }
                        let committed = decode_receipt(&row)?;
                        tx.commit(deadline).await?;
                        return Ok(Ok(ItemMintOutcome::AlreadyCommitted(committed)));
                    }

                    let Some(row) = load_reservation(&mut tx, &frozen.request.cause).await? else {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != frozen.intent_binding {
                        return Ok(Err(ItemMintError::ConflictingCause));
                    }
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != frozen.transaction_id
                        || reservation.event_id != frozen.event_id
                        || reservation.item_instance_id != frozen.item_instance_id
                    {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    }
                    if mint_identity_reused(&mut tx, &frozen).await? {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    }

                    // DUR-03 §32 / D52: the reservation's fence must be live.
                    if !fence_matches(&reservation, &node)
                        || !fence_is_live(&mut tx, &frozen.request.cause.death, &node).await?
                    {
                        return Ok(Err(ItemMintError::AuthorityRejected));
                    }
                    if !corpse_parent_is_live(&mut tx, &frozen.request.cause.death, &placement)
                        .await?
                    {
                        return Ok(Err(ItemMintError::InvalidInput));
                    }

                    let tuple = match tx.frozen_tuple(&frozen.envelope) {
                        Ok(tuple) => tuple,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    insert_corpse_loot_mint(&mut tx, permit, &frozen, tuple, &placement).await?;
                    let committed = CommittedItemMint {
                        transaction_id: frozen.transaction_id,
                        event_id: frozen.event_id,
                        item_instance_id: frozen.item_instance_id,
                        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
                        envelope_sha256: frozen.envelope_sha256,
                    };
                    tx.commit(deadline).await?;
                    Ok(Ok(ItemMintOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an unknown outcome of a frozen candidate. The cause lock waits
    /// for any in-flight attempt, so `None` proves nothing committed for this
    /// cause and the same candidate may be retried. Never reacquires runtime
    /// authority.
    pub async fn reconcile_item_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &mut ItemMintCandidate,
    ) -> Result<Option<CommittedItemMint>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        self.charge_item_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let cause = candidate.request.cause.clone();
        let binding = candidate.intent_binding;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, &cause).await?;
                    let Some(row) = load_receipt(&mut tx, &cause).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != binding {
                        return Ok(Err(ItemMintError::ConflictingCause));
                    }
                    let committed = decode_receipt(&row)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// Read one durable Ground ItemInstance.
    pub async fn read_item_instance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        item_instance_id: [u8; 16],
    ) -> Result<Option<GroundItemInstance>> {
        audit::check_uuid_v7(&item_instance_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT i.item_instance_id::text, i.world_id::text, g.channel_id::text, g.scope_kind, g.instance_id::text, \
                                i.definition_family, i.definition_production_key, \
                                i.definition_revision_ref, i.quantity, \
                                i.minted_transaction_id::text, \
                                g.runtime_scope_ownership_generation::text, g.spatial_position, \
                                g.corpse_ref, g.map_revision, g.content_revision, \
                                g.native_room_placement_context \
                           FROM game_item_instances i \
                           JOIN game_item_ground_locations g USING (item_instance_id, world_id) \
                          WHERE i.item_instance_id = encode($1,'hex')::uuid",
                    )
                    .bind(item_instance_id.as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let item = row.as_ref().map(decode_instance).transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(item))
                })
            })
            .await?
    }
}

fn validate_request(
    request: &ItemMintRequest,
    corpse_entry: Option<&CorpseContainerPlacement>,
) -> Result<()> {
    let cause = &request.cause;
    validate_definition(&cause.loot_table)?;
    audit::check_content_key(&cause.purpose_key)?;
    validate_definition(&request.item)?;
    if let Some(entry) = corpse_entry {
        // The corpse's own MINT is a Ground MINT (D3 §4.1); a loot entry never
        // carries its reserved cause.
        audit::check_uuid_v7(&entry.corpse_item_instance_id)?;
        if cause.purpose_key == CORPSE_MATERIALIZATION_PURPOSE_KEY
            || !(1..=CORPSE_CONTAINER_ENTRIES_MAX).contains(&entry.placement_ordinal)
        {
            return Err(ItemMintError::InvalidInput);
        }
    } else {
        let ground = &request.ground;
        audit::check_technical_bytes(&ground.spatial_position)?;
        audit::check_technical_bytes(&ground.corpse_ref)?;
        audit::check_content_key(&ground.map_revision)?;
        audit::check_content_key(&ground.content_revision)?;
        audit::check_technical_bytes(&ground.native_room_placement_context)?;
    }
    audit::check_content_key(&request.content_revision)?;
    audit::check_content_key(&request.ruleset_revision)?;
    audit::check_content_key(&request.sim_revision)?;
    if request.quantity == 0
        || cause.death.actor_local_generation == 0
        || cause.death.world_id != cause.death.runtime_scope.world_id()
    {
        return Err(ItemMintError::InvalidInput);
    }
    audit::TransactionResourceUsage::MINT.check()?;
    Ok(())
}

fn validate_definition(value: &TypedDefinitionRef) -> Result<()> {
    audit::check_technical_text(&value.family)?;
    audit::check_content_key(&value.production_key)?;
    audit::check_content_key(&value.revision_ref)?;
    Ok(())
}

fn push_text(out: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length = u16::try_from(value.len()).map_err(|_| ItemMintError::InvalidInput)?;
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(value);
    Ok(())
}

fn push_definition(out: &mut Vec<u8>, value: &TypedDefinitionRef) -> Result<()> {
    push_text(out, value.family.as_bytes())?;
    push_text(out, value.production_key.as_bytes())?;
    push_text(out, value.revision_ref.as_bytes())
}

/// Digest of the complete business intent. Frozen identities and the trusted
/// timestamp are excluded, so a re-frozen candidate of the same intent
/// resolves to the original result instead of conflicting.
// Unified native Instance writes use ENCX b5103e7f v2/53-byte framing; retained
// SRVENC v1/61-byte requests remain exact replays. Channel bytes never change.
// Only the complete typed native death may
// translate framing; an arbitrary opaque corpse_ref never acquires another meaning.
fn native_instance_corpse_ref(death: CreatureDeathKey, encx: bool) -> Option<Vec<u8>> {
    let RuntimeScopeRefV1::Instance { instance_id, .. } = death.runtime_scope else {
        return None;
    };
    let mut bytes = Vec::with_capacity(if encx { 53 } else { 61 });
    if encx {
        bytes.push(2);
    }
    bytes.extend_from_slice(death.world_id.as_bytes());
    if !encx {
        bytes.extend_from_slice(b"instance:");
    }
    bytes.extend_from_slice(&instance_id);
    bytes.extend_from_slice(&death.scope_ownership_generation.get().to_be_bytes());
    bytes.extend_from_slice(&death.actor_local_id.to_be_bytes());
    bytes.extend_from_slice(&death.actor_local_generation.to_be_bytes());
    Some(bytes)
}

fn canonical_native_request(
    mut request: ItemMintRequest,
    corpse_entry: Option<&CorpseContainerPlacement>,
) -> ItemMintRequest {
    if corpse_entry.is_none()
        && native_instance_corpse_ref(request.cause.death, false).as_deref()
            == Some(request.ground.corpse_ref.as_slice())
        && let Some(canonical) = native_instance_corpse_ref(request.cause.death, true)
    {
        request.ground.corpse_ref = canonical;
    }
    request
}

fn replay_native_request(
    request: ItemMintRequest,
    corpse_entry: Option<&CorpseContainerPlacement>,
    stored: &[u8],
) -> Result<(ItemMintRequest, [u8; 33])> {
    let canonical = intent_binding(&request, corpse_entry)?;
    if stored == canonical {
        return Ok((request, canonical));
    }
    // Preserve every exact accepted SRVENC v1 input before translating native
    // framing, including opaque technical placement bytes. Full hash equality
    // proves all business fields; this does not reinterpret opaque corpse_ref.
    if matches!(
        request.cause.death.runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ) {
        let legacy = intent_binding_format(&request, corpse_entry, false)?;
        if stored == legacy {
            return Ok((request, legacy));
        }
    }
    let request = canonical_native_request(request, corpse_entry);
    let canonical = intent_binding(&request, corpse_entry)?;
    if stored == canonical {
        return Ok((request, canonical));
    }
    let Some(native) = native_instance_corpse_ref(request.cause.death, false) else {
        return Err(ItemMintError::ConflictingCause);
    };
    let mut alternate = request;
    if corpse_entry.is_none() {
        if native_instance_corpse_ref(alternate.cause.death, true).as_deref()
            != Some(alternate.ground.corpse_ref.as_slice())
        {
            return Err(ItemMintError::ConflictingCause);
        }
        alternate.ground.corpse_ref = native;
    }
    let binding = intent_binding_format(&alternate, corpse_entry, false)?;
    // Both retained 33-byte bindings begin with 1. Only this full semantic hash
    // proves compatibility; never accept a prefix, missing field or scope alias.
    if stored != binding {
        return Err(ItemMintError::ConflictingCause);
    }
    Ok((alternate, binding))
}

fn intent_binding(
    request: &ItemMintRequest,
    corpse_entry: Option<&CorpseContainerPlacement>,
) -> Result<[u8; 33]> {
    intent_binding_format(
        request,
        corpse_entry,
        matches!(
            request.cause.death.runtime_scope,
            RuntimeScopeRefV1::Instance { .. }
        ),
    )
}

fn intent_binding_format(
    request: &ItemMintRequest,
    corpse_entry: Option<&CorpseContainerPlacement>,
    encx_instance_v2: bool,
) -> Result<[u8; 33]> {
    let cause = &request.cause;
    let death = &cause.death;
    let mut semantic = vec![if encx_instance_v2 {
        INSTANCE_INTENT_BINDING_VERSION
    } else {
        INTENT_BINDING_VERSION
    }];
    semantic.extend_from_slice(death.world_id.as_bytes());
    if encx_instance_v2 {
        let RuntimeScopeRefV1::Instance { instance_id, .. } = death.runtime_scope else {
            return Err(ItemMintError::InvalidInput);
        };
        semantic.extend_from_slice(&instance_id);
    } else {
        death.append_scope_id(&mut semantic);
    }
    semantic.extend_from_slice(&death.scope_ownership_generation.get().to_be_bytes());
    semantic.extend_from_slice(&death.actor_local_id.to_be_bytes());
    semantic.extend_from_slice(&death.actor_local_generation.to_be_bytes());
    push_definition(&mut semantic, &cause.loot_table)?;
    push_text(&mut semantic, cause.purpose_key.as_bytes())?;
    semantic.extend_from_slice(&cause.draw_ordinal.to_be_bytes());
    push_definition(&mut semantic, &request.item)?;
    semantic.extend_from_slice(&request.quantity.to_be_bytes());
    let ground = &request.ground;
    // A Ground MINT keeps its original byte layout, so every existing
    // reservation and receipt binding stays valid.
    let placement_fields: Vec<&[u8]> = match corpse_entry {
        None => vec![
            ground.spatial_position.as_slice(),
            ground.corpse_ref.as_slice(),
            ground.map_revision.as_bytes(),
            ground.content_revision.as_bytes(),
            ground.native_room_placement_context.as_slice(),
        ],
        Some(entry) => {
            semantic.extend_from_slice(&entry.placement_ordinal.to_be_bytes());
            // W22a: a fluid joins the binding only when present, so every existing corpse-entry
            // reservation keeps its byte layout.
            if let Some(fluid) = entry.fluid_subtype {
                semantic.extend_from_slice(b"fluid");
                semantic.push(fluid);
            }
            vec![
                b"corpse-container-entry".as_slice(),
                entry.corpse_item_instance_id.as_slice(),
            ]
        }
    };
    for value in placement_fields.into_iter().chain([
        request.content_revision.as_bytes(),
        request.ruleset_revision.as_bytes(),
        request.sim_revision.as_bytes(),
        LOOT_MINT_TYPED_CAUSE.as_bytes(),
    ]) {
        push_text(&mut semantic, value)?;
    }
    // Absent provenance retains the byte layout of historical reservations.
    if let Some(race) = &cause.corpse_race {
        push_text(&mut semantic, b"corpse-source-race.v1")?;
        push_text(&mut semantic, race.key().as_bytes())?;
        push_text(&mut semantic, race.definition_revision().as_bytes())?;
    }
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = [0_u8; 33];
    binding[0] = INTENT_BINDING_VERSION;
    binding[1..].copy_from_slice(&digest);
    Ok(binding)
}

fn definition_message(value: &TypedDefinitionRef) -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: value.family.clone(),
        production_key: value.production_key.clone(),
        revision_ref: value.revision_ref.clone(),
    }
}

fn mint_message(
    request: &ItemMintRequest,
    corpse_entry: Option<&CorpseContainerPlacement>,
    item_instance_id: [u8; 16],
) -> OneItemMintV1 {
    let death = &request.cause.death;
    let world = death.world_id.as_bytes().to_vec();
    let channel = death.channel_bytes().unwrap_or_default();
    let generation = death.scope_ownership_generation.get();
    OneItemMintV1 {
        after: Some(OneItemStateV1 {
            item_instance_id: item_instance_id.to_vec(),
            world_id: world.clone(),
            definition: Some(definition_message(&request.item)),
            quantity: request.quantity,
            lifecycle: ITEM_LIFECYCLE_LIVE,
            key_number: 0,
        }),
        destination: corpse_entry.is_none().then(|| OneItemGroundV1 {
            instance_id: death.instance_bytes(),
            world_id: world.clone(),
            channel_id: channel.clone(),
            spatial_position: request.ground.spatial_position.clone(),
            corpse_ref: request.ground.corpse_ref.clone(),
            map_revision: request.ground.map_revision.clone(),
            content_revision: request.ground.content_revision.clone(),
            native_room_placement_context: request.ground.native_room_placement_context.clone(),
            runtime_scope_ownership_generation: generation,
        }),
        corpse_container_entry: corpse_entry.map(|entry| OneItemContainerEntryV1 {
            parent_item_instance_id: entry.corpse_item_instance_id.to_vec(),
            placement_ordinal: u64::from(entry.placement_ordinal),
        }),
        source: Some(OneItemProvenanceV1 {
            typed_cause: LOOT_MINT_TYPED_CAUSE.into(),
            death_occurrence: Some(CreatureDeathOccurrenceRefV1 {
                instance_id: death.instance_bytes(),
                world_id: world,
                channel_id: channel,
                scope_ownership_generation: generation,
                actor_local_id: death.actor_local_id,
                actor_local_generation: death.actor_local_generation,
            }),
            loot_table: Some(definition_message(&request.cause.loot_table)),
            loot_purpose_key: request.cause.purpose_key.clone(),
            draw_ordinal: request.cause.draw_ordinal,
            content_revision: request.content_revision.clone(),
            ruleset_revision: request.ruleset_revision.clone(),
            sim_revision: request.sim_revision.clone(),
        }),
        before_semantically_absent: true,
    }
}

/// Serializes attempts and reconciliation of one cause. The primary key on the
/// full cause, not this lock, is the uniqueness authority.
async fn lock_cause(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cause: &ItemMintCause,
) -> std::result::Result<(), DurabilityError> {
    let mut key = Vec::new();
    key.extend_from_slice(cause.death.world_id.as_bytes());
    cause.death.append_scope_id(&mut key);
    key.extend_from_slice(&cause.death.scope_ownership_generation.get().to_be_bytes());
    key.extend_from_slice(&cause.death.actor_local_id.to_be_bytes());
    key.extend_from_slice(&cause.death.actor_local_generation.to_be_bytes());
    for value in [
        cause.loot_table.family.as_bytes(),
        cause.loot_table.production_key.as_bytes(),
        cause.loot_table.revision_ref.as_bytes(),
        cause.purpose_key.as_bytes(),
    ] {
        key.extend_from_slice(&u16::try_from(value.len()).unwrap_or(u16::MAX).to_be_bytes());
        key.extend_from_slice(value);
    }
    key.extend_from_slice(&cause.draw_ordinal.to_be_bytes());
    let digest: [u8; 32] = Sha256::digest(&key).into();
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:item-mint:' || encode($1, 'hex'), 0))",
    )
    .bind(digest.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cause: &ItemMintCause,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(bind_cause!(
        sqlx::query(concat!(
            "SELECT intent_binding, transaction_id::text, event_id::text, \
                    item_instance_id::text, occurred_at, envelope_sha256, \
                    corpse_top_damage_character_id::text \
               FROM game_item_mint_receipts WHERE ",
            cause_key!()
        )),
        cause
    )
    .fetch_optional(&mut **tx)
    .await?)
}

async fn load_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cause: &ItemMintCause,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(bind_cause!(
        sqlx::query(concat!(
            "SELECT intent_binding, transaction_id::text, event_id::text, \
                    item_instance_id::text, occurred_at, envelope, \
                    fence_holder_node_id::text, \
                    fence_holder_registration_revision::text, work_units_used \
               FROM game_item_mint_reservations WHERE ",
            cause_key!()
        )),
        cause
    )
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_reservation(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Reservation, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    Ok(Reservation {
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        envelope: row.try_get("envelope")?,
        fence_node_id: uuid_text(row.try_get("fence_holder_node_id")?)?,
        fence_registration_revision: row
            .try_get::<String, _>("fence_holder_registration_revision")?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        work_units_used: u8::try_from(row.try_get::<i16, _>("work_units_used")?)
            .map_err(invalid)?,
    })
}

async fn insert_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &ItemMintRequest,
    intent_binding: &[u8; 33],
    reservation: &Reservation,
    tuple: SelectedType2Tuple,
) -> std::result::Result<(), DurabilityError> {
    // The fence generation ($3 again) is the death's own generation (D52).
    bind_cause!(
        sqlx::query(
            "INSERT INTO game_item_mint_reservations(death_world_id, death_channel_id, \
               death_scope_ownership_generation, death_actor_local_id, \
               death_actor_local_generation, loot_table_family, loot_table_production_key, \
               loot_table_revision_ref, loot_purpose_key, draw_ordinal, intent_binding, \
               transaction_id, event_id, item_instance_id, occurred_at, envelope, \
               fence_scope_ownership_generation, fence_holder_node_id, \
               fence_holder_registration_revision, work_units_used, reserved_at, \
               type2_schema_revision, death_instance_id, death_scope_kind, death_scope_key) \
             VALUES (encode($1,'hex')::uuid, CASE WHEN get_byte($2,0)=1 THEN encode(substring($2 from 2),'hex')::uuid END, $3::text::numeric(20,0), \
               $4, $5::text::numeric(20,0), $6, $7, $8, $9, $10, $11, \
               encode($12,'hex')::uuid, encode($13,'hex')::uuid, encode($14,'hex')::uuid, \
               $15, $16, $3::text::numeric(20,0), encode($17,'hex')::uuid, \
               $18::text::numeric(20,0), 0, \
               floor(extract(epoch FROM statement_timestamp())*1000)::bigint, $19, \
               CASE WHEN get_byte($2,0)=2 THEN encode(substring($2 from 2),'hex')::uuid END, $20, $21)",
        ),
        &request.cause
    )
    .bind(intent_binding.as_slice())
    .bind(reservation.transaction_id.as_slice())
    .bind(reservation.event_id.as_slice())
    .bind(reservation.item_instance_id.as_slice())
    .bind(reservation.occurred_at_unix_ms)
    .bind(reservation.envelope.as_slice())
    .bind(reservation.fence_node_id.as_slice())
    .bind(reservation.fence_registration_revision.to_string())
    .bind(i16::try_from(tuple.schema_revision()).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(request.cause.death.scope_kind())
    .bind(assignment_scope_key(request.cause.death.runtime_scope).as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The reservation's fence names exactly this node incarnation.
fn fence_matches(reservation: &Reservation, node: &NodeIncarnationProof) -> bool {
    let fact = node.fact();
    reservation.fence_node_id == *fact.node_id().as_bytes()
        && reservation.fence_registration_revision == fact.registration_revision()
}

/// DUR-03 §32 / D52: the death's ownership generation is the current
/// assignment of its scope, held by `node`, whose incarnation is current.
async fn fence_is_live(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    death: &CreatureDeathKey,
    node: &NodeIncarnationProof,
) -> std::result::Result<bool, DurabilityError> {
    let fact = node.fact();
    let assignment = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments a \
         WHERE scope_key = $1 AND world_id = encode($2,'hex')::uuid \
           AND channel_id IS NOT DISTINCT FROM encode($3,'hex')::uuid \
           AND activity_instance_id IS NOT DISTINCT FROM encode($7,'hex')::uuid AND state = 1 \
           AND ownership_generation = $4::text::numeric(20,0) \
           AND holder_node_id = encode($5,'hex')::uuid \
           AND holder_registration_revision = $6::text::numeric(20,0) \
           AND (a.scope_kind=1 OR (a.scope_kind=3 AND EXISTS ( \
               SELECT 1 FROM game_activity_handoff_groups g \
               JOIN game_activity_runtime_entries e USING(handoff_id) \
               JOIN game_activity_handoff_participants p USING(handoff_id) \
               WHERE g.world_id=a.world_id AND g.instance_id=a.activity_instance_id \
                 AND g.ownership_generation=a.ownership_generation AND g.state=2 \
                 AND e.holder_node_id=a.holder_node_id \
                 AND e.holder_registration_revision=a.holder_registration_revision \
                 AND NOT EXISTS (SELECT 1 FROM game_activity_runtime_returns r \
                     WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id)))) \
         FOR SHARE",
    )
    .bind(assignment_scope_key(death.runtime_scope).as_slice())
    .bind(death.world_id.as_bytes().as_slice())
    .bind(death.channel_bytes())
    .bind(death.scope_ownership_generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .bind(death.instance_bytes())
    .fetch_optional(&mut **tx)
    .await?;
    Ok(assignment.is_some() && prove_current_incarnation(tx, node).await?)
}

/// D3 §4.2: take the per-scope `oteryn:corpse-cap:` advisory transaction lock
/// (serializing every concurrent corpse-MINT commit for this exact scope
/// against each other -- and only them, so no other MINT/TRANSFER/XP writer
/// of the scope is affected) and recount live corpses currently on Ground for
/// it. The count is authoritative only inside the same transaction as the
/// corpse's own insert; an earlier `freeze_item_mint`-time count is advisory
/// only (§39.4).
async fn corpse_cap_recount(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    death: &CreatureDeathKey,
) -> std::result::Result<i64, DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:corpse-cap:' || encode($1,'hex'), 0))",
    )
    .bind(assignment_scope_key(death.runtime_scope).as_slice())
    .execute(&mut **tx)
    .await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_ground_locations g \
           JOIN game_item_mint_receipts r ON r.item_instance_id = g.item_instance_id \
           JOIN game_item_instances i ON i.item_instance_id = g.item_instance_id \
          WHERE g.world_id = encode($1,'hex')::uuid AND g.channel_id IS NOT DISTINCT FROM encode($2,'hex')::uuid \
            AND g.instance_id IS NOT DISTINCT FROM encode($4,'hex')::uuid \
            AND r.loot_purpose_key = $3 AND i.lifecycle = 1",
    )
    .bind(death.world_id.as_bytes().as_slice())
    .bind(death.channel_bytes())
    .bind(CORPSE_MATERIALIZATION_PURPOSE_KEY)
    .bind(death.instance_bytes())
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

async fn insert_mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    frozen: &FrozenMint,
    tuple: SelectedType2Tuple,
) -> std::result::Result<(), DurabilityError> {
    insert_mint_with_corpse_attribution(tx, permit, frozen, tuple, None).await
}

/// D3-1: the corpse's own MINT, identical to [`insert_mint`] except its
/// receipt also carries the captured top-damage `CharacterId` (D132/§4.3).
/// `materialized_at` is left NULL here; only the deferred
/// `game_item_mint_receipt_materialize_corpse` trigger (migration 0013) ever
/// writes it.
async fn insert_corpse_mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    frozen: &FrozenMint,
    tuple: SelectedType2Tuple,
    top_damage_character_id: [u8; 16],
) -> std::result::Result<(), DurabilityError> {
    insert_mint_with_corpse_attribution(tx, permit, frozen, tuple, Some(top_damage_character_id))
        .await
}

async fn insert_mint_with_corpse_attribution(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    frozen: &FrozenMint,
    tuple: SelectedType2Tuple,
    top_damage_character_id: Option<[u8; 16]>,
) -> std::result::Result<(), DurabilityError> {
    let request = &frozen.request;
    let death = &request.cause.death;
    // SPELL-LOCK-2 §1.2: the Ground INSERT fires the key-33 owner lock of the death's exact scope.
    permit.check_scope(death.runtime_scope)?;
    insert_item_instance(tx, frozen).await?;
    sqlx::query(
        "INSERT INTO game_item_ground_locations(item_instance_id, world_id, channel_id, \
           runtime_scope_ownership_generation, spatial_position, corpse_ref, map_revision, \
           content_revision, native_room_placement_context, scope_kind, instance_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4::text::numeric(20,0), $5, $6, $7, $8, $9, $10, encode($11,'hex')::uuid)",
    )
    .bind(frozen.item_instance_id.as_slice())
    .bind(death.world_id.as_bytes().as_slice())
    .bind(death.channel_bytes())
    .bind(death.scope_ownership_generation.get().to_string())
    .bind(request.ground.spatial_position.as_slice())
    .bind(request.ground.corpse_ref.as_slice())
    .bind(&request.ground.map_revision)
    .bind(&request.ground.content_revision)
    .bind(request.ground.native_room_placement_context.as_slice())
    .bind(ground_scope_columns(death.runtime_scope).0)
    .bind(death.instance_bytes())
    .execute(&mut **tx)
    .await?;
    insert_receipt(tx, frozen, top_damage_character_id, None).await?;
    insert_audit_outbox(tx, frozen, tuple).await
}

/// D3-2: a loot entry's MINT, identical to [`insert_mint`] except that the
/// fresh item is established as `Container(parent = corpse)` entry
/// `placement.placement_ordinal` instead of on Ground, and its receipt names
/// that parent and ordinal (migration 0013).
async fn insert_corpse_loot_mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    frozen: &FrozenMint,
    tuple: SelectedType2Tuple,
    placement: &CorpseContainerPlacement,
) -> std::result::Result<(), DurabilityError> {
    let death = &frozen.request.cause.death;
    // SPELL-LOCK-2 §1.2: the entry INSERT fires the key-33 owner lock of the corpse's Ground
    // root, so the lane must be that root's exact scope.
    let root = sqlx::query(
        "SELECT world_id::text, channel_id::text, scope_kind, instance_id::text FROM game_item_ground_locations \
          WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(placement.corpse_item_instance_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(root) = root else {
        return Err(DurabilityError::InvalidStoredState);
    };
    permit.check_scope(decode_ground_scope(&root)?)?;
    insert_item_instance(tx, frozen).await?;
    sqlx::query(
        "INSERT INTO game_item_corpse_container_entries(item_instance_id, world_id, \
           parent_item_instance_id, placement_ordinal, placed_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4::text::numeric(20,0), encode($5,'hex')::uuid)",
    )
    .bind(frozen.item_instance_id.as_slice())
    .bind(death.world_id().as_bytes().to_vec())
    .bind(placement.corpse_item_instance_id.as_slice())
    .bind(placement.placement_ordinal.to_string())
    .bind(frozen.transaction_id.as_slice())
    .execute(&mut **tx)
    .await?;
    insert_receipt(tx, frozen, None, Some(placement)).await?;
    insert_loot_fluid(tx, frozen, placement).await?;
    insert_audit_outbox(tx, frozen, tuple).await
}

async fn insert_nested_loot_mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
    tuple: SelectedType2Tuple,
    placement: &CorpseContainerPlacement,
) -> std::result::Result<(), DurabilityError> {
    insert_item_instance(tx, frozen).await?;
    sqlx::query(
        "INSERT INTO game_item_nested_entries(item_instance_id, world_id, \
           parent_item_instance_id, placement_ordinal, placed_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4::text::numeric(20,0), encode($5,'hex')::uuid)",
    )
    .bind(frozen.item_instance_id.as_slice())
    .bind(frozen.request.cause.death.world_id().as_bytes().to_vec())
    .bind(placement.corpse_item_instance_id.as_slice())
    .bind(placement.placement_ordinal.to_string())
    .bind(frozen.transaction_id.as_slice())
    .execute(&mut **tx)
    .await?;
    insert_receipt(tx, frozen, None, Some(placement)).await?;
    insert_loot_fluid(tx, frozen, placement).await?;
    insert_audit_outbox(tx, frozen, tuple).await
}

/// W22a: a loot entry with a fluid subtype carries it as the item's fluid state, proven by its
/// own loot-mint row in the same transaction (migration 0597).
async fn insert_loot_fluid(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
    placement: &CorpseContainerPlacement,
) -> std::result::Result<(), DurabilityError> {
    let Some(fluid) = placement.fluid_subtype.filter(|fluid| *fluid != 0) else {
        return Ok(());
    };
    sqlx::query(
        "INSERT INTO game_item_fluid_loot_mints(item_instance_id, transaction_id, fluid_subtype) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)",
    )
    .bind(frozen.item_instance_id.as_slice())
    .bind(frozen.transaction_id.as_slice())
    .bind(i16::from(fluid))
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_fluid_states(item_instance_id, world_id, fluid_subtype, \
           last_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, encode($4,'hex')::uuid)",
    )
    .bind(frozen.item_instance_id.as_slice())
    .bind(frozen.request.cause.death.world_id().as_bytes().to_vec())
    .bind(i16::from(fluid))
    .bind(frozen.transaction_id.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_item_instance(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
) -> std::result::Result<(), DurabilityError> {
    let request = &frozen.request;
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
           encode($7,'hex')::uuid)",
    )
    .bind(frozen.item_instance_id.as_slice())
    .bind(request.cause.death.world_id().as_bytes().to_vec())
    .bind(&request.item.family)
    .bind(&request.item.production_key)
    .bind(&request.item.revision_ref)
    .bind(i64::from(request.quantity))
    .bind(frozen.transaction_id.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
    top_damage_character_id: Option<[u8; 16]>,
    corpse_entry: Option<&CorpseContainerPlacement>,
) -> std::result::Result<(), DurabilityError> {
    let request = &frozen.request;
    let death = &request.cause.death;
    sqlx::query(
        "INSERT INTO game_item_mint_receipts(death_world_id, death_channel_id, \
           death_scope_ownership_generation, death_actor_local_id, \
           death_actor_local_generation, loot_table_family, loot_table_production_key, \
           loot_table_revision_ref, loot_purpose_key, draw_ordinal, intent_binding, \
           transaction_id, event_id, item_instance_id, occurred_at, envelope_sha256, \
           committed_at, corpse_top_damage_character_id, \
           destination_parent_item_instance_id, destination_ordinal, death_instance_id, \
           death_scope_kind, death_scope_key) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), $4, \
           $5::text::numeric(20,0), $6, $7, $8, $9, $10, $11, encode($12,'hex')::uuid, \
           encode($13,'hex')::uuid, encode($14,'hex')::uuid, $15, $16, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint, \
           encode($17,'hex')::uuid, encode($18,'hex')::uuid, $19::text::numeric(20,0), encode($20,'hex')::uuid, $21, $22)",
    )
    .bind(death.world_id.as_bytes().as_slice())
    .bind(death.channel_bytes())
    .bind(death.scope_ownership_generation.get().to_string())
    .bind(i64::from(death.actor_local_id))
    .bind(death.actor_local_generation.to_string())
    .bind(&request.cause.loot_table.family)
    .bind(&request.cause.loot_table.production_key)
    .bind(&request.cause.loot_table.revision_ref)
    .bind(&request.cause.purpose_key)
    .bind(i64::from(request.cause.draw_ordinal))
    .bind(frozen.intent_binding.as_slice())
    .bind(frozen.transaction_id.as_slice())
    .bind(frozen.event_id.as_slice())
    .bind(frozen.item_instance_id.as_slice())
    .bind(frozen.occurred_at_unix_ms)
    .bind(frozen.envelope_sha256.as_slice())
    .bind(top_damage_character_id.map(|value| value.to_vec()))
    .bind(corpse_entry.map(|entry| entry.corpse_item_instance_id.to_vec()))
    .bind(corpse_entry.map(|entry| entry.placement_ordinal.to_string()))
    .bind(death.instance_bytes())
    .bind(death.scope_kind())
    .bind(assignment_scope_key(death.runtime_scope).as_slice())
    .execute(&mut **tx)
    .await?;
    if let Some(race) = &request.cause.corpse_race {
        sqlx::query(
            "INSERT INTO game_corpse_source_races(item_instance_id,mint_transaction_id,race_key,race_definition_revision) \
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4)",
        )
        .bind(frozen.item_instance_id.as_slice())
        .bind(frozen.transaction_id.as_slice())
        .bind(race.key())
        .bind(race.definition_revision())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn insert_audit_outbox(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
    tuple: SelectedType2Tuple,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(frozen.event_id.as_slice())
    .bind(frozen.transaction_id.as_slice())
    .bind(EVENT_TYPE_ID)
    .bind(i64::from(tuple.schema_revision()))
    .bind(tuple.retention_profile_id())
    .bind(frozen.item_instance_id.as_slice())
    .bind(frozen.occurred_at_unix_ms)
    .bind(audit::AUDIT_RETENTION_P90D_MS)
    .bind(frozen.envelope.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The frozen identities already belong to a committed MINT, audit event or
/// item: a different logical transaction must never reuse them.
async fn mint_identity_reused(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
) -> std::result::Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_item_mint_receipts \
                         WHERE transaction_id = encode($1,'hex')::uuid \
                            OR event_id = encode($2,'hex')::uuid \
                            OR item_instance_id = encode($3,'hex')::uuid) \
             OR EXISTS (SELECT 1 FROM game_item_audit_outbox \
                         WHERE transaction_id = encode($1,'hex')::uuid \
                            OR event_id = encode($2,'hex')::uuid) \
             OR EXISTS (SELECT 1 FROM game_item_instances \
                         WHERE item_instance_id = encode($3,'hex')::uuid)",
    )
    .bind(frozen.transaction_id.as_slice())
    .bind(frozen.event_id.as_slice())
    .bind(frozen.item_instance_id.as_slice())
    .fetch_one(&mut **tx)
    .await?)
}

/// D3 §4.1: the parent of a corpse-loot MINT is the live corpse
/// `ItemInstance` (still on Ground) that this exact death's own
/// `CORPSE_MATERIALIZATION` MINT committed. The migration 0013 guard enforces
/// the same at commit; this is the early, typed refusal.
async fn corpse_parent_is_live(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    death: &CreatureDeathKey,
    placement: &CorpseContainerPlacement,
) -> std::result::Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS ( \
           SELECT 1 FROM game_item_mint_receipts cr \
             JOIN game_item_ground_locations cg ON cg.item_instance_id = cr.item_instance_id \
             JOIN game_item_instances ci ON ci.item_instance_id = cr.item_instance_id \
            WHERE cr.item_instance_id = encode($1,'hex')::uuid \
              AND cr.loot_purpose_key = $2 \
              AND cr.death_world_id = encode($3,'hex')::uuid \
              AND cr.death_scope_id = $4 \
              AND cr.death_scope_ownership_generation = $5::text::numeric(20,0) \
              AND cr.death_actor_local_id = $6 \
              AND cr.death_actor_local_generation = $7::text::numeric(20,0) \
              AND ci.lifecycle = 1)",
    )
    .bind(placement.corpse_item_instance_id.as_slice())
    .bind(CORPSE_MATERIALIZATION_PURPOSE_KEY)
    .bind(death.world_id.as_bytes().as_slice())
    .bind(death.scope_id().to_vec())
    .bind(death.scope_ownership_generation.get().to_string())
    .bind(i64::from(death.actor_local_id))
    .bind(death.actor_local_generation.to_string())
    .fetch_one(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedItemMint, DurabilityError> {
    let digest: Vec<u8> = row.try_get("envelope_sha256")?;
    Ok(CommittedItemMint {
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        envelope_sha256: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    })
}

fn decode_instance(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<GroundItemInstance, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    Ok(GroundItemInstance {
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        world_id: WorldId::decode(&uuid_text(row.try_get("world_id")?)?).map_err(invalid)?,
        runtime_scope: decode_ground_scope(row)?,
        definition: TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        },
        quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        runtime_scope_ownership_generation: row
            .try_get::<String, _>("runtime_scope_ownership_generation")?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        ground: GroundPlacement {
            spatial_position: row.try_get("spatial_position")?,
            corpse_ref: row.try_get("corpse_ref")?,
            map_revision: row.try_get("map_revision")?,
            content_revision: row.try_get("content_revision")?,
            native_room_placement_context: row.try_get("native_room_placement_context")?,
        },
        minted_transaction_id: uuid_text(row.try_get("minted_transaction_id")?)?,
    })
}

/// Existing Ground custody's exact namespace. Channel columns and key-33 bytes
/// retain their original representation; an Instance never borrows its origin.
pub(crate) fn ground_scope_columns(
    scope: crate::foundation::RuntimeScopeRefV1,
) -> (i16, Option<[u8; 16]>, Option<[u8; 16]>) {
    match scope {
        crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } => {
            (1, Some(*channel_id.as_bytes()), None)
        }
        crate::foundation::RuntimeScopeRefV1::Instance { instance_id, .. } => {
            (2, None, Some(instance_id))
        }
    }
}

pub(crate) fn decode_ground_scope(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<crate::foundation::RuntimeScopeRefV1, DurabilityError> {
    use crate::foundation::RuntimeScopeRefV1;
    let invalid = || DurabilityError::InvalidStoredState;
    let world = WorldId::decode(&uuid_text(row.try_get("world_id")?)?).map_err(|_| invalid())?;
    let channel: Option<String> = row.try_get("channel_id")?;
    let instance: Option<String> = row.try_get("instance_id")?;
    match (row.try_get::<i16, _>("scope_kind")?, channel, instance) {
        (1, Some(channel), None) => Ok(RuntimeScopeRefV1::channel(
            world,
            ChannelId::decode(&uuid_text(&channel)?).map_err(|_| invalid())?,
        )),
        (2, None, Some(instance)) => {
            let instance_id = uuid_text(&instance)?;
            super::item_mint_audit::check_uuid_v7(&instance_id).map_err(|_| invalid())?;
            Ok(RuntimeScopeRefV1::Instance {
                world_id: world,
                instance_id,
            })
        }
        _ => Err(invalid()),
    }
}

pub(super) fn uuid_text(value: &str) -> std::result::Result<[u8; 16], DurabilityError> {
    let hex: String = value
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if hex.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut out = [0_u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn definition(family: &str, key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: family.into(),
            production_key: key.into(),
            revision_ref: "rev-1".into(),
        }
    }

    fn request() -> ItemMintRequest {
        ItemMintRequest {
            cause: ItemMintCause::for_test(
                WorldId::decode(&id(1)).expect("world"),
                ChannelId::decode(&id(2)).expect("channel"),
                ScopeOwnershipGeneration::new(1).expect("generation"),
                7,
                1,
                definition("LootTable", "fixture:loot"),
                "fixture:purpose.drop".into(),
                1,
            ),
            item: definition("ItemType", "fixture:alpha"),
            quantity: 1,
            ground: GroundPlacement {
                spatial_position: vec![1, 2, 3],
                corpse_ref: id(3).to_vec(),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: id(6).to_vec(),
            },
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        }
    }

    #[test]
    fn srx2_scavenge_corpse_source_is_typed_and_bound_to_mint_intent() {
        let race = crate::domain::bestiary::BestiaryRace::new(
            "oteryn:creature.dragon",
            "dragon-r1",
            vec![5, 50, 500],
        )
        .expect("race");
        assert!(request().cause.with_corpse_race(race.clone()).is_err());
        let mut corpse = request();
        corpse.cause.purpose_key = CORPSE_MATERIALIZATION_PURPOSE_KEY.into();
        corpse.cause.draw_ordinal = 0;
        let legacy = intent_binding(&corpse, None).expect("legacy");
        corpse.cause = corpse.cause.with_corpse_race(race).expect("typed corpse");
        assert_ne!(legacy, intent_binding(&corpse, None).expect("bound race"));
        let mut other = corpse.clone();
        other.cause = other
            .cause
            .with_corpse_race(
                crate::domain::bestiary::BestiaryRace::new(
                    "oteryn:creature.dragon_lord",
                    "dragon-r1",
                    vec![5, 50, 500],
                )
                .expect("other race"),
            )
            .expect("typed other corpse");
        assert_ne!(
            intent_binding(&corpse, None).expect("first"),
            intent_binding(&other, None).expect("second")
        );
    }

    #[test]
    fn intent_binding_covers_the_whole_intent_and_every_cause_component() {
        let base = intent_binding(&request(), None).expect("binding");
        assert_eq!(base[0], INTENT_BINDING_VERSION);
        assert_eq!(intent_binding(&request(), None).expect("binding"), base);
        let mutations: [fn(&mut ItemMintRequest); 8] = [
            |r| r.cause.death.actor_local_id = 8,
            |r| r.cause.death.actor_local_generation = 2,
            |r| r.cause.loot_table.revision_ref = "rev-2".into(),
            |r| r.cause.purpose_key = "fixture:purpose.other".into(),
            |r| r.cause.draw_ordinal = 2,
            |r| r.item.production_key = "fixture:beta".into(),
            |r| r.quantity = 2,
            |r| r.ground.spatial_position = vec![1, 2, 4],
        ];
        for mutate in mutations {
            let mut changed = request();
            mutate(&mut changed);
            assert_ne!(intent_binding(&changed, None).expect("binding"), base);
        }
    }

    #[test]
    fn invalid_input_fails_before_database_work() {
        let mut zero = request();
        zero.quantity = 0;
        assert!(matches!(
            validate_request(&zero, None),
            Err(ItemMintError::InvalidInput)
        ));
        let mut generation = request();
        generation.cause.death.actor_local_generation = 0;
        assert!(matches!(
            validate_request(&generation, None),
            Err(ItemMintError::InvalidInput)
        ));
        let mut long = request();
        long.cause.purpose_key = "p".repeat(513);
        assert!(matches!(
            validate_request(&long, None),
            Err(ItemMintError::InvalidInput)
        ));
        let mut max = request();
        max.cause.purpose_key = "p".repeat(512);
        max.ground.corpse_ref = vec![3; 128];
        assert!(validate_request(&max, None).is_ok());
        let mut technical = request();
        technical.ground.corpse_ref = vec![3; 129];
        assert!(matches!(
            validate_request(&technical, None),
            Err(ItemMintError::InvalidInput)
        ));
    }

    #[test]
    fn creature_mint_scope_identity_never_aliases_channel_and_instance() {
        let channel = request();
        let mut instance = request();
        instance.cause.death.runtime_scope = RuntimeScopeRefV1::instance(
            channel.cause.death.world_id,
            *channel
                .cause
                .death
                .runtime_scope
                .channel_id()
                .expect("valid scoped fixture")
                .as_bytes(),
        )
        .expect("valid scoped fixture");
        assert_ne!(
            channel.cause.death.scope_id(),
            instance.cause.death.scope_id()
        );
        assert_ne!(
            intent_binding(&channel, None).expect("valid scoped fixture"),
            intent_binding(&instance, None).expect("valid scoped fixture")
        );
        let channel_message = mint_message(&channel, None, id(9));
        let instance_message = mint_message(&instance, None, id(9));
        assert_ne!(channel_message, instance_message);
    }

    #[test]
    fn instance_corpse_audit_decodes_the_tagged_death_scope() -> std::result::Result<(), AuditError>
    {
        use prost::Message;
        let mut message = mint_message(&request(), None, id(9));
        let source = message.source.as_mut().ok_or(AuditError::InvalidInput)?;
        let death = source
            .death_occurrence
            .as_mut()
            .ok_or(AuditError::InvalidInput)?;
        death.channel_id.clear();
        let mut bytes = death.encode_to_vec();
        bytes.extend_from_slice(&[50, 16]); // additive instance_id field 6
        bytes.extend_from_slice(&id(4));
        *death = CreatureDeathOccurrenceRefV1::decode(bytes.as_slice())
            .map_err(|_| AuditError::InvalidInput)?;
        let ground = message
            .destination
            .as_mut()
            .ok_or(AuditError::InvalidInput)?;
        ground.channel_id.clear();
        ground.instance_id = Some(id(4).to_vec());
        audit::check_mint(&message)
    }

    #[test]
    fn frozen_mint_message_is_admissible_and_in_the_death_scope() {
        let message = mint_message(&request(), None, id(9));
        audit::check_mint(&message).expect("admissible MINT");
        let ground = message.destination.expect("ground");
        assert_eq!(ground.world_id, id(1).to_vec());
        assert_eq!(ground.channel_id, id(2).to_vec());
        assert_eq!(ground.runtime_scope_ownership_generation, 1);
    }

    #[test]
    fn physical_instance_death_reaches_the_mint_cause_without_channel_aliasing() {
        use crate::foundation::RuntimeScopeRefV1;
        use oteryn_gameplay_entities::carrier::CombatDeathFixture;

        let world = WorldId::decode(&id(1)).expect("world");
        let instance_scope = RuntimeScopeRefV1::instance(world, id(2)).expect("Instance");
        let channel_scope = RuntimeScopeRefV1::channel(
            world,
            ChannelId::decode(&id(2)).expect("same UUID in the Channel namespace"),
        );
        let death = |scope| {
            let mut fixture = CombatDeathFixture::new_for_scope(
                scope,
                ScopeOwnershipGeneration::new(1).expect("generation"),
            )
            .expect("physical carrier");
            fixture
                .strike("fixture:scoped-corpse.strike", CombatDeathFixture::HEALTH)
                .expect("lethal strike");
            fixture.project_death().expect("physical death").0
        };
        let instance_death = death(instance_scope);
        let channel_death = death(channel_scope);
        assert!(instance_death.channel_id().is_none());
        assert_eq!(
            instance_death.actor_local_id(),
            channel_death.actor_local_id()
        );
        assert_eq!(
            instance_death.actor_local_generation(),
            channel_death.actor_local_generation()
        );
        let from_death = |death| {
            let mut value = request();
            value.cause = ItemMintCause::from_creature_death(
                death,
                definition("LootTable", "fixture:loot"),
                "fixture:purpose.drop".into(),
                1,
            )
            .expect("physical Instance and Channel death causes are supported");
            value
        };
        let instance = from_death(instance_death);
        let channel = from_death(channel_death);
        assert_ne!(
            intent_binding(&instance, None).expect("Instance binding"),
            intent_binding(&channel, None).expect("Channel binding")
        );
        let message = mint_message(&instance, None, id(9));
        audit::check_mint(&message).expect("scoped MINT");
        assert_eq!(
            message
                .destination
                .expect("Ground")
                .runtime_scope()
                .expect("scope"),
            instance_scope
        );
        let source = message
            .source
            .expect("source")
            .death_occurrence
            .expect("death");
        assert_eq!(source.instance_id, Some(id(2).to_vec()));
        assert!(source.channel_id.is_empty());
    }

    #[test]
    fn instance_corpse_mint_scope_is_tagged_and_round_trips_without_a_channel() {
        let channel = request();
        let mut instance = channel.clone();
        instance.cause.death.runtime_scope =
            RuntimeScopeRefV1::instance(instance.cause.death.world_id, id(2))
                .expect("typed Instance");
        assert_ne!(
            intent_binding(&channel, None).expect("Channel intent"),
            intent_binding(&instance, None).expect("Instance intent"),
            "equal UUIDs and local death fields never alias the two namespaces"
        );
        let mint = mint_message(&instance, None, id(9));
        let wire = audit::encode_mint_event(
            MintEventIdentity {
                event_id: id(7),
                transaction_id: id(8),
                occurred_at_unix_ms: 1,
                server_build_id: SERVER_BUILD_ID,
            },
            mint.clone(),
            audit::Type2EventTuple::V1,
        )
        .expect("Instance audit");
        let (envelope, decoded) = audit::decode_envelope(&wire).expect("strict audit round trip");
        assert_eq!(decoded, mint);
        assert_eq!(envelope.channel_id, None);
        assert_eq!(envelope.instance_id, Some(id(2).to_vec()));
        let mut wrong_ground = mint.clone();
        wrong_ground
            .destination
            .as_mut()
            .expect("Ground")
            .instance_id = Some(id(3).to_vec());
        assert_eq!(
            audit::check_mint(&wrong_ground),
            Err(AuditError::InvalidInput)
        );
        let mut borrowed_ground = mint.clone();
        let ground = borrowed_ground.destination.as_mut().expect("Ground");
        ground.instance_id = None;
        ground.channel_id = id(2).to_vec();
        assert_eq!(
            audit::check_mint(&borrowed_ground),
            Err(AuditError::InvalidInput),
            "equal UUIDs do not turn Instance death provenance into Channel Ground"
        );
        let mut mixed = mint.clone();
        mixed
            .source
            .as_mut()
            .expect("source")
            .death_occurrence
            .as_mut()
            .expect("death")
            .channel_id = id(2).to_vec();
        assert_eq!(audit::check_mint(&mixed), Err(AuditError::InvalidInput));
        let mut empty = mint.clone();
        empty
            .source
            .as_mut()
            .expect("source")
            .death_occurrence
            .as_mut()
            .expect("death")
            .instance_id = None;
        assert_eq!(audit::check_mint(&empty), Err(AuditError::InvalidInput));
        for malformed in [Vec::new(), vec![0; 16], id(2)[..15].to_vec()] {
            let mut malformed_scope = mint.clone();
            malformed_scope
                .source
                .as_mut()
                .expect("source")
                .death_occurrence
                .as_mut()
                .expect("death")
                .instance_id = Some(malformed);
            assert_eq!(
                audit::check_mint(&malformed_scope),
                Err(AuditError::InvalidInput)
            );
        }
    }

    #[test]
    fn instance_corpse_audit_rejects_duplicate_and_unknown_death_scope_fields() {
        use prost::Message;
        fn message_field(out: &mut Vec<u8>, tag: u64, value: &[u8]) {
            prost::encoding::encode_varint((tag << 3) | 2, out);
            prost::encoding::encode_varint(value.len() as u64, out);
            out.extend_from_slice(value);
        }
        fn payload(mint: &OneItemMintV1, death: &[u8]) -> Vec<u8> {
            let mut source = mint.source.clone().expect("source");
            let mut source_bytes = Vec::new();
            message_field(&mut source_bytes, 1, source.typed_cause.as_bytes());
            message_field(&mut source_bytes, 2, death);
            source.typed_cause.clear();
            source.death_occurrence = None;
            source_bytes.extend_from_slice(&source.encode_to_vec());
            let mut skeleton = mint.clone();
            skeleton.source = None;
            skeleton.before_semantically_absent = false;
            let mut mint_bytes = skeleton.encode_to_vec();
            message_field(&mut mint_bytes, 3, &source_bytes);
            mint_bytes.extend_from_slice(&[32, 1]);
            let mut wire = vec![8, 1];
            message_field(&mut wire, 2, &mint_bytes);
            wire
        }
        let mut request = request();
        request.cause.death.runtime_scope =
            RuntimeScopeRefV1::instance(request.cause.death.world_id, id(2)).expect("Instance");
        let mint = mint_message(&request, None, id(9));
        let death = mint
            .source
            .as_ref()
            .expect("source")
            .death_occurrence
            .as_ref()
            .expect("death")
            .encode_to_vec();
        let canonical = payload(&mint, &death);
        assert_eq!(
            canonical,
            audit::OneItemTransactionV1 {
                interpretation_revision: audit::INTERPRETATION_REVISION,
                operation: Some(audit::OneItemOperationV1::Mint(mint.clone())),
            }
            .encode_to_vec()
        );
        assert!(audit::decode_payload(&canonical).is_ok());
        for field in [6, 7] {
            let mut changed = death.clone();
            message_field(&mut changed, field, &id(2));
            assert_eq!(
                audit::decode_payload(&payload(&mint, &changed)),
                Err(AuditError::InvalidInput)
            );
        }
    }

    fn placement(ordinal: u32) -> CorpseContainerPlacement {
        CorpseContainerPlacement {
            corpse_item_instance_id: id(8),
            placement_ordinal: ordinal,
            fluid_subtype: None,
        }
    }

    #[test]
    fn corpse_loot_binding_covers_parent_and_ordinal_and_differs_from_ground() {
        let base = intent_binding(&request(), Some(&placement(1))).expect("binding");
        assert_eq!(
            intent_binding(&request(), Some(&placement(1))).expect("binding"),
            base
        );
        assert_ne!(
            intent_binding(&request(), Some(&placement(2))).expect("binding"),
            base
        );
        let other_parent = CorpseContainerPlacement {
            corpse_item_instance_id: id(9),
            placement_ordinal: 1,
            fluid_subtype: None,
        };
        assert_ne!(
            intent_binding(&request(), Some(&other_parent)).expect("binding"),
            base
        );
        assert_ne!(intent_binding(&request(), None).expect("binding"), base);
    }

    /// W22a: the loot fluid is part of the corpse-entry intent; no fluid keeps the old layout.
    #[test]
    fn corpse_loot_binding_covers_the_fluid_subtype() {
        let base = intent_binding(&request(), Some(&placement(1))).expect("binding");
        let fluid = |subtype| CorpseContainerPlacement {
            fluid_subtype: Some(subtype),
            ..placement(1)
        };
        let five = intent_binding(&request(), Some(&fluid(5))).expect("binding");
        assert_ne!(five, base);
        assert_ne!(
            intent_binding(&request(), Some(&fluid(6))).expect("binding"),
            five
        );
    }

    #[test]
    fn corpse_loot_placement_is_bounded_before_database_work() {
        for ordinal in [1, CORPSE_CONTAINER_ENTRIES_MAX] {
            assert!(validate_request(&request(), Some(&placement(ordinal))).is_ok());
        }
        for ordinal in [0, CORPSE_CONTAINER_ENTRIES_MAX + 1] {
            assert!(matches!(
                validate_request(&request(), Some(&placement(ordinal))),
                Err(ItemMintError::InvalidInput)
            ));
        }
        let mut reserved = request();
        reserved.cause.purpose_key = CORPSE_MATERIALIZATION_PURPOSE_KEY.into();
        assert!(matches!(
            validate_request(&reserved, Some(&placement(1))),
            Err(ItemMintError::InvalidInput)
        ));
        let bad_parent = CorpseContainerPlacement {
            corpse_item_instance_id: [0; 16],
            placement_ordinal: 1,
            fluid_subtype: None,
        };
        assert!(matches!(
            validate_request(&request(), Some(&bad_parent)),
            Err(ItemMintError::InvalidInput)
        ));
    }

    #[test]
    fn corpse_loot_message_has_a_container_destination_and_no_ground() {
        let message = mint_message(&request(), Some(&placement(3)), id(9));
        audit::check_mint(&message).expect("admissible corpse-loot MINT");
        assert!(message.destination.is_none());
        let entry = message.corpse_container_entry.expect("container entry");
        assert_eq!(entry.parent_item_instance_id, id(8).to_vec());
        assert_eq!(entry.placement_ordinal, 3);
    }
    fn native_instance_request() -> ItemMintRequest {
        let mut value = request();
        value.cause.death.runtime_scope =
            RuntimeScopeRefV1::instance(value.cause.death.world_id, id(2)).expect("Instance");
        value.ground.corpse_ref =
            native_instance_corpse_ref(value.cause.death, true).expect("native death");
        value
    }

    #[test]
    fn accepted_instance_frozen_formats_preserve_exact_binding_and_ground_framing() {
        // Independently framed SHA256 preimages from SRVENC9850de690 and ENCXb5103e7f.
        let canonical = [
            1, 114, 37, 219, 121, 134, 200, 34, 102, 77, 34, 82, 111, 161, 248, 191, 44, 210, 247,
            230, 160, 44, 98, 224, 97, 176, 202, 141, 135, 68, 220, 158, 94,
        ];
        let historical = [
            1, 49, 171, 175, 203, 187, 219, 2, 159, 118, 147, 167, 230, 226, 54, 38, 232, 217, 106,
            103, 209, 92, 71, 242, 51, 75, 106, 40, 27, 101, 152, 126, 254,
        ];
        let current = native_instance_request();
        assert_eq!(
            intent_binding(&current, None).expect("canonical"),
            canonical
        );
        let (same, same_binding) =
            replay_native_request(current.clone(), None, &canonical).expect("ENCX frozen replay");
        assert_eq!(same, current);
        assert_eq!(same_binding, canonical);
        let (old, old_binding) = replay_native_request(current.clone(), None, &historical)
            .expect("SRVENC frozen replay");
        assert_eq!(old_binding, historical);
        assert_eq!(old.ground.corpse_ref.len(), 61);
        assert_eq!(
            old.ground.corpse_ref,
            native_instance_corpse_ref(old.cause.death, false).expect("native")
        );
        assert_eq!(
            intent_binding_format(&old, None, false).expect("historical hash"),
            historical
        );
        // Both explicit callers (53 and61) resume the exact stored representation.
        let old_v1 = intent_binding_format(&old, None, false).expect("historical exact v1 caller");
        assert_eq!(
            replay_native_request(old.clone(), None, &old_v1)
                .expect("exact accepted v1")
                .0,
            old
        );
        assert_eq!(
            replay_native_request(old.clone(), None, &canonical)
                .expect("canonical from61")
                .0,
            current
        );
        assert_eq!(
            replay_native_request(old.clone(), None, &historical)
                .expect("historical from61")
                .0,
            old
        );
        assert_eq!(canonical_native_request(old, None), current);
        assert_eq!(current.ground.corpse_ref.len(), 53);
        // A format byte alone cannot authorize compatibility; both formats begin with1.
        let mut forged = historical;
        forged[32] ^= 1;
        assert!(matches!(
            replay_native_request(current, None, &forged),
            Err(ItemMintError::ConflictingCause)
        ));
    }

    #[test]
    fn historical_instance_replay_rejects_business_scope_lane_and_opaque_ref_changes() {
        let source = native_instance_request();
        let mut old = source.clone();
        old.ground.corpse_ref = native_instance_corpse_ref(old.cause.death, false).expect("native");
        let historical = intent_binding_format(&old, None, false).expect("frozen old hash");
        let canonical = intent_binding(&source, None).expect("frozen canonical hash");
        let changes: [fn(&mut ItemMintRequest); 24] = [
            |r| r.cause.death.world_id = WorldId::decode(&id(9)).expect("world"),
            |r| {
                r.cause.death.runtime_scope = RuntimeScopeRefV1::channel(
                    r.cause.death.world_id,
                    ChannelId::decode(&id(2)).expect("same UUID Channel"),
                )
            },
            |r| {
                r.cause.death.runtime_scope =
                    RuntimeScopeRefV1::instance(r.cause.death.world_id, id(9))
                        .expect("different Instance")
            },
            |r| {
                r.cause.corpse_race = Some(
                    crate::domain::bestiary::BestiaryRace::new(
                        "oteryn:creature.dragon",
                        "dragon-r1",
                        vec![5, 50, 500],
                    )
                    .expect("race"),
                )
            },
            |r| {
                r.cause.death.scope_ownership_generation =
                    ScopeOwnershipGeneration::new(2).expect("generation")
            },
            |r| r.cause.death.actor_local_id += 1,
            |r| r.cause.death.actor_local_generation += 1,
            |r| r.cause.loot_table.family.push('x'),
            |r| r.cause.loot_table.production_key.push('x'),
            |r| r.cause.loot_table.revision_ref.push('x'),
            |r| r.cause.purpose_key.push('x'),
            |r| r.cause.draw_ordinal += 1,
            |r| r.item.family.push('x'),
            |r| r.item.production_key.push('x'),
            |r| r.item.revision_ref.push('x'),
            |r| r.quantity += 1,
            |r| r.ground.spatial_position.push(1),
            |r| r.ground.corpse_ref.push(1),
            |r| r.ground.map_revision.push('x'),
            |r| r.ground.content_revision.push('x'),
            |r| r.ground.native_room_placement_context.push(1),
            |r| r.content_revision.push('x'),
            |r| r.ruleset_revision.push('x'),
            |r| r.sim_revision.push('x'),
        ];
        for (index, change) in changes.into_iter().enumerate() {
            let mut changed = source.clone();
            change(&mut changed);
            for binding in [&canonical, &historical] {
                assert!(
                    matches!(
                        replay_native_request(changed.clone(), None, binding),
                        Err(ItemMintError::ConflictingCause)
                    ),
                    "business field {index} is immutable"
                );
            }
        }
        assert!(
            matches!(
                replay_native_request(source.clone(), Some(&placement(1)), &historical),
                Err(ItemMintError::ConflictingCause)
            ),
            "Ground cannot replay as corpse entry"
        );
        for target in [placement(1), placement(2)] {
            let binding = intent_binding_format(&source, Some(&target), false).expect("old entry");
            let (selected, selected_binding) =
                replay_native_request(source.clone(), Some(&target), &binding)
                    .expect("old entry replay");
            assert_eq!(selected, source);
            assert_eq!(selected_binding, binding);
            let mut changed = target;
            changed.placement_ordinal += 1;
            assert!(replay_native_request(source.clone(), Some(&changed), &binding).is_err());
            changed = target;
            changed.corpse_item_instance_id = id(9);
            assert!(replay_native_request(source.clone(), Some(&changed), &binding).is_err());
            changed = target;
            changed.fluid_subtype = Some(1);
            assert!(replay_native_request(source.clone(), Some(&changed), &binding).is_err());
            assert!(replay_native_request(source.clone(), None, &binding).is_err());
        }
    }

    #[test]
    fn channel_historical_binding_and_opaque_placement_remain_byte_identical() {
        let source = request();
        let expected = [
            1, 7, 251, 114, 176, 11, 158, 234, 8, 104, 171, 242, 144, 239, 200, 156, 121, 174, 57,
            151, 188, 106, 35, 30, 25, 252, 183, 98, 234, 82, 11, 150, 226,
        ];
        assert_eq!(
            intent_binding(&source, None).expect("Channel binding"),
            expected
        );
        assert_eq!(canonical_native_request(source.clone(), None), source);
        let (selected, binding) =
            replay_native_request(source.clone(), None, &expected).expect("Channel replay");
        assert_eq!(selected, source);
        assert_eq!(binding, expected);
        assert!(native_instance_corpse_ref(source.cause.death, true).is_none());
        assert!(intent_binding_format(&source, None, true).is_err());
    }
}
